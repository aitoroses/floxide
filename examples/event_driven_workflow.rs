// Temperature Monitoring System: An Event-Driven Workflow Example
//
// This example demonstrates how to build an event-driven workflow system
// using the Flow Framework. It implements a temperature monitoring system
// that processes events from multiple temperature sensors and triggers
// appropriate actions based on the temperature readings.
//
// Key concepts demonstrated:
// 1. Event-driven workflows for handling asynchronous events
// 2. Custom action types for specialized routing logic
// 3. Integration between event-driven and standard workflows
// 4. Stateful context management across multiple events
// 5. Timeout-based execution boundaries
// 6. Simulated event sources and processing
//
// The system architecture consists of:
// - Event Sources: Simulated temperature sensors generating random readings
// - Event Processor: A classifier node that categorizes temperature readings
// - Handler Nodes: Specialized nodes for different temperature classifications
// - Monitoring Context: Shared state tracking temperature history and alerts
//
// IMPORTANT ROUTING PATTERN:
// In event-driven workflows, it's crucial to understand the distinction between:
// 1. Event Sources - Nodes that generate events (implement wait_for_event)
// 2. Processors - Nodes that only process events but don't generate them
//
// The workflow execution always follows this pattern:
// 1. Get an event from an event source
// 2. Process the event with a processor node
// 3. Route back to an event source for the next event
//
// Processor nodes should NEVER be routed to directly without an event source
// in between, as they cannot generate new events.
//
// This example is designed in accordance with:
// - ADR-0009: Event-Driven Workflow Pattern
// - ADR-0017: Event-Driven Node Pattern
// - ADR-0020: Event-Driven Workflow Routing Guidelines

use async_trait::async_trait;
use flowrs_core::{ActionType, FlowrsError, Node, NodeId, NodeOutcome, Workflow};
use flowrs_event::{
    ChannelEventSource, EventActionExt, EventDrivenNode, EventDrivenWorkflow,
    NestedEventDrivenWorkflow,
};
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tokio::time::sleep;
use tracing::{info, warn};
use uuid::Uuid;

/// Temperature reading event from a sensor
///
/// This struct represents a single temperature reading event from a sensor.
/// It contains the sensor ID, temperature value, and timestamp.
#[derive(Debug, Clone)]
struct TemperatureEvent {
    /// Unique identifier for the sensor
    sensor_id: String,
    /// Temperature reading in Celsius
    temperature: f32,
    /// Unix timestamp when the reading was taken
    _timestamp: u64,
}

/// Custom action type for temperature monitoring workflow
///
/// This enum defines the possible actions that can be taken in response
/// to temperature events. It implements the ActionType trait to provide
/// string representations and the EventActionExt trait for termination
/// and timeout actions.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum TempAction {
    /// Normal temperature range, no action needed
    Normal,
    /// High temperature, activate cooling systems
    High,
    /// Low temperature, activate heating systems
    Low,
    /// Critical temperature, emergency shutdown
    _Critical,
    /// Workflow timeout occurred
    Timeout,
    /// Workflow completed successfully
    Complete,
}

impl Default for TempAction {
    fn default() -> Self {
        Self::Normal
    }
}

impl ActionType for TempAction {
    fn name(&self) -> &str {
        match self {
            Self::Normal => "normal",
            Self::High => "high",
            Self::Low => "low",
            Self::_Critical => "critical",
            Self::Timeout => "timeout",
            Self::Complete => "complete",
        }
    }
}

impl EventActionExt for TempAction {
    /// Defines which action should cause the workflow to terminate
    fn terminate() -> Self {
        Self::Complete
    }

    /// Defines which action should be used for timeout conditions
    fn timeout() -> Self {
        Self::Timeout
    }
}

/// Context for temperature monitoring
///
/// This struct maintains the state of the temperature monitoring system,
/// including temperature history, alerts, and average temperatures per sensor.
/// It provides methods for recording temperatures and generating alerts.
#[derive(Debug, Default, Clone)]
struct MonitoringContext {
    /// Temperature history for each sensor
    temperature_history: HashMap<String, Vec<f32>>,
    /// List of alerts generated during monitoring
    alerts: Vec<String>,
    /// Average temperature for each sensor
    average_temperatures: HashMap<String, f32>,
}

impl MonitoringContext {
    /// Creates a new empty monitoring context
    fn new() -> Self {
        Self {
            temperature_history: HashMap::new(),
            alerts: Vec::new(),
            average_temperatures: HashMap::new(),
        }
    }

    /// Records a temperature reading for a sensor and updates the average
    ///
    /// This method:
    /// 1. Adds the temperature to the sensor's history
    /// 2. Recalculates the average temperature for the sensor
    /// 3. Updates the average_temperatures map
    fn add_temperature(&mut self, sensor_id: &str, temp: f32) {
        let history = self
            .temperature_history
            .entry(sensor_id.to_string())
            .or_default();

        history.push(temp);

        // Update the average
        let avg = history.iter().sum::<f32>() / history.len() as f32;
        self.average_temperatures.insert(sensor_id.to_string(), avg);
    }

