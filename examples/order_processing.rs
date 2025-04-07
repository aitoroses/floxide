// Order Processing Workflow: A Complete Business Process Example
//
// This example demonstrates how to implement a complete order processing workflow
// using the Flow Framework. It models a realistic business process with multiple
// stages, error handling, and conditional routing.
//
// Key concepts demonstrated:
// 1. Custom action types for specialized workflow routing
// 2. Multi-stage business process implementation
// 3. Error handling and recovery with retry mechanisms
// 4. State management across workflow stages
// 5. Conditional execution paths based on business rules
//
// The workflow implements a complete order processing system:
// - Order validation: Checks order details for completeness and validity
// - Payment processing: Handles payment with retry capability
// - Order fulfillment: Manages shipping and delivery
// - Error handling: Provides graceful failure paths and notifications
//
// This pattern is particularly useful for:
// - Business process automation
// - E-commerce and order management systems
// - Multi-stage workflows with conditional paths
// - Processes requiring audit trails and state tracking
//
// This example is designed in accordance with:
// - ADR-0008: Business Process Workflow Pattern
// - ADR-0011: Error Handling and Recovery Strategy

use async_trait::async_trait;
use floxide_core::{
    node, ActionType, FloxideError, Node, NodeId, NodeOutcome, RetryNode, Workflow,
};
use std::time::Duration;
use tracing::{error, info, warn};
use uuid::Uuid;

/// Custom action type for our order workflow
///
/// This enum defines the possible routing actions within the order processing
/// workflow. Each action represents a different path the workflow can take.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum OrderAction {
    /// Default action, used when no specific routing is needed
    Default,
    /// Proceed to the next step in the normal flow
    Next,
    /// An error occurred that requires special handling
    Error,
    /// Cancel the order and stop processing
    _CancelOrder,
    /// Order was successfully processed
    Success,
    /// Order processing failed
    Failure,
}

impl Default for OrderAction {
    fn default() -> Self {
        Self::Default
    }
}

impl ActionType for OrderAction {
    /// Returns a string representation of the action
    ///
    /// This is used for logging and debugging purposes.
    fn name(&self) -> &str {
        match self {
            Self::Default => "default",
            Self::Next => "next",
            Self::Error => "error",
            Self::_CancelOrder => "cancel_order",
            Self::Success => "success",
            Self::Failure => "failure",
        }
    }
}

/// An item in an order
///
/// Represents a product or service that a customer has ordered,
/// including its name, price, and quantity.
#[allow(dead_code)]
#[derive(Debug, Clone)]
struct OrderItem {
    /// Name or description of the item
    name: String,
    /// Price per unit
    price: f64,
    /// Number of units ordered
    quantity: u32,
}

/// Possible order statuses
///
/// Represents the current state of an order as it moves
/// through the processing workflow.
#[derive(Debug, Clone, PartialEq)]
enum OrderStatus {
    /// Order has been created but not yet validated
    Created,
    /// Order has been validated and is ready for payment
    Validated,
    /// Payment has been processed successfully
    PaymentProcessed,
    /// Order has been shipped
    Shipped,
    /// Order has been delivered to the customer
    Delivered,
    /// Order has been cancelled
    Cancelled,
}

/// Context for an order processing workflow
///
/// This struct maintains the state of an order as it moves through
/// the processing workflow, including customer details, items,
/// payment information, and processing history.
#[derive(Debug, Clone)]
struct OrderContext {
    /// Unique identifier for the order
    order_id: String,
    /// Customer identifier
    _customer_id: String,
    /// Items included in the order
    items: Vec<OrderItem>,
    /// Total amount of the order
    total_amount: f64,
    /// Shipping address for delivery
    shipping_address: Option<String>,
    /// Processing notes and history
    notes: Vec<String>,
    /// Current status of the order
    status: OrderStatus,
    /// Number of payment attempts
    payment_attempts: u32,
}

impl OrderContext {
    /// Create a new order context
    ///
    /// Initializes a new order with the given customer ID, items, and
    /// shipping address. Sets the initial status to Created.
    fn new(customer_id: &str, items: Vec<OrderItem>, shipping_address: Option<String>) -> Self {
        let total = items
            .iter()
            .map(|item| item.price * item.quantity as f64)
            .sum();

        Self {
            order_id: Uuid::new_v4().to_string(),
            _customer_id: customer_id.to_string(),
            items,
            total_amount: total,
            shipping_address,
            notes: Vec::new(),
            status: OrderStatus::Created,
            payment_attempts: 0,
        }
    }

