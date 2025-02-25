use chrono::{Duration as ChronoDuration, Timelike, Utc, Weekday};
use flowrs_core::DefaultAction;
use flowrs_timer::{
    Schedule, SimpleTimer, TimerActionExt, TimerNode, TimerWorkflow,
};
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, Level};
use tracing_subscriber::fmt::format::FmtSpan;

#[derive(Debug, Clone)]
struct CounterContext {
    count: i32,
    message: String,
}

impl CounterContext {
    fn new() -> Self {
        Self {
            count: 0,
            message: String::new(),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_span_events(FmtSpan::CLOSE)
        .init();

    info!("Starting timer node examples");

    // Run examples
    run_simple_timer_example().await?;
    run_timer_workflow_example().await?;
    run_timer_in_standard_workflow_example().await?;
    run_different_schedule_types_example().await?;

    info!("All examples completed");
    Ok(())
}

// Example 1: Simple timer that executes once after a short delay
async fn run_simple_timer_example() -> Result<(), Box<dyn std::error::Error>> {
    info!("Running simple timer example");
    
    // Create a context
    let mut ctx = CounterContext::new();
    
    // Create a timer that will execute after 2 seconds
    let timer = SimpleTimer::new(
        Schedule::Once(Utc::now() + ChronoDuration::seconds(2)),
        |ctx: &mut CounterContext| {
            ctx.count += 1;
            ctx.message = "Simple timer executed".to_string();
            info!("Simple timer executed, context: {:?}", ctx);
            Ok(DefaultAction::Next)
        }
    );
    
    info!("Simple timer scheduled, waiting for execution...");
    
    // Execute the timer
    let action = TimerNode::execute_on_schedule(&timer, &mut ctx).await?;
    
    info!("Simple timer completed with action: {:?}", action);
    info!("Context after execution: {:?}", ctx);
    
    Ok(())
}

// Example 2: Timer workflow with multiple timers
async fn run_timer_workflow_example() -> Result<(), Box<dyn std::error::Error>> {
    info!("Running timer workflow example");
    
    // Create a context
    let mut ctx = CounterContext::new();
    
    // Create the first timer node
    let timer1 = Arc::new(SimpleTimer::with_id(
        "timer1",
        Schedule::Once(Utc::now() + ChronoDuration::seconds(1)),
        |ctx: &mut CounterContext| {
            ctx.count += 1;
            ctx.message = "First timer executed".to_string();
            info!("First timer executed, count: {}", ctx.count);
            Ok(DefaultAction::Next)
        }
    ));
    
    // Create the second timer node
    let timer2 = Arc::new(SimpleTimer::with_id(
        "timer2",
        Schedule::Once(Utc::now() + ChronoDuration::seconds(2)),
        |ctx: &mut CounterContext| {
            ctx.count += 2;
            ctx.message = format!("{} and second timer executed", ctx.message);
            info!("Second timer executed, count: {}", ctx.count);
            Ok(DefaultAction::Next)
        }
    ));
    
    // Create the third timer node with a termination action
    let timer3 = Arc::new(SimpleTimer::with_id(
        "timer3",
        Schedule::Once(Utc::now() + ChronoDuration::seconds(3)),
        |ctx: &mut CounterContext| {
            ctx.count += 3;
            ctx.message = format!("{} and third timer executed", ctx.message);
            info!("Third timer executed, count: {}", ctx.count);
            Ok(DefaultAction::Custom("terminate".to_string()))
        }
    ));
    
    // Create a timer workflow
    let mut workflow = TimerWorkflow::new(
        timer1.clone(),
        DefaultAction::Custom("terminate".to_string()),
    );
    
    // Add the other timer nodes
    workflow.add_node(timer2.clone());
    workflow.add_node(timer3.clone());
    
    // Set up the routing
    workflow.set_route(&TimerNode::id(&*timer1), DefaultAction::Next, &TimerNode::id(&*timer2));
    workflow.set_route(&TimerNode::id(&*timer2), DefaultAction::Next, &TimerNode::id(&*timer3));
    
    info!("Timer workflow created, starting execution...");
    
    // Execute the workflow
    workflow.execute(&mut ctx).await?;
    
    info!("Timer workflow completed");
    info!("Final context: {:?}", ctx);
    
    Ok(())
}

// Example 3: Using a timer node in a standard workflow
async fn run_timer_in_standard_workflow_example() -> Result<(), Box<dyn std::error::Error>> {
    info!("Running timer in standard workflow example");
    
    // Create a context
    let mut ctx = CounterContext::new();
    
    // Using a simpler approach with direct timer execution
    let timer = SimpleTimer::new(
        Schedule::Interval(Duration::from_secs(1)),
        |ctx: &mut CounterContext| {
            ctx.count += 1;
            ctx.message = format!("Timer executed {} times", ctx.count);
            info!("Timer executed, count: {}", ctx.count);
            
            // After 3 executions, return complete action
            if ctx.count >= 3 {
                Ok(DefaultAction::complete())
            } else {
                Ok(DefaultAction::retry())
            }
        }
    );
    
    info!("Timer created, starting execution...");
    
    // Execute the timer until it returns complete action
    let mut action = DefaultAction::retry();
    while action != DefaultAction::complete() {
        action = TimerNode::execute_on_schedule(&timer, &mut ctx).await?;
        info!("Timer executed with action: {:?}", action);
    }
    
    info!("Timer execution completed");
    info!("Final context: {:?}", ctx);
    
    Ok(())
}

// Example 4: Demonstrating different schedule types
async fn run_different_schedule_types_example() -> Result<(), Box<dyn std::error::Error>> {
    info!("Running different schedule types example");
    
    // Create a context
    let mut ctx = CounterContext::new();
    
    // Create a timer with an interval schedule (every 1 second)
    let interval_timer = SimpleTimer::with_id(
        "interval_timer",
        Schedule::Interval(Duration::from_secs(1)),
        |ctx: &mut CounterContext| {
            ctx.count += 1;
            info!("Interval timer executed, count: {}", ctx.count);
            
            // Run for 3 iterations
            if ctx.count >= 3 {
                ctx.message = "Interval timer completed".to_string();
                Ok(DefaultAction::Next)
            } else {
                Ok(DefaultAction::retry())
            }
        }
    );
    
    info!("Interval timer example: will execute every second for 3 iterations");
    
    // Execute the interval timer until it returns Success action
    let mut action = DefaultAction::retry();
    while action == DefaultAction::retry() {
        action = TimerNode::execute_on_schedule(&interval_timer, &mut ctx).await?;
    }
    
    info!("Interval timer completed with message: {}", ctx.message);
    
    // Reset the context
    ctx.count = 0;
    ctx.message = String::new();
    
    // Create a timer with a daily schedule (will execute at the next minute)
    let now = Utc::now();
    let next_minute = (now.minute() + 1) % 60;
    let next_hour = if next_minute == 0 { (now.hour() + 1) % 24 } else { now.hour() };
    
    let daily_timer = SimpleTimer::with_id(
        "daily_timer",
        Schedule::Daily(next_hour, next_minute),
        move |ctx: &mut CounterContext| {
            ctx.count += 1;
            ctx.message = "Daily timer executed".to_string();
            info!("Daily timer would execute at {}:{:02} every day", next_hour, next_minute);
            Ok(DefaultAction::Next)
        }
    );
    
    // For demonstration purposes, we won't actually wait for the daily timer
    // Instead, we'll just print the next execution time
    let next_execution = TimerNode::schedule(&daily_timer).next_execution()?;
    info!("Daily timer would next execute at: {}", next_execution);
    
    // Create a weekly timer for demonstration
    let weekly_timer = SimpleTimer::with_id(
        "weekly_timer",
        Schedule::Weekly(Weekday::Mon, 9, 0),
        |ctx: &mut CounterContext| {
            ctx.message = "Weekly timer executed".to_string();
            Ok(DefaultAction::Next)
        }
    );
    
    // Show when the weekly timer would execute next
    let next_execution = TimerNode::schedule(&weekly_timer).next_execution()?;
    info!("Weekly timer would next execute at: {} (Monday at 9:00 AM)", next_execution);
    
    // Create a monthly timer for demonstration
    let monthly_timer = SimpleTimer::with_id(
        "monthly_timer",
        Schedule::Monthly(1, 0, 0),
        |ctx: &mut CounterContext| {
            ctx.message = "Monthly timer executed".to_string();
            Ok(DefaultAction::Next)
        }
    );
    
    // Show when the monthly timer would execute next
    let next_execution = TimerNode::schedule(&monthly_timer).next_execution()?;
    info!("Monthly timer would next execute at: {} (1st day of month at midnight)", next_execution);
    
    Ok(())
} 