    /// Adds an alert message to the alerts list
    fn add_alert(&mut self, message: impl Into<String>) {
        self.alerts.push(message.into());
    }
}

/// Custom temperature threshold classifier node
///
/// This node is responsible for classifying temperature readings based on
/// configurable thresholds. It implements the EventDrivenNode trait to process
/// temperature events and return appropriate actions.
///
/// IMPORTANT: This is a processor node, not an event source. It should only
/// receive events from an event source and never be treated as a source itself.
/// The workflow should always route from this node back to an event source node.
struct TemperatureClassifier {
    /// Unique node identifier
    id: NodeId,
    /// Threshold for high temperature (in Celsius)
    high_threshold: f32,
    /// Threshold for low temperature (in Celsius)
    low_threshold: f32,
    /// Threshold for critical temperature (in Celsius)
    critical_threshold: f32,
}

impl TemperatureClassifier {
    /// Creates a new temperature classifier with the specified thresholds
    fn new(high_threshold: f32, low_threshold: f32, critical_threshold: f32) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            high_threshold,
            low_threshold,
            critical_threshold,
        }
    }
}

#[async_trait]
impl EventDrivenNode<TemperatureEvent, MonitoringContext, TempAction> for TemperatureClassifier {
    /// This node is not an event source, so wait_for_event always returns an error.
    /// In the event-driven workflow pattern, only proper event sources should be
    /// used to generate events. Processor nodes like this one should only be used
    /// to process events received from an event source.
    async fn wait_for_event(&mut self) -> Result<TemperatureEvent, FlowrsError> {
        // This is just a processor node, not an event source
        Err(FlowrsError::Other(
            "TemperatureClassifier is not an event source".to_string(),
        ))
    }

    /// Processes a temperature event and returns an appropriate action
    ///
    /// This method:
    /// 1. Records the temperature in the monitoring context
    /// 2. Compares the temperature against thresholds
    /// 3. Generates alerts for abnormal temperatures
    /// 4. Returns an action based on the temperature classification
    async fn process_event(
        &self,
        event: TemperatureEvent,
        ctx: &mut MonitoringContext,
    ) -> Result<TempAction, FlowrsError> {
        info!(
            sensor_id = %event.sensor_id,
            temperature = %event.temperature,
            "Classifying temperature"
        );

        // Record the temperature in the context
        ctx.add_temperature(&event.sensor_id, event.temperature);

        // Classify the temperature
        let action = if event.temperature >= self.critical_threshold {
            ctx.add_alert(format!(
                "CRITICAL TEMPERATURE ALERT: Sensor {} reported {}°C",
                event.sensor_id, event.temperature
            ));
            TempAction::Complete
        } else if event.temperature >= self.high_threshold {
            ctx.add_alert(format!(
                "High temperature warning: Sensor {} reported {}°C",
                event.sensor_id, event.temperature
            ));
            TempAction::High
        } else if event.temperature <= self.low_threshold {
            ctx.add_alert(format!(
                "Low temperature warning: Sensor {} reported {}°C",
                event.sensor_id, event.temperature
            ));
            TempAction::Low
        } else {
            TempAction::Normal
        };

        Ok(action)
    }

    fn id(&self) -> NodeId {
        self.id.clone()
    }
}

/// Node to handle normal temperature range
///
/// This node is activated when temperatures are within the normal range.
/// It simply logs the current state and continues processing.
struct NormalTempHandler {
    id: NodeId,
}

impl NormalTempHandler {
    fn new() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
        }
    }
}

#[async_trait]
impl Node<MonitoringContext, TempAction> for NormalTempHandler {
    type Output = ();

    fn id(&self) -> NodeId {
        self.id.clone()
    }

    async fn process(
        &self,
        ctx: &mut MonitoringContext,
    ) -> Result<NodeOutcome<Self::Output, TempAction>, FlowrsError> {
        info!("Processing normal temperature range");

        // For normal temperatures, we just log that everything is fine
        info!("All temperatures within normal range");
        info!("Current averages: {:?}", ctx.average_temperatures);

        Ok(NodeOutcome::Success(()))
    }
}

/// Node to handle high temperature alerts
///
/// This node is activated when temperatures exceed the high threshold.
/// It simulates activating cooling systems and logs all alerts.
struct HighTempHandler {
    id: NodeId,
}

impl HighTempHandler {
    fn new() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
        }
    }
}

#[async_trait]
impl Node<MonitoringContext, TempAction> for HighTempHandler {
    type Output = ();

    fn id(&self) -> NodeId {
        self.id.clone()
    }

