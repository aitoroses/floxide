// examples/readme_example.rs
// Example for README showcasing SharedState, on_failure, and retry.

// Required dependencies in Cargo.toml:
// floxide = "*"
// tokio = { version = "1", features = ["full"] }
// async-trait = "0.1"
// serde = { version = "1", features = ["derive"] }

use floxide::{
    context::SharedState, node, workflow, BackoffStrategy, FloxideError, Node, RetryError,
    RetryPolicy, Transition, Workflow, WorkflowCtx,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;
use tracing::Level; // For simulating work

// --- Data Structures ---

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct OrderDetails {
    order_id: String,
    item: String,
    quantity: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
enum OrderStatus {
    #[default]
    Received,
    Validated,
    PaymentProcessed,
    StockAllocated,
    CustomerNotified,
    FailedValidation, // From Validate Node Error
    FailedPayment,    // Explicit status set by PaymentFailedNode
    FailedStock,      // Explicit status set by StockAllocationFailedNode
}

// --- Shared Context ---

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
struct OrderContext {
    status: SharedState<OrderStatus>,
    error_message: SharedState<Option<String>>,
    stock_allocation_retries: SharedState<u32>,
}

// --- Workflow Nodes ---

// Node 1: Validate Order
node! {
    pub struct ValidateOrderNode {};
    context = OrderContext;
    input = OrderDetails;
    output = OrderDetails;
    |ctx, details| {
        tracing::info!("Validating order: {}", details.order_id);
        sleep(Duration::from_millis(50)).await;
        if details.quantity == 0 {
            let err_msg = "Validation Failed: Quantity cannot be zero.".to_string();
            return Err(FloxideError::Generic(err_msg));
        }
        ctx.status.set(OrderStatus::Validated).await;
        Ok(Transition::Next(details))
    }
}

// Node 2: Process Payment
node! {
    pub struct ProcessPaymentNode {};
    context = OrderContext;
    input = OrderDetails;
    output = OrderDetails;
    |ctx, details| {
        tracing::info!("Processing payment for order: {}", details.order_id);
        sleep(Duration::from_millis(100)).await;
        if details.item == "FailPayment" {
             let err_msg = "Payment Failed: Card declined.".to_string();
             return Err(FloxideError::Generic(err_msg));
        }
        ctx.status.set(OrderStatus::PaymentProcessed).await;
        Ok(Transition::Next(details))
    }
}

// Node 3: Allocate Stock (Now with retry logic simulation)
node! {
    pub struct AllocateStockNode {};
    context = OrderContext;
    input = OrderDetails;
    output = OrderDetails;
    | ctx, details | {
        tracing::info!("Allocating stock for order: {}", details.order_id);
        sleep(Duration::from_millis(50)).await;
        if details.item == "RetryStock" {
            // let retries = *ctx.stock_allocation_retries.get().await;
            // ctx.stock_allocation_retries.set(retries + 1).await;
            // if retries < 1 {
                return Err(FloxideError::Generic("Stock allocation failed.".to_string()));
            // }
        }
        tracing::info!("Stock allocated successfully.");
        ctx.status.set(OrderStatus::StockAllocated).await;
        Ok(Transition::Next(details))
    }
}

// Node 4: Notify Customer (Final Node)
node! {
    pub struct NotifyCustomerNode {};
    context = OrderContext;
    input = OrderDetails;
    output = OrderStatus; // Workflow output is the final status
    |ctx, details| {
        tracing::info!("Notifying customer order: {}", details.order_id);
        sleep(Duration::from_millis(50)).await;
        ctx.status.set(OrderStatus::CustomerNotified).await;
        let final_status = ctx.status.get().await.clone();
        Ok(Transition::Next(final_status))
    }
}

// --- Failure Handling Nodes ---

// Node F1: Handle Payment Failure
node! {
    pub struct PaymentFailedNode {};
    context = OrderContext;
    input = OrderDetails;
    output = OrderDetails;
    |ctx, details| {
        tracing::info!("Executing PaymentFailedNode due to: {:?}", details);
        ctx.status.set(OrderStatus::FailedPayment).await;
        Ok(Transition::Next(details))
    }
}

// Node F2: Handle Stock Allocation Failure (After Retries)
node! {
    pub struct StockAllocationFailedNode {};
    context = OrderContext;
    input = OrderDetails;
    output = OrderDetails;
    |ctx, details| {
        tracing::info!("Compensating payment for order: {:?}", details);
        ctx.status.set(OrderStatus::FailedStock).await;
        Ok(Transition::Next(details))
    }
}

// --- Workflow Definition ---
workflow! {
    pub struct OrderProcessingWorkflow {
        validate: ValidateOrderNode,
        payment: ProcessPaymentNode,
        payment_failed: PaymentFailedNode,
        #[retry = stock_retry_policy]
        allocate_stock: AllocateStockNode,
        stock_retry_policy: RetryPolicy,
        stock_failed: StockAllocationFailedNode,
        notify: NotifyCustomerNode,
    }
    context = OrderContext;
    start = validate;
    edges {
        validate => { [payment] };
        payment => { [allocate_stock] };
        payment on_failure => { [payment_failed] };
        allocate_stock => { [notify] };
        allocate_stock on_failure => { [stock_failed] };
        payment_failed => { [notify] };
        stock_failed => { [notify] };
        notify => {};
    }
}

// --- Main Execution ---

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();
    // --- Run 1: Successful Order ---
    tracing::info!("--- Running Successful Order ---");
    run_workflow(
        OrderDetails {
            order_id: "ORD-123".to_string(),
            item: "Laptop".to_string(),
            quantity: 1,
        },
        true,
    )
    .await;

    // // --- Run 2: Order that fails payment ---
    tracing::info!("--- Running Order Failing Payment ---");
    run_workflow(
        OrderDetails {
            order_id: "ORD-456".to_string(),
            item: "FailPayment".to_string(), // This causes payment failure
            quantity: 2,
        },
        false,
    )
    .await;

    // --- Run 3: Order that requires stock retry ---
    tracing::info!("--- Running Order Requiring Stock Retry ---");
    run_workflow(
        OrderDetails {
            order_id: "ORD-789".to_string(),
            item: "RetryStock".to_string(), // This causes transient stock failure
            quantity: 1,
        },
        false,
    )
    .await;

    // --- Print DOT Visualization ---
}

// Helper function to run a workflow and print results
async fn run_workflow(details: OrderDetails, print_dot: bool) {
    let workflow = OrderProcessingWorkflow {
        validate: ValidateOrderNode {},
        payment: ProcessPaymentNode {},
        stock_retry_policy: RetryPolicy::new(
            3,
            Duration::from_millis(50),
            Duration::from_secs(1),
            BackoffStrategy::Linear,
            RetryError::All,
        ),
        payment_failed: PaymentFailedNode {},
        allocate_stock: AllocateStockNode {},
        stock_failed: StockAllocationFailedNode {},
        notify: NotifyCustomerNode {},
    };

    // Print DOT Visualization
    if print_dot {
        println!("{}", workflow.to_dot());
    }

    let ctx = OrderContext::default(); // New context for each run
    let wf_ctx = WorkflowCtx::new(ctx);
    tracing::info!("Starting workflow for Order ID: {}", details.order_id);
    let result = workflow.run(&wf_ctx, details).await;

    let final_status = wf_ctx.store.status.get().await.clone();
    let error_msg = wf_ctx.store.error_message.get().await.clone();

    match result {
        Ok(status_from_result) => {
            tracing::info!(
                "Workflow completed. Result Status: {:?}, Context Status: {:?}, Error: {:?}",
                status_from_result,
                final_status,
                error_msg
            );
        }
        Err(e) => {
            // This happens if validation fails (or another node fails without on_failure)
            tracing::error!(
                "Workflow failed unexpectedly: {:?}. Context Status: {:?}, Error: {:?}",
                e,
                final_status,
                error_msg
            );
        }
    }
    tracing::info!(
        "Final Context State: {:?}",
        *wf_ctx.store.status.get().await
    );

    tracing::info!("------\n");
}