    /// Add a note to the order
    ///
    /// Records a processing note or event in the order history.
    fn add_note(&mut self, note: &str) {
        self.notes.push(note.to_string());
    }

    /// Calculate order subtotal (before tax/shipping)
    ///
    /// Computes the total cost of all items in the order.
    fn calculate_subtotal(&self) -> f64 {
        self.items
            .iter()
            .map(|item| item.price * item.quantity as f64)
            .sum()
    }

    /// Update order status
    ///
    /// Changes the current status of the order and records the transition.
    fn update_status(&mut self, status: OrderStatus) {
        self.status = status;
    }
}

/// A node that validates the order
///
/// This node checks that the order has all required information
/// and meets business rules before proceeding to payment.
#[derive(Debug, Clone)]
struct ValidateOrderNode {
    /// Unique identifier for this node
    id: String,
}

impl ValidateOrderNode {
    /// Creates a new ValidateOrderNode with a random UUID
    fn new() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
        }
    }
}

#[async_trait]
impl Node<OrderContext, OrderAction> for ValidateOrderNode {
    type Output = ();

    /// Returns the unique identifier for this node
    fn id(&self) -> NodeId {
        self.id.clone()
    }

    /// Validates the order and determines the next action
    ///
    /// This method:
    /// 1. Checks if the order has items
    /// 2. Verifies that a shipping address is provided
    /// 3. Updates the order status if validation passes
    /// 4. Returns an appropriate action based on validation results
    async fn process(
        &self,
        ctx: &mut OrderContext,
    ) -> Result<NodeOutcome<Self::Output, OrderAction>, FloxideError> {
        info!("Validating order {}", ctx.order_id);

        // Check if order has items
        if ctx.items.is_empty() {
            ctx.add_note("Validation failed: Order has no items");
            return Err(FloxideError::node_execution(
                self.id(),
                "Order has no items",
            ));
        }

        // Check if total is correct
        let calculated_total = ctx.calculate_subtotal();
        if (calculated_total - ctx.total_amount).abs() > 0.01 {
            ctx.add_note(&format!(
                "Validation failed: Total amount mismatch: expected {}, got {}",
                calculated_total, ctx.total_amount
            ));
            return Err(FloxideError::node_execution(
                self.id(),
                "Total amount mismatch",
            ));
        }

        // Check if shipping address is provided
        if ctx.shipping_address.is_none() {
            ctx.add_note("Validation failed: No shipping address provided");
            return Err(FloxideError::node_execution(
                self.id(),
                "No shipping address provided",
            ));
        }

        // Update status and add note
        ctx.update_status(OrderStatus::Validated);
        ctx.add_note("Order validated successfully");

        info!("Order {} validated successfully", ctx.order_id);
        Ok(NodeOutcome::RouteToAction(OrderAction::Next))
    }
}

/// A node that processes payment for the order
///
/// This node handles payment processing with retry capability.
#[derive(Debug, Clone)]
struct ProcessPaymentNode {
    /// Unique identifier for this node
    id: String,
    /// Whether payment processing should fail (for testing)
    should_fail: bool,
}

impl ProcessPaymentNode {
    /// Creates a new ProcessPaymentNode with a random UUID
    fn new(should_fail: bool) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            should_fail,
        }
    }
}

#[async_trait]
impl Node<OrderContext, OrderAction> for ProcessPaymentNode {
    type Output = ();

    /// Returns the unique identifier for this node
    fn id(&self) -> NodeId {
        self.id.clone()
    }

    /// Processes payment for the order
    ///
    /// This method:
    /// 1. Increments the payment attempts counter
    /// 2. Simulates payment processing with a delay
    /// 3. Returns an error if payment fails (based on should_fail flag)
    /// 4. Updates the order status if payment succeeds
    /// 5. Returns an appropriate action based on payment results
    async fn process(
        &self,
        ctx: &mut OrderContext,
    ) -> Result<NodeOutcome<Self::Output, OrderAction>, FloxideError> {
        ctx.payment_attempts += 1;
        info!(
            "Processing payment for order {} (attempt {})",
            ctx.order_id, ctx.payment_attempts
        );

        // Simulate payment processing
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Simulate payment failure if configured
        if self.should_fail {
            let note = format!(
                "Payment processing failed on attempt {}",
                ctx.payment_attempts
            );
            ctx.add_note(&note);
            warn!("{}", note);
            return Err(FloxideError::node_execution(
                self.id(),
                format!("Payment failed on attempt {}", ctx.payment_attempts),
            ));
        }

        // Payment succeeded
        ctx.update_status(OrderStatus::PaymentProcessed);
        ctx.add_note(&format!(
            "Payment processed successfully on attempt {}",
            ctx.payment_attempts
        ));

        info!("Payment for order {} processed successfully", ctx.order_id);
        Ok(NodeOutcome::RouteToAction(OrderAction::Next))
    }
}