    async fn process(
        &self,
        ctx: &mut MonitoringContext,
    ) -> Result<NodeOutcome<Self::Output, TempAction>, FlowrsError> {
        warn!("Processing high temperature alert");

        // Simulate activating cooling systems
        info!("Activating cooling systems for sensors with high temperatures");

        // Log all the alerts that have been generated
        for alert in &ctx.alerts {
            warn!("{}", alert);
        }

        Ok(NodeOutcome::Success(()))
    }
}

/// Node to handle low temperature alerts
///
/// This node is activated when temperatures fall below the low threshold.
/// It simulates activating heating systems and logs all alerts.
struct LowTempHandler {
    id: NodeId,
}

impl LowTempHandler {
    fn new() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
        }
    }
}

#[async_trait]
impl Node<MonitoringContext, TempAction> for LowTempHandler {
    type Output = ();

    fn id(&self) -> NodeId {
        self.id.clone()
    }

    async fn process(
        &self,
        ctx: &mut MonitoringContext,
    ) -> Result<NodeOutcome<Self::Output, TempAction>, FlowrsError> {
        warn!("Processing low temperature alert");

        // Simulate activating heating systems
        info!("Activating heating systems for sensors with low temperatures");

        // Log all the alerts that have been generated
        for alert in &ctx.alerts {
            warn!("{}", alert);
        }

        Ok(NodeOutcome::Success(()))
    }
}

/// Node to handle critical temperature alerts
///
/// This node is activated when temperatures exceed the critical threshold.
/// It simulates emergency procedures and terminates the workflow.
struct CriticalTempHandler {
    id: NodeId,
}

impl CriticalTempHandler {
    fn new() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
        }
    }
}

#[async_trait]
impl Node<MonitoringContext, TempAction> for CriticalTempHandler {
    type Output = ();

    fn id(&self) -> NodeId {
        self.id.clone()
    }

    async fn process(
        &self,
        ctx: &mut MonitoringContext,
    ) -> Result<NodeOutcome<Self::Output, TempAction>, FlowrsError> {
        warn!("Processing CRITICAL temperature alert");

        // Simulate emergency shutdown procedures
        info!("INITIATING EMERGENCY PROCEDURES");
        info!("Shutting down affected systems");
        info!("Notifying emergency response team");

        // Log all the alerts that have been generated
        for alert in &ctx.alerts {
            warn!("{}", alert);
        }

        // After handling a critical alert, we want to route to a complete action
        // to terminate the workflow
        Ok(NodeOutcome::RouteToAction(TempAction::Complete))
    }
}

