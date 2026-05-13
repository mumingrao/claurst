use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AutoApproveMode {
    /// No auto-approve — all tool calls require confirmation.
    #[default]
    None,
    /// Auto-approve edits to existing files, but not new files or commands.
    AcceptEdits,
    /// Bypass all permissions — approve everything including bash commands.
    BypassPermissions,
    /// Auto-approve with plan mode — shows plan before execution.
    Plan,
}

impl AutoApproveMode {
    pub fn auto_approves_bash(&self) -> bool {
        matches!(self, Self::BypassPermissions)
    }

    pub fn auto_approves_edits(&self) -> bool {
        matches!(self, Self::AcceptEdits | Self::BypassPermissions)
    }

    pub fn is_plan_mode(&self) -> bool {
        matches!(self, Self::Plan)
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::None => "",
            Self::AcceptEdits => "auto-edit",
            Self::BypassPermissions => "bypass",
            Self::Plan => "plan-mode",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AutoModeState {
    pub mode: AutoApproveMode,
    pub warning_accepted: bool,
    pub activated_session: Option<String>,
    pub activated_turn: Option<u32>,
}

impl AutoModeState {
    pub fn new(mode: AutoApproveMode) -> Self {
        Self {
            mode,
            warning_accepted: false,
            activated_session: None,
            activated_turn: None,
        }
    }

    pub fn activate_bypass(&mut self, session_id: &str, turn: u32) {
        self.mode = AutoApproveMode::BypassPermissions;
        self.warning_accepted = true;
        self.activated_session = Some(session_id.to_string());
        self.activated_turn = Some(turn);
    }

    pub fn reset(&mut self) {
        self.mode = AutoApproveMode::None;
        self.warning_accepted = false;
    }
}