#[cfg(test)]
mod tests {
    use crate::action::*;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    #[test]
    fn test_default_action_implements_action_type() {
        fn is_action_type<T: ActionType>(_: &T) -> bool {
            true
        }

        let action = DefaultAction::Next;
        assert!(is_action_type(&action));
    }

    #[test]
    fn test_default_action_equality() {
        let action1 = DefaultAction::Next;
        let action2 = DefaultAction::Next;
        let action3 = DefaultAction::Default;

        assert_eq!(action1, action2);
        assert_ne!(action1, action3);
    }

    #[test]
    fn test_default_action_clone() {
        let action = DefaultAction::Next;
        let cloned = action.clone();

        assert_eq!(action, cloned);
    }

    #[test]
    fn test_default_action_hash() {
        let action1 = DefaultAction::Next;
        let action2 = DefaultAction::Next;

        let mut hasher1 = DefaultHasher::new();
        let mut hasher2 = DefaultHasher::new();

        action1.hash(&mut hasher1);
        action2.hash(&mut hasher2);

        assert_eq!(hasher1.finish(), hasher2.finish());
    }

    #[test]
    fn test_custom_action_type() {
        // Define a custom action type
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        enum PaymentAction {
            PaymentReceived,
            PaymentDeclined,
            RefundRequested,
        }

        impl ActionType for PaymentAction {
            fn name(&self) -> &str {
                match self {
                    Self::PaymentReceived => "payment_received",
                    Self::PaymentDeclined => "payment_declined",
                    Self::RefundRequested => "refund_requested",
                }
            }
        }

        // Test that it works as an ActionType
        fn is_action_type<T: ActionType>(_: &T) -> bool {
            true
        }

        let action = PaymentAction::PaymentReceived;
        assert!(is_action_type(&action));
    }
}
