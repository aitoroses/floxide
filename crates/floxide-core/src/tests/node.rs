#[cfg(test)]
mod tests {
    use crate::action::*;
    use crate::error::*;
    use crate::node::*;
    use async_trait::async_trait;

    // Simple test context
    #[derive(Debug, Clone)]
    struct TestContext {
        value: i32,
    }

    // Test node that adds a value and returns a complete outcome
    struct AddNode {
        id: String,
    }

    impl AddNode {
        fn new(id: impl Into<String>) -> Self {
            Self { id: id.into() }
        }
    }

    #[async_trait]
    impl Node<TestContext, DefaultAction> for AddNode {
        type Output = i32;

        fn id(&self) -> String {
            self.id.clone()
        }

        async fn process(
            &self,
            ctx: &mut TestContext,
        ) -> Result<NodeOutcome<Self::Output, DefaultAction>, FloxideError> {
            ctx.value += 1;
            Ok(NodeOutcome::Success(ctx.value))
        }
    }

    // Test node that checks if value is positive and returns different actions based on that
    struct CheckPositiveNode {
        id: String,
    }

    impl CheckPositiveNode {
        fn new(id: impl Into<String>) -> Self {
            Self { id: id.into() }
        }
    }

    #[async_trait]
    impl Node<TestContext, DefaultAction> for CheckPositiveNode {
        type Output = bool;

        fn id(&self) -> String {
            self.id.clone()
        }

        async fn process(
            &self,
            ctx: &mut TestContext,
        ) -> Result<NodeOutcome<Self::Output, DefaultAction>, FloxideError> {
            if ctx.value > 0 {
                Ok(NodeOutcome::RouteToAction(DefaultAction::Next))
            } else {
                Ok(NodeOutcome::RouteToAction(DefaultAction::Error))
            }
        }
    }

    // Test node that fails during processing
    struct FailingNode {
        id: String,
    }

    impl FailingNode {
        fn new(id: impl Into<String>) -> Self {
            Self { id: id.into() }
        }
    }

    #[async_trait]
    impl Node<TestContext, DefaultAction> for FailingNode {
        type Output = bool;

        fn id(&self) -> String {
            self.id.clone()
        }

        async fn process(
            &self,
            _ctx: &mut TestContext,
        ) -> Result<NodeOutcome<Self::Output, DefaultAction>, FloxideError> {
            Err(FloxideError::node_execution(
                self.id(),
                "This node always fails",
            ))
        }
    }

    #[tokio::test]
    async fn test_add_node() {
        let node = AddNode::new("add");
        let mut ctx = TestContext { value: 5 };

        let result = node.process(&mut ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            NodeOutcome::Success(value) => {
                assert_eq!(value, 6);
                assert_eq!(ctx.value, 6);
            }
            _ => panic!("Expected Success outcome"),
        }
    }

    #[tokio::test]
    async fn test_check_positive_node() {
        let node = CheckPositiveNode::new("check_positive");

        // Test with positive value
        let mut ctx_positive = TestContext { value: 5 };
        let result_positive = node.process(&mut ctx_positive).await;
        assert!(result_positive.is_ok());

        match result_positive.unwrap() {
            NodeOutcome::RouteToAction(action) => {
                assert_eq!(action, DefaultAction::Next);
            }
            _ => panic!("Expected RouteToAction outcome"),
        }

        // Test with negative value
        let mut ctx_negative = TestContext { value: -5 };
        let result_negative = node.process(&mut ctx_negative).await;
        assert!(result_negative.is_ok());

        match result_negative.unwrap() {
            NodeOutcome::RouteToAction(action) => {
                assert_eq!(action, DefaultAction::Error);
            }
            _ => panic!("Expected RouteToAction outcome"),
        }
    }

    #[tokio::test]
    async fn test_failing_node() {
        let node = FailingNode::new("failing");
        let mut ctx = TestContext { value: 0 };

        let result = node.process(&mut ctx).await;
        assert!(result.is_err());

        let error = result.unwrap_err();
        match error {
            FloxideError::NodeExecution(msg) => {
                assert!(msg.contains("failing"));
                assert!(msg.contains("This node always fails"));
            }
            _ => panic!("Expected NodeExecution error"),
        }
    }
}
