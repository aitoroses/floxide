//! # Floxide Timer
//!
//! Timer-based node extensions for the Floxide framework.
//!
//! This crate provides time-based workflow capabilities through
//! the TimerNode trait and various time-based schedule implementations.

use async_trait::async_trait;
use chrono::{DateTime, Datelike, Duration as ChronoDuration, Timelike, Utc, Weekday};
use floxide_core::{error::FloxideError, ActionType, DefaultAction, Node, NodeId, NodeOutcome};
use std::collections::HashMap;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, warn};
use uuid::Uuid;

/// Represents a time schedule for execution
#[derive(Debug, Clone)]
pub enum Schedule {
    /// Execute once at a specific time
    Once(DateTime<Utc>),

    /// Execute repeatedly at fixed intervals
    Interval(Duration),

    /// Execute daily at a specified hour and minute (24-hour format)
    Daily(u32, u32),

    /// Execute weekly on a specified day at a specified hour and minute
    Weekly(Weekday, u32, u32),

    /// Execute monthly on a specified day at a specified hour and minute
    Monthly(u32, u32, u32),

    /// Execute according to a cron expression (not fully implemented)
    /// This is a placeholder for future implementation
    Cron(String),
}

impl Schedule {
    /// Calculate the next execution time based on the current time
    pub fn next_execution(&self) -> Result<DateTime<Utc>, FloxideError> {
        let now = Utc::now();

        match self {
            Schedule::Once(time) => {
                if time <= &now {
                    Err(FloxideError::Other(
                        "Scheduled time has already passed".to_string(),
                    ))
                } else {
                    Ok(*time)
                }
            }

            Schedule::Interval(duration) => Ok(now
                + ChronoDuration::from_std(*duration).map_err(|e| {
                    FloxideError::Other(format!("Failed to convert duration: {}", e))
                })?),

            Schedule::Daily(hour, minute) => {
                let mut next = now;

                // Set the next execution time to today at the specified hour and minute
                next = next
                    .with_hour(*hour)
                    .and_then(|dt| dt.with_minute(*minute))
                    .and_then(|dt| dt.with_second(0))
                    .and_then(|dt| dt.with_nanosecond(0))
                    .ok_or_else(|| FloxideError::Other("Invalid hour or minute".to_string()))?;

                // If this time has already passed today, schedule it for tomorrow
                if next <= now {
                    next += ChronoDuration::days(1);
                }

                Ok(next)
            }

            Schedule::Weekly(weekday, hour, minute) => {
                let mut next = now;

                // Set the next execution time to the specified hour and minute
                next = next
                    .with_hour(*hour)
                    .and_then(|dt| dt.with_minute(*minute))
                    .and_then(|dt| dt.with_second(0))
                    .and_then(|dt| dt.with_nanosecond(0))
                    .ok_or_else(|| FloxideError::Other("Invalid hour or minute".to_string()))?;

                // Calculate days until the next occurrence of the specified weekday
                let days_until_weekday =
                    (*weekday as i32 - now.weekday().num_days_from_monday() as i32 + 7) % 7;

                // If the time has already passed today and it's the specified weekday, schedule it for next week
                if days_until_weekday == 0 && next <= now {
                    next += ChronoDuration::days(7);
                } else {
                    next += ChronoDuration::days(days_until_weekday as i64);
                }

                Ok(next)
            }

            Schedule::Monthly(day, hour, minute) => {
                let mut next = now;

                // Set the next execution time to the specified hour and minute
                next = next
                    .with_hour(*hour)
                    .and_then(|dt| dt.with_minute(*minute))
                    .and_then(|dt| dt.with_second(0))
                    .and_then(|dt| dt.with_nanosecond(0))
                    .ok_or_else(|| FloxideError::Other("Invalid hour or minute".to_string()))?;

                // Set the day of the month
                let current_day = now.day();

                // If the specified day is valid for the current month
                if *day <= 31 {
                    // Try to set the day
                    match next.with_day(*day) {
                        Some(date) => next = date,
                        None => {
                            return Err(FloxideError::Other(format!(
                                "Invalid day {} for the current month",
                                day
                            )))
                        }
                    }

                    // If this time has already passed this month, or the day is earlier than today,
                    // schedule it for next month
                    if next <= now || *day < current_day {
                        // Move to the 1st of next month and then try to set the day
                        next += ChronoDuration::days(32); // Move well into next month
                        next = next.with_day(1).ok_or_else(|| {
                            FloxideError::Other("Failed to set day to 1".to_string())
                        })?;

                        // Try to set the specified day in the next month
                        next = next.with_day(*day).ok_or_else(|| {
                            FloxideError::Other(format!("Invalid day {} for next month", day))
                        })?;
                    }
                } else {
                    return Err(FloxideError::Other(format!(
                        "Invalid day of month: {}",
                        day
                    )));
                }

                Ok(next)
            }

            Schedule::Cron(_expression) => {
                // Placeholder for cron implementation
                // For now, just return an error
                Err(FloxideError::Other(
                    "Cron expressions are not yet implemented".to_string(),
                ))
            }
        }
    }

