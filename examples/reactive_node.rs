// Reactive Node Pattern: Event-Driven Data Processing Example
//
// This example demonstrates how to implement and use the Reactive Node pattern
// in the Flow Framework. The Reactive Node pattern allows nodes to react to
// external events and data changes, making it ideal for:
//
// Key concepts demonstrated:
// 1. Reactive processing based on external events
// 2. File system monitoring and change detection
// 3. Sensor data processing with real-time updates
// 4. Custom reactive node implementations
// 5. Integration with event streams and external data sources
//
// The example implements two scenarios:
// - File watcher: A node that monitors file system changes and reacts to modifications
// - Sensor data processor: A node that processes streaming sensor measurements
//
// Reactive nodes are particularly useful for:
// - Monitoring external resources (files, databases, APIs)
// - Processing real-time data streams
// - Building event-driven architectures
// - Creating responsive systems that adapt to changing conditions
//
// This example is designed in accordance with:
// - ADR-0014: Reactive Node Pattern
// - ADR-0019: External Event Integration Strategy

use floxide_core::{DefaultAction, FloxideError};
use floxide_reactive::{
    CustomReactiveNode, FileChange, FileWatcherNode, ReactiveActionExt, ReactiveNode,
    ReactiveNodeAdapter,
};
use futures::{Stream, StreamExt};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::sleep;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{info, Level};

/// Helper function to convert IO errors to FloxideError
///
/// This utility function provides a consistent way to convert standard
/// IO errors into the framework's error type for better error handling.
fn io_err_to_floxide(err: std::io::Error) -> FloxideError {
    FloxideError::Other(format!("IO error: {}", err))
}

/// Context for file watching operations
///
/// This struct maintains the state of file monitoring operations,
/// including change counts, file sizes, and a history of detected changes.
#[derive(Debug, Clone)]
struct FileWatchContext {
    // Using underscore prefix to indicate intentionally unused field
    // but kept for documentation purposes
    _file_path: String,
    change_count: usize,
    latest_size: u64,
    changes: HashMap<String, Vec<FileChange>>,
}

impl FileWatchContext {
    /// Creates a new file watch context for the specified file path
    ///
    /// Initializes an empty context that will track changes to the given file.
    fn new(file_path: impl Into<String>) -> Self {
        Self {
            _file_path: file_path.into(),
            change_count: 0,
            latest_size: 0,
            changes: HashMap::new(),
        }
    }

    /// Records a file change event in the context
    ///
    /// Updates the change count, latest file size, and adds the change
    /// to the history of changes for the specific file path.
    fn record_change(&mut self, change: FileChange) {
        self.change_count += 1;
        self.latest_size = change.size;

        let changes = self.changes.entry(change.path.clone()).or_default();
        changes.push(change);
    }
}

/// Measurement data structure for sensor readings
///
/// Represents a single measurement from a sensor, including the sensor ID,
/// timestamp, value, and unit of measurement.
#[derive(Debug, Clone)]
struct MeasurementData {
    sensor_id: String,
    timestamp: u64,
    value: f64,
    unit: String,
}

/// Context for sensor data processing operations
///
/// This struct maintains the state of sensor data processing operations,
/// including the history of measurements, total measurement count, and
/// the latest values for each sensor.
#[derive(Debug, Clone)]
struct SensorContext {
    measurements: HashMap<String, Vec<MeasurementData>>,
    total_measurements: usize,
    latest_values: HashMap<String, f64>,
}

impl SensorContext {
    /// Creates a new sensor context for processing measurements
    ///
    /// Initializes an empty context that will track sensor measurements.
    fn new() -> Self {
        Self {
            measurements: HashMap::new(),
            total_measurements: 0,
            latest_values: HashMap::new(),
        }
    }

    /// Records a measurement in the context
    ///
    /// Updates the total measurement count, latest values, and adds the
    /// measurement to the history of measurements for the specific sensor.
    fn record_measurement(&mut self, data: MeasurementData) {
        self.total_measurements += 1;
        self.latest_values
            .insert(data.sensor_id.clone(), data.value);

        let measurements = self.measurements.entry(data.sensor_id.clone()).or_default();
        measurements.push(data);
    }
}

#[tokio::main]
async fn main() -> Result<(), FloxideError> {
    // Initialize logging
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    // Run examples
    info!("Running file watcher example...");
    run_file_watcher_example().await?;

    info!("Running custom sensor data example...");
    run_sensor_data_example().await?;

    Ok(())
}

