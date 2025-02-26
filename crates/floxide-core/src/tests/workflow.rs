#[cfg(test)]
mod tests {
    use crate::action::*;
    use crate::error::*;
    use crate::node::*;
    use crate::workflow::*;
    use async_trait::async_trait;

    // Simple test context
    #[derive(Debug, Clone)]
    struct TestContext {
        value: i32,
        path: Vec<String>,
    }

    // Define an Operation enum to represent different arithmetic operations
    #[derive(Debug, Clone, Copy)]
    enum Operation {
        Add,
        Multiply,
        Subtract,
    }

    // Arithmetic node that performs operations and returns different actions based on thresholds
    #[derive(Debug)]
    struct ArithmeticNode {
        id: NodeId,
        name: String,
        operation: Operation,
        operand: i32,
        threshold: i32,
    }

    impl ArithmeticNode {
        fn new(name: &str, operation: Operation, operand: i32, threshold: i32) -> Self {
            Self {
                id: uuid::Uuid::new_v4().to_string(),
                name: name.to_string(),
                operation,
                operand,
                threshold,
            }
        }

        fn perform_operation(&self, value: i32) -> i32 {
            match self.operation {
                Operation::Add => value + self.operand,
                Operation::Multiply => value * self.operand,
                Operation::Subtract => value - self.operand,
            }
        }
    }

    #[async_trait]
    impl Node<TestContext, DefaultAction> for ArithmeticNode {
        type Output = i32;

        fn id(&self) -> NodeId {
            self.id.clone()
        }

        async fn process(
            &self,
            ctx: &mut TestContext,
        ) -> Result<NodeOutcome<Self::Output, DefaultAction>, FloxideError> {
            // Record that we visited this node
            ctx.path.push(self.name.clone());

            // Perform the operation
            ctx.value = self.perform_operation(ctx.value);

            // Choose transition based on threshold and node name
            if self.name == "subtract" {
                // For the subtract node (usually the last node in tests),
                // always return Success to end the workflow
                Ok(NodeOutcome::Success(ctx.value))
            } else if ctx.value > self.threshold {
                // Larger than threshold - route to next node
                Ok(NodeOutcome::RouteToAction(DefaultAction::Next))
            } else if ctx.value < 0 {
                // Negative - route to error handler
                Ok(NodeOutcome::RouteToAction(DefaultAction::Error))
            } else {
                // Otherwise - success
                Ok(NodeOutcome::Success(ctx.value))
            }
        }
    }

    #[tokio::test]
    async fn test_simple_workflow_execution() {
        // Create a simple add node
        let add_node = ArithmeticNode::new("add", Operation::Add, 5, 100);

        // Create workflow with just the add node
        let workflow = Workflow::new(add_node);

        // Execute workflow with initial value 10
        let mut ctx = TestContext {
            value: 10,
            path: Vec::new(),
        };

        let result = workflow.execute(&mut ctx).await;

        // Should succeed and return 15
        assert!(result.is_ok());
        assert_eq!(ctx.value, 15); // 10 + 5 = 15
        assert_eq!(ctx.path, vec!["add"]);
    }

    #[tokio::test]
    async fn test_linear_workflow() {
        // Create nodes for a linear workflow
        let add_node = ArithmeticNode::new("add", Operation::Add, 5, 10);
        let add_id = add_node.id();

        let multiply_node = ArithmeticNode::new("multiply", Operation::Multiply, 2, 20);
        let multiply_id = multiply_node.id();

        let subtract_node = ArithmeticNode::new("subtract", Operation::Subtract, 15, 0);
        let subtract_id = subtract_node.id();

        // Build workflow
        let mut workflow = Workflow::new(add_node);

        // Add the nodes
        workflow.add_node(multiply_node);
        workflow.add_node(subtract_node);

        // Connect the nodes explicitly with the Next action
        workflow.connect(&add_id, DefaultAction::Next, &multiply_id);
        workflow.connect(&multiply_id, DefaultAction::Next, &subtract_id);

        // Execute workflow with initial value 6
        let mut ctx = TestContext {
            value: 6,
            path: Vec::new(),
        };

        let result = workflow.execute(&mut ctx).await;

        // Print out error if there is one
        if let Err(ref err) = result {
            eprintln!("Workflow execution error: {:?}", err);
        }

        // Should succeed with the following calculations:
        // 6 + 5 = 11 (> 10, so route to Next)
        // 11 * 2 = 22 (> 20, so route to Next)
        // 22 - 15 = 7 (not > 0, so return success)
        assert!(result.is_ok());
        assert_eq!(ctx.value, 7);
        assert_eq!(ctx.path, vec!["add", "multiply", "subtract"]);
    }

    #[tokio::test]
    async fn test_workflow_without_default_route() {
        // Create a node with no outgoing connections
        // Use a threshold of 100 to make sure it returns Success (value is 11 < 100)
        let add_node = ArithmeticNode::new("add", Operation::Add, 5, 100);

        // Build workflow with just this node - no connections needed
        let workflow = Workflow::new(add_node);

        // Execute workflow with initial value 6
        let mut ctx = TestContext {
            value: 6,
            path: Vec::new(),
        };

        let result = workflow.execute(&mut ctx).await;

        // Print out error if there is one
        if let Err(ref err) = result {
            eprintln!("Workflow execution error: {:?}", err);
        }

        // Should succeed and finish after the first node
        assert!(result.is_ok());
        assert_eq!(ctx.value, 11); // 6 + 5 = 11
        assert_eq!(ctx.path, vec!["add"]);
    }

    #[tokio::test]
    async fn test_workflow_with_error_handler() {
        // Create nodes - the add node will produce a negative value that triggers the error route
        let add_node = ArithmeticNode::new("add", Operation::Add, -15, 10); // Will make value negative
        let add_id = add_node.id();

        // Create a special error handler that always returns Success
        // We can't use ArithmeticNode directly because it would route based on threshold
        #[derive(Debug)]
        struct ErrorHandlerNode {
            id: NodeId,
        }

        impl ErrorHandlerNode {
            fn new() -> Self {
                Self {
                    id: uuid::Uuid::new_v4().to_string(),
                }
            }
        }

        #[async_trait]
        impl Node<TestContext, DefaultAction> for ErrorHandlerNode {
            type Output = i32;

            fn id(&self) -> NodeId {
                self.id.clone()
            }

            async fn process(
                &self,
                ctx: &mut TestContext,
            ) -> Result<NodeOutcome<Self::Output, DefaultAction>, FloxideError> {
                // Record that this node was visited
                ctx.path.push("error_handler".to_string());

                // Just return success, value unchanged
                Ok(NodeOutcome::Success(ctx.value))
            }
        }

        let error_handler = ErrorHandlerNode::new();
        let error_id = error_handler.id();

        // Build workflow with error handling
        let mut workflow = Workflow::new(add_node);
        workflow.add_node(error_handler);

        // Connect using the Error action
        workflow.connect(&add_id, DefaultAction::Error, &error_id);

        // Execute workflow with initial value 5
        let mut ctx = TestContext {
            value: 5,
            path: Vec::new(),
        };

        let result = workflow.execute(&mut ctx).await;

        // Print out error if there is one
        if let Err(ref err) = result {
            eprintln!("Workflow execution error: {:?}", err);
        }

        // Should succeed by following the error path
        // 5 + (-15) = -10 (< 0, so route to error)
        // Error handler returns Success
        assert!(result.is_ok());
        assert_eq!(ctx.value, -10);
        assert_eq!(ctx.path, vec!["add", "error_handler"]);
    }
}
