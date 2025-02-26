//! Example demonstrating the ReactiveNode functionality for reacting to changes
//! in external data sources.

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

// Helper function to convert IO errors to FloxideError
fn io_err_to_floxide(err: std::io::Error) -> FloxideError {
    FloxideError::Other(format!("IO error: {}", err))
}

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
    fn new(file_path: impl Into<String>) -> Self {
        Self {
            _file_path: file_path.into(),
            change_count: 0,
            latest_size: 0,
            changes: HashMap::new(),
        }
    }

    fn record_change(&mut self, change: FileChange) {
        self.change_count += 1;
        self.latest_size = change.size;

        let changes = self.changes.entry(change.path.clone()).or_default();
        changes.push(change);
    }
}

#[derive(Debug, Clone)]
struct MeasurementData {
    sensor_id: String,
    timestamp: u64,
    value: f64,
    unit: String,
}

#[derive(Debug, Clone)]
struct SensorContext {
    measurements: HashMap<String, Vec<MeasurementData>>,
    total_measurements: usize,
    latest_values: HashMap<String, f64>,
}

impl SensorContext {
    fn new() -> Self {
        Self {
            measurements: HashMap::new(),
            total_measurements: 0,
            latest_values: HashMap::new(),
        }
    }

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