/// A node that ships the order
///
/// This node manages shipping and delivery of the order.
#[derive(Debug, Clone)]
struct ShipOrderNode {
    /// Unique identifier for this node
    id: String,
}

impl ShipOrderNode {
    /// Creates a new ShipOrderNode with a random UUID
    fn new() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
        }
    }
}

#[async_trait]
impl Node<OrderContext, OrderAction> for ShipOrderNode {
    type Output = ();

    /// Returns the unique identifier for this node
    fn id(&self) -> NodeId {
        self.id.clone()
    }

    /// Ships the order
    ///
    /// This method:
    /// 1. Retrieves the shipping address from the order context
    /// 2. Updates the order status to Shipped
    /// 3. Adds a note to the order history
    /// 4. Returns an appropriate action based on shipping results
    async fn process(
        &self,
        ctx: &mut OrderContext,
    ) -> Result<NodeOutcome<Self::Output, OrderAction>, FloxideError> {
        info!("Shipping order {}", ctx.order_id);

        // Clone needed values before mutating ctx
        let address = ctx.shipping_address.as_ref().unwrap().clone();

        ctx.add_note(&format!("Order shipped to: {}", address));
        ctx.update_status(OrderStatus::Shipped);

        info!("Order {} shipped to {}", ctx.order_id, address);
        Ok(NodeOutcome::RouteToAction(OrderAction::Next))
    }
}

/// A node that delivers the order
///
/// This node manages delivery of the order to the customer.
#[derive(Debug, Clone)]
struct DeliverOrderNode {
    /// Unique identifier for this node
    id: String,
}

impl DeliverOrderNode {
    /// Creates a new DeliverOrderNode with a random UUID
    fn new() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
        }
    }
}

#[async_trait]
impl Node<OrderContext, OrderAction> for DeliverOrderNode {
    type Output = ();

    /// Returns the unique identifier for this node
    fn id(&self) -> NodeId {
        self.id.clone()
    }

    /// Delivers the order
    ///
    /// This method:
    /// 1. Updates the order status to Delivered
    /// 2. Adds a note to the order history
    /// 3. Returns an appropriate action based on delivery results
    async fn process(
        &self,
        ctx: &mut OrderContext,
    ) -> Result<NodeOutcome<Self::Output, OrderAction>, FloxideError> {
        info!("Delivering order {}", ctx.order_id);

        ctx.update_status(OrderStatus::Delivered);
        ctx.add_note("Order delivered successfully");

        info!("Order {} delivered successfully", ctx.order_id);
        Ok(NodeOutcome::RouteToAction(OrderAction::Success))
    }
}

/// A node that cancels the order
///
/// This node cancels the order and stops processing.
#[derive(Debug, Clone)]
struct CancelOrderNode {
    /// Unique identifier for this node
    id: String,
}

impl CancelOrderNode {
    /// Creates a new CancelOrderNode with a random UUID
    fn new() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
        }
    }
}

#[async_trait]
impl Node<OrderContext, OrderAction> for CancelOrderNode {
    type Output = ();

    /// Returns the unique identifier for this node
    fn id(&self) -> NodeId {
        self.id.clone()
    }

    /// Cancels the order
    ///
    /// This method:
    /// 1. Updates the order status to Cancelled
    /// 2. Adds a note to the order history
    /// 3. Returns an appropriate action based on cancellation results
    async fn process(
        &self,
        ctx: &mut OrderContext,
    ) -> Result<NodeOutcome<Self::Output, OrderAction>, FloxideError> {
        info!("Cancelling order {}", ctx.order_id);

        ctx.update_status(OrderStatus::Cancelled);
        ctx.add_note("Order was cancelled");

        warn!("Order {} was cancelled", ctx.order_id);
        Ok(NodeOutcome::RouteToAction(OrderAction::Failure))
    }
}

