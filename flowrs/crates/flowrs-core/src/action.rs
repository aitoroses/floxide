use std::fmt::{self, Debug};
use std::hash::Hash;

/// Trait for action types that can be used for node transitions
pub trait ActionType: Clone + PartialEq + Eq + std::hash::Hash + Send + Sync + 'static {
    /// Get the name of this action
    fn name(&self) -> &str;
}

/// Default action type for simple workflows
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DefaultAction {
    /// Default action (used for fallback routing)
    Default,
    /// Next node in sequence
    Next,
    /// Error handler
    Error,
    /// Custom named action
    Custom(String),
}

impl DefaultAction {
    /// Create a new custom action with the given name
    pub fn new(name: impl Into<String>) -> Self {
        Self::Custom(name.into())
    }
}

impl Default for DefaultAction {
    fn default() -> Self {
        Self::Default
    }
}

impl ActionType for DefaultAction {
    fn name(&self) -> &str {
        match self {
            Self::Default => "default",
            Self::Next => "next",
            Self::Error => "error",
            Self::Custom(name) => name,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_action_name() {
        assert_eq!(DefaultAction::Default.name(), "default");
        assert_eq!(DefaultAction::Next.name(), "next");
        assert_eq!(DefaultAction::Error.name(), "error");
        assert_eq!(DefaultAction::Custom("custom".into()).name(), "custom");
    }

    #[test]
    fn test_default_action_creation() {
        let action = DefaultAction::new("test-action");
        assert_eq!(action, DefaultAction::Custom("test-action".into()));
        assert_eq!(action.name(), "test-action");
    }
}
