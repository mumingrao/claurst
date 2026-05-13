#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EffortLevel {
    /// Quick, straightforward implementation with minimal overhead.
    Low,
    /// Balanced approach with standard implementation and testing.
    Medium,
    /// Comprehensive implementation with extensive testing and documentation.
    High,
    /// Maximum capability with deepest reasoning (Opus 4.6 only).
    Max,
}

impl EffortLevel {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "low" => Some(Self::Low),
            "medium" => Some(Self::Medium),
            "high" => Some(Self::High),
            "max" => Some(Self::Max),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Max => "max",
        }
    }

    pub fn thinking_budget_tokens(&self) -> Option<u32> {
        match self {
            Self::Low => None,
            Self::Medium => Some(5_000),
            Self::High => Some(10_000),
            Self::Max => Some(20_000),
        }
    }

    pub fn temperature(&self) -> Option<f32> {
        match self {
            Self::Low => Some(0.0),
            Self::Medium | Self::High | Self::Max => None,
        }
    }

    pub fn glyph(&self) -> &'static str {
        match self {
            Self::Low => "○",
            Self::Medium => "◐",
            Self::High => "●",
            Self::Max => "◉",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Low => "Quick, straightforward implementation with minimal overhead",
            Self::Medium => "Balanced approach with standard implementation and testing",
            Self::High => "Comprehensive implementation with extensive testing and documentation",
            Self::Max => "Maximum capability with deepest reasoning (Opus 4.6 only)",
        }
    }
}

impl std::fmt::Display for EffortLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}