    /// Calculate the duration until the next execution
    pub fn duration_until_next(&self) -> Result<Duration, FloxideError> {
        let next = self.next_execution()?;
        let now = Utc::now();

        let duration = next.signed_duration_since(now);
        if duration.num_milliseconds() <= 0 {
            return Err(FloxideError::Other(
                "Scheduled time is in the past".to_string(),
            ));
        }

        Ok(Duration::from_millis(duration.num_milliseconds() as u64))
    }
}

/// A node that executes based on time schedules
#[async_trait]
pub trait TimerNode<Context, Action>: Send + Sync
where
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static + Default + Debug,
{
    /// Define the execution schedule
    fn schedule(&self) -> Schedule;

    /// Execute the node on schedule
    async fn execute_on_schedule(&self, ctx: &mut Context) -> Result<Action, FloxideError>;

    /// Get the node's unique identifier
    fn id(&self) -> NodeId;
}

/// A simple timer that executes a function based on a schedule
pub struct SimpleTimer<F>
where
    F: Send + Sync + 'static,
{
    id: NodeId,
    schedule: Schedule,
    action: F,
}

impl<F> SimpleTimer<F>
where
    F: Send + Sync + 'static,
{
    /// Create a new simple timer with a default ID
    pub fn new(schedule: Schedule, action: F) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            schedule,
            action,
        }
    }

    /// Create a new simple timer with a specific ID
    pub fn with_id(id: impl Into<String>, schedule: Schedule, action: F) -> Self {
        Self {
            id: id.into(),
            schedule,
            action,
        }
    }
}

#[async_trait]
impl<Context, Action, F> TimerNode<Context, Action> for SimpleTimer<F>
where
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static + Default + Debug,
    F: Fn(&mut Context) -> Result<Action, FloxideError> + Send + Sync + 'static,
{
    fn schedule(&self) -> Schedule {
        self.schedule.clone()
    }

    async fn execute_on_schedule(&self, ctx: &mut Context) -> Result<Action, FloxideError> {
        (self.action)(ctx)
    }

    fn id(&self) -> NodeId {
        self.id.clone()
    }
}

/// A timer workflow that orchestrates execution of timer nodes
pub struct TimerWorkflow<Context, Action>
where
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static + Default + Debug,
{
    nodes: HashMap<NodeId, Arc<dyn TimerNode<Context, Action>>>,
    routes: HashMap<(NodeId, Action), NodeId>,
    initial_node: NodeId,
    termination_action: Action,
}

