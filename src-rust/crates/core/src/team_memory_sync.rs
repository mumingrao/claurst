use std::collections::HashMap;
use std::path::PathBuf;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};

const MAX_FILE_SIZE_BYTES: usize = 250 * 1024;

const MAX_PUT_BODY_BYTES: usize = 200 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SyncState {
    pub last_known_etag: Option<String>,
    pub server_checksums: HashMap<String, String>,
    pub server_max_entries: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMemoryEntry {
    pub key: String,
    pub content: String,
    pub checksum: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TeamMemoryData {
    pub entries: Vec<TeamMemoryEntry>,
    pub etag: Option<String>,
}

pub fn content_checksum(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("sha256:{}", hex::encode(hasher.finalize()))
}

pub fn validate_memory_path(path: &str) -> Result<()> {
    if path.contains('\0') {
        anyhow::bail!("Path contains null bytes: {:?}", path);
    }
    let lower = path.to_ascii_lowercase();
    if lower.contains("%2e") || lower.contains("%2f") {
        anyhow::bail!("Path contains URL-encoded traversal sequences: {:?}", path);
    }
    if path.contains('\\') {
        anyhow::bail!("Path contains backslashes: {:?}", path);
    }
    if path.starts_with('/') {
        anyhow::bail!("Absolute Unix paths not allowed: {:?}", path);
    }
    // Windows-style absolute path: e.g. "C:" or "c:"
    if path.len() >= 2 {
        let mut chars = path.chars();
        let first = chars.next().unwrap();
        if first.is_ascii_alphabetic() && chars.next() == Some(':') {
            anyhow::bail!("Absolute Windows paths not allowed: {:?}", path);
        }
    }
    if path.split('/').any(|component| component == "..") {
        anyhow::bail!("Path traversal not allowed: {:?}", path);
    }
    Ok(())
}

pub struct TeamMemorySync {
    api_base: String,
    repo: String,
    token: String,
    team_dir: PathBuf,
}

impl TeamMemorySync {
    pub fn new(api_base: String, repo: String, token: String, team_dir: PathBuf) -> Self {
        Self {
            api_base,
            repo,
            token,
            team_dir
        }
    }

    pub async fn pull(&self, state: &mut SyncState) -> Result<()> {
        let client = reqwest::Client::new();
        let url = format!(
            "{}/api/claude_code/team_memory?repo={}",
            self.api_base,
            urlencoding::encode(&self.repo),
        );

        let response = client
            .get(&url)
            .bearer_auth(&self.token)
            .send()
            .await
            .context("team memory pull: HTTP request failed")?;

        let http_status = response.status();

        if http_status.as_u16() == 404 {
            return Ok(());
        }

        if !http_status.is_success() {
            anyhow::bail!("team memory pull failed with status {}", http_status);
        }

        if let Some(etag) = response
            .headers()
            .get("etag")
            .and_then(|v| v.to_str().ok())
        {
            state.last_known_etag = Some(etag.to_string());
        }

        let data: TeamMemoryData = response
            .json()
            .await
            .context("team memory pull: failed to parse response JSON")?;

        state.server_checksums.clear();

        for entry in &data.entries {
            validate_memory_path(&entry.key)
                .with_context(|| format!("server returned unsafe path: {:?}", entry.key))?;

            state
                .server_checksums
                .insert(entry.key.clone(), entry.checksum.clone());

            let local_path = self.team_dir.join(&entry.key);
            if let Some(parent) = local_path.parent() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .with_context(|| format!("create_dir_all for {:?}", parent))?;
            }

            if entry.content.len() <= MAX_FILE_SIZE_BYTES {
                tokio::fs::write(&local_path, &entry.content)
                    .await
                    .with_context(|| format!("writing {:?}", local_path))?;
            }
        }

        Ok(())
    }

    pub async fn push(&self, state: &mut SyncState) -> Result<()> {
        let local_entries = self
            .scan_local_files()
            .await
            .context("team memory push: scanning local files")?;

        let changed: Vec<TeamMemoryEntry> = local_entries
            .into_iter()
            .filter(|entry| {
                state
                    .server_checksums
                    .get(&entry.key)
                    .map(|s| s.as_str())
                    != Some(&entry.checksum)
            })
            .collect();

        if changed.is_empty() {
            return Ok(());
        }

        let batches = self.pack_batches(changed)
        for batch in batches {
            self.update_batch(batch, state)
                .await
                .context("team memory push: uploading batch")?;
        }

        Ok(())
    }

    fn pack_batches(&self, entries: Vec<TeamMemoryEntry>) -> Vec<Vec<TeamMemoryEntry>> {
        let mut batches: Vec<Vec<TeamMemoryEntry>> = Vec::new();
        let mut current: Vec<TeamMemoryEntry> = Vec::new();
        let mut current_size: usize = 0;

        for entry in entries {
            let entry_size = entry.key.len() + entry.content.len() + 100;

            if entry_size > MAX_PUT_BODY_BYTES {
                if !current.is_empty() {
                    batches.push(std::mem::take(&mut current));
                    current_size = 0;
                }
                batches.push(vec![entry]);
                continue;
            }

            if current_size + entry_size > MAX_PUT_BODY_BYTES && !current.is_empty() {
                batches.push(std::mem::take(&mut current));
                current_size = 0;
            }

            current_size += entry_size;
            current.push(entry);
        }

        if !current.is_empty() {
            batches.push(current)
        }

        batches
    }

    async fn upload_batch(
        &self,
        batch: Vec<TeamMemoryEntry>,
        state: &mut SyncState
    ) -> Result<()> {
        let client = reqwest::Client::new();
        let url = format!(
            "{}/api/claude_code/team_memory?repo={}",
            self.api_base,
            urlencoding::encode(&self.repo),
        );

        let body = serde_json::json!({ "entries": batch });

        let mut req = client
            .put(&url)
            .bearer_auth(&self.token)
            .json(&body);

        if let Some(etag) = &state.last_known_etag {
            req = req.header("If-Match", etag);
        }

        let response = req
            .send()
            .await
            .context("team memory: PUT request failed")?;

        let status = response.status().as_u16();

        match status {
            200 | 201 | 204 => {
                if let Some(etag) = response
                    .headers()
                    .get("etag")
                    .and_then(|v| v.to_str().ok())
                {
                    state.last_known_etag = Some(etag.to_string());
                }
                // Update local checksum map to reflect uploaded state
                for entry in &batch {
                    state
                        .server_checksums
                        .insert(entry.key.clone(), entry.checksum.clone());
                }
                Ok(())
            }
            412 => anyhow::bail!("Conflict (412 Precondition Failed): ETag mismatch, retry needed"),
            413 => anyhow::bail!("Payload too large (413)"),
            401 | 403 => anyhow::bail!("Authentication error ({})", status),
            _ => anyhow::bail!("Upload failed with status {}", status),
        }
    }

    async fn scan_local_files(&self) -> Result<Vec<TeamMemoryEntry>> {
        let mut entries = Vec::new();

        if !self.team_dir.exists() {
            return Ok(entries);
        }

        // Iterative DFS using an explicit stack to avoid deep recursion
        let mut stack = vec![self.team_dir.clone()];

        while let Some(dir) = stack.pop() {
            let mut read_dir = tokio::fs::read_dir(&dir)
                .await
                .with_context(|| format!("read_dir {:?}", dir))?;

            while let Some(entry) = read_dir.next_entry().await? {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().map(|e| e == "md").unwrap_or(false) {
                    let content = tokio::fs::read_to_string(&path)
                        .await
                        .with_context(|| format!("reading {:?}", path))?;

                    if content.len() > MAX_FILE_SIZE_BYTES {
                        continue; // Skip files that are too large
                    }

                    let key = path
                        .strip_prefix(&self.team_dir)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/");

                    let checksum = content_checksum(&content);
                    entries.push(TeamMemoryEntry { key, content, checksum });
                }
            }
        }

        entries.sort_by(|a, b| a.key.cmp(&b.key));
        Ok(entries)
    }
}

// Secret scanner
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretMatch {
    pub label: String,
}

pub fn scan_for_secrets(content: &str) -> Vec<SecretMatch> {
    const PATTERNS: &[(&str, &str)] = &[
        // Cloud providers
        (r"(?:A3T[A-Z0-9]|AKIA|ASIA|ABIA|ACCA)[A-Z2-7]{16}", "AWS access key"),
        (r"AIza[\w-]{35}", "GCP API key"),
        // AI APIs
        (r"sk-ant-api03-[a-zA-Z0-9_\-]{93}AA", "Anthropic API key"),
        (r"sk-ant-admin01-[a-zA-Z0-9_\-]{93}AA", "Anthropic admin API key"),
        (r"sk-[a-zA-Z0-9]{20}T3BlbkFJ[a-zA-Z0-9]{20}", "OpenAI API key"),
        // Version control
        (r"ghp_[0-9a-zA-Z]{36}", "GitHub personal access token"),
        (r"github_pat_\w{82}", "GitHub fine-grained PAT"),
        (r"(?:ghu|ghs)_[0-9a-zA-Z]{36}", "GitHub app token"),
        (r"gho_[0-9a-zA-Z]{36}", "GitHub OAuth token"),
        (r"glpat-[\w-]{20}", "GitLab PAT"),
        // Communication
        (r"xoxb-[0-9]{10,13}-[0-9]{10,13}[a-zA-Z0-9-]*", "Slack bot token"),
        // Crypto / private keys
        (r"-----BEGIN[ A-Z0-9_-]{0,100}PRIVATE KEY", "Private key"),
        // Payments
        (r"(?:sk|rk)_(?:test|live|prod)_[a-zA-Z0-9]{10,99}", "Stripe secret key"),
        // NPM
        (r"npm_[a-zA-Z0-9]{36}", "NPM access token"),
    ];

    let mut findings: Vec<SecretMatch> = Vec::new();

    for (pattern, label) in PATTERNS {
        if let Ok(re) = regex::Regex::new(pattern) {
            if re.is_match(content) {
                findings.push(SecretMatch { label: label.to_string() });
            }
        }
    }

    findings
}

#[cfg(test)]
mod tests {
    use super::*;
    //use tempfile::TempDir;

    #[test]
    fn test_checksum_format() {
        let cs = content_checksum("hello");
        assert!(cs.starts_with("sha256:"), "checksum should start with sha256:");
        assert_eq!(cs.len(), "sha256:".len() + 64, "sha256 hex is 64 chars");
    }


}