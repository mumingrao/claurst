use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TokenWarningLevel {
    /// < 80% used — no warning.
    None,
    /// >= 80% used — show caution indicator.
    Warning,
    /// >= 95% used — show critical warning; compact strongly recommended.
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBudget {
    pub tokens_used: u64,
    pub context_window: u64,
    pub tokens_remaining: u64,
    pub fill_fraction: f64,
    pub warning_level: TokenWarningLevel,
}

impl TokenBudget {
    pub fn new(tokens_used: u64, context_window: u64) -> Self {
        let remaining = context_window.saturating_sub(tokens_used);
        let fraction = if context_window == 0 {
            0.0
        } else {
            tokens_used as f64 / context_window as f64
        };
        let warning_level = if fraction >= 0.95 {
            TokenWarningLevel::Critical
        } else if fraction >= 0.80 {
            TokenWarningLevel::Warning
        } else {
            TokenWarningLevel::None
        };
        Self {
            tokens_used,
            context_window,
            tokens_remaining: remaining,
            fill_fraction: fraction,
            warning_level,
        }
    }

    pub fn should_compact(&self) -> bool {
        self.fill_fraction >= 0.90
    }

    pub fn should_collapse(&self) -> bool {
        self.fill_fraction >= 0.97
    }

    pub fn display(&self) -> String {
        format!(
            "{} / {} ({:.0}%)",
            format_token_count(self.tokens_used),
            format_token_count(self.context_window),
            self.fill_fraction * 100.0,
        )
    }
}

pub fn format_token_count(count: u64) -> String {
    if count >= 1_000_000 {
        format!("{:.1}M", count as f64 / 1_000_000.0)
    } else if count >= 1_000 {
        format!("{:.1}K", count as f64 / 1_000.0)
    } else {
        count.to_string()
    }
}