impl<Context, Action> TimerWorkflow<Context, Action>
where
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static + Default + Debug,
{
    /// Create a new timer workflow with an initial node
    pub fn new(
        initial_node: Arc<dyn TimerNode<Context, Action>>,
        termination_action: Action,
    ) -> Self {
        let id = initial_node.id();

        let mut nodes = HashMap::new();
        nodes.insert(id.clone(), initial_node);

        Self {
            nodes,
            routes: HashMap::new(),
            initial_node: id,
            termination_action,
        }
    }

    /// Add a node to the workflow
    pub fn add_node(&mut self, node: Arc<dyn TimerNode<Context, Action>>) {
        let id = node.id();
        self.nodes.insert(id, node);
    }

    /// Set a route from one node to another based on an action
    pub fn set_route(&mut self, from_id: &NodeId, action: Action, to_id: &NodeId) {
        self.routes.insert((from_id.clone(), action), to_id.clone());
    }

    /// Execute the workflow until completion or error
    pub async fn execute(&self, ctx: &mut Context) -> Result<(), FloxideError> {
        let mut current_node_id = self.initial_node.clone();

        loop {
            let node = self.nodes.get(&current_node_id).ok_or_else(|| {
                FloxideError::Other(format!("Node not found: {}", current_node_id))
            })?;

            // Calculate the time until the next execution
            let wait_duration = match node.schedule().duration_until_next() {
                Ok(duration) => duration,
                Err(e) => {
                    warn!(
                        "Failed to calculate next execution time for node {}: {}",
                        current_node_id, e
                    );
                    // Default to a short interval if we can't calculate the next time
                    Duration::from_secs(5)
                }
            };

            // Wait until it's time to execute
            debug!(
                "Waiting {:?} until next execution of node {}",
                wait_duration, current_node_id
            );
            sleep(wait_duration).await;

            // Execute the node
            let action = match node.execute_on_schedule(ctx).await {
                Ok(action) => action,
                Err(e) => {
                    warn!("Error executing node {}: {}", current_node_id, e);
                    // Continue with the next node if there's an error
                    Action::default()
                }
            };

            // If the action is the termination action, stop the workflow
            if action == self.termination_action {
                debug!("Workflow terminated by node {}", current_node_id);
                break;
            }

            // Find the next node based on the action
            if let Some(next_node_id) = self.routes.get(&(current_node_id.clone(), action.clone()))
            {
                debug!(
                    "Moving from node {} to node {}",
                    current_node_id, next_node_id
                );
                current_node_id = next_node_id.clone();
            } else {
                // If there's no route for this action, use the default action
                if let Some(next_node_id) = self
                    .routes
                    .get(&(current_node_id.clone(), Action::default()))
                {
                    debug!(
                        "No route found for action {:?}, using default route to node {}",
                        action, next_node_id
                    );
                    current_node_id = next_node_id.clone();
                } else {
                    // If there's no default route either, stop the workflow
                    warn!(
                        "No route found for node {} with action {:?} and no default route",
                        current_node_id, action
                    );
                    break;
                }
            }
        }

        Ok(())
    }
}

/// An adapter to use a timer node as a standard node
pub struct TimerNodeAdapter<Context, Action>
where
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static + Default + Debug,
{
    node: Arc<dyn TimerNode<Context, Action>>,
    id: NodeId,
    execute_immediately: bool,
}

impl<Context, Action> TimerNodeAdapter<Context, Action>
where
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static + Default + Debug,
{
    /// Create a new timer node adapter
    pub fn new(node: Arc<dyn TimerNode<Context, Action>>, execute_immediately: bool) -> Self {
        let id = node.id();
        Self {
            node,
            id,
            execute_immediately,
        }
    }

    /// Create a new timer node adapter with a specific ID
    pub fn with_id(
        node: Arc<dyn TimerNode<Context, Action>>,
        id: impl Into<String>,
        execute_immediately: bool,
    ) -> Self {
        Self {
            node,
            id: id.into(),
            execute_immediately,
        }
    }
}

#[async_trait]
impl<Context, Action> Node<Context, Action> for TimerNodeAdapter<Context, Action>
where
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static + Default + Debug,
{
    type Output = ();

    fn id(&self) -> NodeId {
        self.id.clone()
    }

    async fn process(
        &self,
        ctx: &mut Context,
    ) -> Result<NodeOutcome<Self::Output, Action>, FloxideError> {
        if self.execute_immediately {
            // Execute the node immediately
            let action = self.node.execute_on_schedule(ctx).await?;
            Ok(NodeOutcome::RouteToAction(action))
        } else {
            // Wait until the scheduled time
            let wait_duration = self.node.schedule().duration_until_next()?;
            debug!(
                "Waiting {:?} before executing node {}",
                wait_duration, self.id
            );
            sleep(wait_duration).await;

            // Execute the node
            let action = self.node.execute_on_schedule(ctx).await?;
            Ok(NodeOutcome::RouteToAction(action))
        }
    }
}

/// A nested timer workflow that can be used as a standard node
pub struct NestedTimerWorkflow<Context, Action>
where
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static + Default + Debug,
{
    workflow: Arc<TimerWorkflow<Context, Action>>,
    id: NodeId,
    complete_action: Action,
    _phantom: PhantomData<(Context, Action)>,
}

impl<Context, Action> NestedTimerWorkflow<Context, Action>
where
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static + Default + Debug,
{
    /// Create a new nested timer workflow
    pub fn new(workflow: Arc<TimerWorkflow<Context, Action>>, complete_action: Action) -> Self {
        Self {
            workflow,
            id: Uuid::new_v4().to_string(),
            complete_action,
            _phantom: PhantomData,
        }
    }

    /// Create a new nested timer workflow with a specific ID
    pub fn with_id(
        workflow: Arc<TimerWorkflow<Context, Action>>,
        id: impl Into<String>,
        complete_action: Action,
    ) -> Self {
        Self {
            workflow,
            id: id.into(),
            complete_action,
            _phantom: PhantomData,
        }
    }
}