/// Create a notification node with a custom message
///
/// This function creates a new node that logs a notification message
/// and adds it to the order history.
fn create_notification_node(message: String) -> impl Node<OrderContext, OrderAction, Output = ()> {
    node(move |mut ctx: OrderContext| {
        let msg = message.clone();
        async move {
            info!("{}: {}", msg, ctx.order_id);
            ctx.add_note(&format!("Notification: {}", msg));
            Ok((ctx, NodeOutcome::<(), OrderAction>::Success(())))
        }
    })
}

/// Create an order processing workflow
///
/// This function creates a new workflow that processes an order from
/// creation to delivery, including payment processing and error handling.
fn create_order_workflow(should_fail_payment: bool) -> Workflow<OrderContext, OrderAction> {
    // Create all the nodes
    let validate_node = ValidateOrderNode::new();
    let validate_id = validate_node.id();

    let payment_node = ProcessPaymentNode::new(should_fail_payment);
    let _payment_id = payment_node.id();

    // Create a retry node for payment processing
    let payment_with_retry = RetryNode::with_exponential_backoff(
        payment_node,
        3,                          // Max 3 retries
        Duration::from_millis(100), // Start with 100ms
        Duration::from_secs(1),     // Max delay 1 second
    );
    let payment_retry_id = payment_with_retry.id();

    let shipping_node = ShipOrderNode::new();
    let shipping_id = shipping_node.id();

    let delivery_node = DeliverOrderNode::new();
    let delivery_id = delivery_node.id();

    let cancel_node = CancelOrderNode::new();
    let cancel_id = cancel_node.id();

    let order_created_notification = create_notification_node("Order created".to_string());
    let created_notif_id = order_created_notification.id();

    let order_completed_notification = create_notification_node("Order completed".to_string());
    let completed_notif_id = order_completed_notification.id();

    // Build workflow
    let mut workflow = Workflow::new(order_created_notification);
    workflow
        .add_node(validate_node)
        .add_node(payment_with_retry)
        .add_node(shipping_node)
        .add_node(delivery_node)
        .add_node(cancel_node)
        .add_node(order_completed_notification)
        // Connect the nodes with proper transitions
        .set_default_route(&created_notif_id, &validate_id)
        .set_default_route(&validate_id, &payment_retry_id)
        .set_default_route(&payment_retry_id, &shipping_id)
        .set_default_route(&shipping_id, &delivery_id)
        .connect(&delivery_id, OrderAction::Success, &completed_notif_id)
        // Error handling
        .connect(&validate_id, OrderAction::Error, &cancel_id)
        .connect(&payment_retry_id, OrderAction::Error, &cancel_id);

    workflow
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create a valid order
    let items = vec![
        OrderItem {
            name: "Book".to_string(),
            price: 15.99,
            quantity: 2,
        },
        OrderItem {
            name: "T-shirt".to_string(),
            price: 20.00,
            quantity: 1,
        },
    ];

    let mut valid_order = OrderContext::new(
        "customer123",
        items.clone(),
        Some("123 Main St, Anytown".to_string()),
    );

    // Create a workflow that should succeed
    let success_workflow = create_order_workflow(false);

    // Process the valid order
    info!("Processing valid order");
    if let Err(err) = success_workflow.execute(&mut valid_order).await {
        error!("Workflow failed: {}", err);
    }

    // Log the final state
    info!("Valid order final status: {:?}", valid_order.status);
    info!("Valid order notes:");
    for (i, note) in valid_order.notes.iter().enumerate() {
        info!("  {}. {}", i + 1, note);
    }

    // Create a workflow that should fail payment
    let failure_workflow = create_order_workflow(true);

    // Create another order that will fail payment
    let mut failing_order = OrderContext::new(
        "customer456",
        items,
        Some("456 Oak Ave, Somewhere".to_string()),
    );

    // Process the order with failing payment
    info!("\nProcessing order with failing payment");
    if let Err(err) = failure_workflow.execute(&mut failing_order).await {
        warn!("Workflow failed as expected: {}", err);
    }

    // Log the final state
    info!("Failing order final status: {:?}", failing_order.status);
    info!("Failing order notes:");
    for (i, note) in failing_order.notes.iter().enumerate() {
        info!("  {}. {}", i + 1, note);
    }
}