/// Runs the file watcher example
///
/// Demonstrates how to use the Reactive Node pattern to monitor file system
/// changes and react to modifications.
async fn run_file_watcher_example() -> Result<(), FloxideError> {
    // Create a temporary file for demonstration
    let temp_file_path = "temp_watch_file.txt";
    {
        let mut file = File::create(temp_file_path).map_err(io_err_to_floxide)?;
        file.write_all(b"Initial content")
            .map_err(io_err_to_floxide)?;
    }
    info!("Created temporary file at {}", temp_file_path);

    // Create a file watcher node with a short polling interval and a change handler
    // The change handler addresses the lifetime issue
    let file_watcher = FileWatcherNode::<FileWatchContext, DefaultAction>::new(temp_file_path)
        .with_poll_interval(Duration::from_millis(200))
        .with_change_handler(|change: FileChange, ctx: &mut FileWatchContext| {
            // Use a static function instead of capturing ctx to avoid lifetime issues
            let path = change.path.clone();

            // Record the change in the context before moving into the async block
            ctx.record_change(change);

            async move {
                // Log the change
                info!("Change detected for file: {}", path);

                // Return a "change_detected" action
                Ok(DefaultAction::change_detected())
            }
        });

    // Create a context to track file changes
    let ctx = FileWatchContext::new(temp_file_path);

    // Create a reactive node adapter
    let reactive_adapter = ReactiveNodeAdapter::new(file_watcher).with_buffer_size(5);

    // Start watching for changes in a separate task
    let adapter_clone = Arc::new(reactive_adapter);

    let watcher_task = tokio::spawn(async move {
        let mut change_count = 0;

        // Create a stream of actions from the adapter
        if let Ok(mut action_stream) = adapter_clone.start_watching(ctx).await {
            // Process actions from the stream
            while let Some(action) = action_stream.next().await {
                if action.is_change_detected() {
                    change_count += 1;
                    info!("Change detected: {}", change_count);

                    // Stop after 3 changes
                    if change_count >= 3 {
                        break;
                    }
                }
            }
        }
    });

    // Modify the file a few times
    for i in 1..=3 {
        // Wait a bit
        sleep(Duration::from_millis(500)).await;

        // Modify the file
        let content = format!("Updated content {}", i);
        let mut file = File::create(temp_file_path).map_err(io_err_to_floxide)?;
        file.write_all(content.as_bytes())
            .map_err(io_err_to_floxide)?;
        info!("Modified file: {}", content);
    }

    // Wait for the watcher task to complete
    let _ = tokio::time::timeout(Duration::from_secs(5), watcher_task).await;

    // Clean up
    if Path::new(temp_file_path).exists() {
        std::fs::remove_file(temp_file_path).map_err(io_err_to_floxide)?;
        info!("Removed temporary file");
    }

    Ok(())
}

/// Runs the custom sensor data example
///
/// Demonstrates how to use the Reactive Node pattern to process streaming
/// sensor measurements and react to changes.
async fn run_sensor_data_example() -> Result<(), FloxideError> {
    // Create a custom reactive node for sensor data
    let sensor_node = CustomReactiveNode::<_, _, _, _, _>::new(
        || {
            // Create a simulated stream of sensor data
            let (tx, rx) = mpsc::channel(10);

            // Generate sensor data in the background
            tokio::spawn(async move {
                let sensors = vec!["temp-1", "humid-1", "pressure-1"];
                let mut timestamp = 1000;

                for i in 0..10 {
                    for &sensor in &sensors {
                        // Create some simulated measurements
                        let data = MeasurementData {
                            sensor_id: sensor.to_string(),
                            timestamp,
                            value: 20.0 + (i as f64) + rand::random::<f64>(),
                            unit: if sensor.starts_with("temp") {
                                "°C".to_string()
                            } else if sensor.starts_with("humid") {
                                "%".to_string()
                            } else {
                                "hPa".to_string()
                            },
                        };

                        // Send the data
                        if tx.send(data).await.is_err() {
                            break;
                        }
                    }

                    timestamp += 1;
                    sleep(Duration::from_millis(200)).await;
                }
            });

            Ok(Box::new(ReceiverStream::new(rx))
                as Box<dyn Stream<Item = MeasurementData> + Send + Unpin>)
        },
        |data: MeasurementData, ctx: &mut SensorContext| {
            // Record the measurement
            info!(
                "Received measurement: {} = {} {} (timestamp: {})",
                data.sensor_id, data.value, data.unit, data.timestamp
            );

            ctx.record_measurement(data.clone());

            // Return action based on the measurement
            if data.sensor_id.starts_with("temp") && data.value > 25.0 {
                info!("Temperature alert: {} {}", data.value, data.unit);
                Ok(DefaultAction::Custom("temperature_alert".to_string()))
            } else {
                Ok(DefaultAction::change_detected())
            }
        },
    );

    // Clone the context for the reactive adapter
    let mut sensor_ctx = SensorContext::new();

    // Process some sensor data directly
    let mut sensor_stream = sensor_node.watch().await?;

    // Process 5 measurements
    for _ in 0..5 {
        if let Some(data) = sensor_stream.next().await {
            let action = sensor_node.react_to_change(data, &mut sensor_ctx).await?;
            info!("Processed measurement, action: {:?}", action);
        }
    }

    // Show summary
    info!(
        "Recorded {} measurements from {} sensors",
        sensor_ctx.total_measurements,
        sensor_ctx.measurements.len()
    );

    for (sensor_id, value) in &sensor_ctx.latest_values {
        info!("Latest value for {}: {}", sensor_id, value);
    }

    Ok(())
}