#[async_trait]
impl<Context, Action> Node<Context, Action> for NestedTimerWorkflow<Context, Action>
where
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static + Default + Debug,
{
    type Output = ();

    fn id(&self) -> NodeId {
        self.id.clone()
    }

    async fn process(
        &self,
        ctx: &mut Context,
    ) -> Result<NodeOutcome<Self::Output, Action>, FloxideError> {
        // Execute the timer workflow
        let result = self.workflow.execute(ctx).await;

        match result {
            Ok(_) => Ok(NodeOutcome::RouteToAction(self.complete_action.clone())),
            Err(e) => Err(e),
        }
    }
}

/// Extension trait for ActionType to provide timer-specific actions
pub trait TimerActionExt: ActionType {
    /// Create a complete action for timer nodes
    fn complete() -> Self;

    /// Create a retry action for timer nodes
    fn retry() -> Self;
}

impl TimerActionExt for DefaultAction {
    fn complete() -> Self {
        DefaultAction::Custom("timer_complete".to_string())
    }

    fn retry() -> Self {
        DefaultAction::Custom("timer_retry".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test the Schedule::next_execution method
    #[tokio::test]
    async fn test_schedule_next_execution() {
        // Test Once schedule
        let future_time = Utc::now() + ChronoDuration::hours(1);
        let once_schedule = Schedule::Once(future_time);
        let next = once_schedule.next_execution().unwrap();
        assert_eq!(next, future_time);

        // Test Interval schedule
        let interval_schedule = Schedule::Interval(Duration::from_secs(60));
        let next = interval_schedule.next_execution().unwrap();
        let diff = (next - Utc::now()).num_seconds();
        assert!(diff > 0 && diff <= 61); // Allow a small margin for execution time

        // Test Daily schedule (this is a simple test; more complex tests would verify exact times)
        let now = Utc::now();
        let future_hour = (now.hour() + 1) % 24;
        let daily_schedule = Schedule::Daily(future_hour, 0);
        let next = daily_schedule.next_execution().unwrap();
        assert!(next > now);
        assert_eq!(next.hour(), future_hour);
        assert_eq!(next.minute(), 0);
    }

    // Test a simple timer node
    #[tokio::test]
    async fn test_simple_timer() {
        // Create a context
        let mut ctx = "test_context".to_string();

        // Create a simple timer that executes immediately
        let timer = SimpleTimer::new(
            Schedule::Once(Utc::now() + ChronoDuration::milliseconds(100)),
            |ctx: &mut String| {
                *ctx = format!("{}_executed", ctx);
                Ok(DefaultAction::Next)
            },
        );

        // Execute the timer
        let action = timer.execute_on_schedule(&mut ctx).await.unwrap();

        // Verify the results
        assert_eq!(action, DefaultAction::Next);
        assert_eq!(ctx, "test_context_executed");
    }

    // Test a timer workflow
    #[tokio::test]
    async fn test_timer_workflow() {
        // Create a context
        let mut ctx = 0;

        // Create timer nodes
        let timer1 = Arc::new(SimpleTimer::with_id(
            "timer1",
            Schedule::Once(Utc::now() + ChronoDuration::milliseconds(100)),
            |ctx: &mut i32| {
                *ctx += 1;
                Ok(DefaultAction::Next)
            },
        ));

        let timer2 = Arc::new(SimpleTimer::with_id(
            "timer2",
            Schedule::Once(Utc::now() + ChronoDuration::milliseconds(200)),
            |ctx: &mut i32| {
                *ctx += 2;
                Ok(DefaultAction::Custom("terminate".to_string()))
            },
        ));

        // Create a workflow
        let mut workflow = TimerWorkflow::new(
            timer1.clone(),
            DefaultAction::Custom("terminate".to_string()),
        );

        workflow.add_node(timer2.clone());
        workflow.set_route(&timer1.id(), DefaultAction::Next, &timer2.id());

        // Execute the workflow with a timeout to prevent the test from hanging
        let handle = tokio::spawn(async move {
            workflow.execute(&mut ctx).await.unwrap();
            ctx
        });

        // Wait for the workflow to complete or timeout
        let result = tokio::time::timeout(Duration::from_secs(1), handle)
            .await
            .unwrap()
            .unwrap();

        // Verify the results
        assert_eq!(result, 3); // 1 from timer1 + 2 from timer2
    }
}