/// Simulate temperature sensor events
///
/// This function simulates a temperature sensor by generating random temperature
/// readings and sending them to the provided channel. It takes parameters to
/// configure the behavior of the sensor.
///
/// # Parameters
///
/// * `sender` - The channel to send events to
/// * `sensor_id` - Unique identifier for this sensor
/// * `base_temp` - The baseline temperature this sensor fluctuates around
/// * `variance` - How much the temperature can vary from the baseline
/// * `interval_ms` - Time between readings in milliseconds
/// * `count` - Number of readings to generate
async fn simulate_temperature_events(
    sender: mpsc::Sender<TemperatureEvent>,
    sensor_id: String,
    base_temp: f32,
    variance: f32,
    interval_ms: u64,
    count: usize,
) {
    for i in 0..count {
        // Simulate some random temperature fluctuation
        let variance = if i % 5 == 0 {
            // Occasionally generate a more extreme reading
            variance * 3.0
        } else {
            variance
        };

        let temp_change = (rand::random::<f32>() * 2.0 - 1.0) * variance;
        let temperature = base_temp + temp_change;

        let event = TemperatureEvent {
            sensor_id: sensor_id.clone(),
            temperature,
            _timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        info!(
            sensor_id = %event.sensor_id,
            temperature = %event.temperature,
            "Sending temperature event"
        );

        if sender.send(event).await.is_err() {
            warn!(
                "Channel closed, stopping temperature simulation for sensor {}",
                sensor_id
            );
            break;
        }

        sleep(Duration::from_millis(interval_ms)).await;
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for better logging
    tracing_subscriber::fmt::init();

    info!("Starting temperature monitoring system example");

    // Create our monitoring context
    let mut ctx = MonitoringContext::new();

    // Create an event processor for temperature events
    // The classifier has thresholds at:
    // - High: 30°C
    // - Low: 10°C
    // - Critical: 40°C
    let classifier = TemperatureClassifier::new(30.0, 10.0, 40.0);
    let classifier_id = classifier.id();
    let classifier = Arc::new(Mutex::new(classifier));

    // Create the event source with a buffer capacity of 100 events
    let (source, sender) = ChannelEventSource::<TemperatureEvent>::new(100);
    let source_id = <ChannelEventSource<TemperatureEvent> as EventDrivenNode<
        TemperatureEvent,
        MonitoringContext,
        TempAction,
    >>::id(&source)
    .clone();
    let source = Arc::new(Mutex::new(source));

    // Create the event-driven workflow with a termination action of Complete
    let mut workflow = EventDrivenWorkflow::new(source.clone(), TempAction::Complete);

    // Add the classifier node to the workflow
    workflow.add_node(classifier.clone());

    // Configure routing from the source to the classifier
    workflow.set_route(&source_id, TempAction::default(), &classifier_id);

    // Add routes from classifier back to the source for non-terminating actions
    workflow.set_route(&classifier_id, TempAction::Normal, &source_id);
    workflow.set_route(&classifier_id, TempAction::High, &source_id);
    workflow.set_route(&classifier_id, TempAction::Low, &source_id);
    // The TempAction::Complete action will naturally terminate the workflow
    // No explicit route needed for TempAction::Complete as it's the workflow's termination action

    // Log the routing configuration for clarity
    info!("Event-driven workflow routing configuration:");
    info!("  Source ({}) -> Classifier ({})", source_id, classifier_id);
    info!("  Classifier: Normal -> Source");
    info!("  Classifier: High -> Source");
    info!("  Classifier: Low -> Source");
    info!("  Classifier: Complete -> [Workflow Termination]");

    // Create a standard workflow for handling different temperature ranges
    // We'll use our normal handler as the initial node
    let normal_handler = NormalTempHandler::new();
    let normal_handler_id = normal_handler.id().clone();

    let mut temp_handler_workflow = Workflow::new(normal_handler);

    // Add nodes for different temperature classifications
    let high_handler = HighTempHandler::new();
    let high_handler_id = high_handler.id().clone();
    temp_handler_workflow.add_node(high_handler);

    let low_handler = LowTempHandler::new();
    let low_handler_id = low_handler.id().clone();
    temp_handler_workflow.add_node(low_handler);

    let critical_handler = CriticalTempHandler::new();
    let critical_handler_id = critical_handler.id().clone();
    temp_handler_workflow.add_node(critical_handler);

    // We'll manually set up routing for the handler nodes
    // Since the Workflow API doesn't have add_route method, we'll simulate it
    // In a real application, you would implement a proper routing system

    // For demonstration purposes only, we'll just log what would happen
    info!("Setting up temperature handling routes:");
    info!(
        "  - Normal temperatures -> Normal handler ({})",
        normal_handler_id
    );
    info!(
        "  - High temperatures -> High handler ({})",
        high_handler_id
    );
    info!("  - Low temperatures -> Low handler ({})", low_handler_id);
    info!(
        "  - Critical temperatures -> Critical handler ({})",
        critical_handler_id
    );

    // Create a nested event-driven workflow adapter to use in the standard workflow
    let workflow = Arc::new(workflow);
    let _nested_workflow =
        NestedEventDrivenWorkflow::new(workflow.clone(), TempAction::Complete, TempAction::Timeout);

    // Create three simulated temperature sensors with different characteristics
    let sensor1 = tokio::spawn(simulate_temperature_events(
        sender.clone(),
        "Sensor-001".to_string(),
        25.0, // Normal temperature around 25°C
        5.0,  // With variance of ±5°C
        2000, // Send an event every 2 seconds
        20,   // Send 20 events total
    ));

    let sensor2 = tokio::spawn(simulate_temperature_events(
        sender.clone(),
        "Sensor-002".to_string(),
        15.0, // Cooler area, around 15°C
        3.0,  // Less variance
        3000, // Send an event every 3 seconds
        15,   // Send 15 events total
    ));

    let sensor3 = tokio::spawn(simulate_temperature_events(
        sender.clone(),
        "Sensor-003".to_string(),
        35.0, // Hotter area, around 35°C
        8.0,  // More variance
        2500, // Send an event every 2.5 seconds
        18,   // Send 18 events total
    ));

    // Execute the event-driven workflow with a timeout of 60 seconds
    let result = workflow
        .execute_with_timeout(&mut ctx, Duration::from_secs(15))
        .await;

    // Wait for all sensors to finish
    let _ = tokio::join!(sensor1, sensor2, sensor3);

    // Handle the workflow result
    match result {
        Ok(()) => {
            info!("Temperature monitoring workflow completed successfully");
        }
        Err(e) => {
            warn!("Temperature monitoring workflow ended with error: {}", e);
        }
    }

    // Display final monitoring statistics
    info!("Final monitoring statistics:");
    info!("Total alerts: {}", ctx.alerts.len());
    info!("Average temperatures by sensor:");
    for (sensor, avg) in &ctx.average_temperatures {
        info!("  {}: {:.1}°C", sensor, avg);
    }

    Ok(())
}
