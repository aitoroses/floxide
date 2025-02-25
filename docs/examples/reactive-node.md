# Reactive Node Example

This example demonstrates how to use reactive nodes in the Flowrs framework for building event-driven, reactive workflows.

## Overview

Reactive nodes enable:
- Stream processing
- Event-driven workflows
- Real-time data processing
- Backpressure handling

## Implementation

Let's create a reactive workflow that processes a stream of temperature readings:

```rust
use flowrs_core::{ActionType, DefaultAction, FlowrsError, NodeId};
use flowrs_reactive::{CustomReactiveNode, ReactiveNode};
use futures::stream::{self, Stream};
use std::time::{Duration, Instant};
use tokio::time;

// Define our data types
#[derive(Debug, Clone)]
struct Temperature {
    celsius: f64,
    timestamp: Instant,
}

#[derive(Debug, Clone)]
struct Alert {
    message: String,
    level: AlertLevel,
    timestamp: Instant,
}

#[derive(Debug, Clone)]
enum AlertLevel {
    Info,
    Warning,
    Critical,
}

// Define our context
#[derive(Debug, Clone)]
struct MonitoringContext {
    temperatures: Vec<Temperature>,
    alerts: Vec<Alert>,
}

// Create a temperature monitoring node
fn create_temperature_monitor() -> impl ReactiveNode<Temperature, MonitoringContext, DefaultAction> {
    CustomReactiveNode::new(
        // Watch function that creates a stream of temperature readings
        || {
            let stream = stream::unfold(0, |state| async move {
                time::sleep(Duration::from_secs(1)).await;
                let temp = Temperature {
                    celsius: 20.0 + (state as f64 / 10.0),
                    timestamp: Instant::now(),
                };
                Some((temp, state + 1))
            });
            Ok(Box::new(stream))
        },
        // React function that processes each temperature reading
        |temp: Temperature, ctx: &mut MonitoringContext| {
            ctx.temperatures.push(temp.clone());
            
            let alert = match temp.celsius {
                t if t > 25.0 => Some(Alert {
                    message: format!("Critical temperature: {:.1}°C", t),
                    level: AlertLevel::Critical,
                    timestamp: temp.timestamp,
                }),
                t if t > 22.0 => Some(Alert {
                    message: format!("Warning temperature: {:.1}°C", t),
                    level: AlertLevel::Warning,
                    timestamp: temp.timestamp,
                }),
                _ => None,
            };

            if let Some(alert) = alert {
                ctx.alerts.push(alert);
                Ok(DefaultAction::change_detected())
            } else {
                Ok(DefaultAction::no_change())
            }
        },
    )
}

// Example usage
#[tokio::main]
async fn main() -> Result<(), FlowrsError> {
    let mut ctx = MonitoringContext {
        temperatures: Vec::new(),
        alerts: Vec::new(),
    };

    let monitor = create_temperature_monitor();
    
    // Set up the watch stream with backpressure handling
    let mut stream = monitor.watch().await?;
    
    // Process temperature readings with proper error handling
    while let Some(temp) = stream.next().await {
        match monitor.react_to_change(temp, &mut ctx).await {
            Ok(action) if action.is_change_detected() => {
                println!("Alert triggered! Total alerts: {}", ctx.alerts.len());
            }
            Ok(_) => {
                println!("Temperature normal: {:.1}°C", ctx.temperatures.last().unwrap().celsius);
            }
            Err(e) => {
                eprintln!("Error processing temperature: {}", e);
                // Implement your error recovery strategy here
            }
        }
    }

    Ok(())
}
```

## Running the Example

Here's how to run the reactive nodes:

```rust
#[tokio::main]
async fn main() -> Result<(), FlowrsError> {
    // Create the reactive stream
    let (temperature_stream, temperature_sender) = ReactiveStream::new();

    // Create the nodes
    let monitor = create_temperature_monitor();
    let processor = create_alert_processor();

    // Create the workflow
    let workflow = temperature_stream
        .through(monitor)
        .through(processor)
        .for_each(|message| println!("{}", message));

    // Send some test data
    temperature_sender.send(Temperature {
        celsius: 24.0,
        timestamp: Instant::now(),
    }).await;

    temperature_sender.send(Temperature {
        celsius: 26.0,
        timestamp: Instant::now(),
    }).await;

    temperature_sender.send(Temperature {
        celsius: 31.0,
        timestamp: Instant::now(),
    }).await;
}
```

## Advanced Usage

### Backpressure Handling

```rust
// Create a node with backpressure handling
fn create_throttled_monitor() -> impl ReactiveNode<Temperature, Alert> {
    reactive_node(|temp: Temperature| async move {
        // ... processing logic ...
    })
    .with_buffer(100) // Buffer up to 100 items
    .with_backpressure(BackpressureStrategy::DropOldest)
}
```

### Error Handling

```rust
// Create a node with error recovery
fn create_robust_monitor() -> impl ReactiveNode<Temperature, Alert> {
    reactive_node(|temp: Temperature| async move {
        if temp.celsius < -273.15 {
            return Err(FlowrsError::new("Invalid temperature"));
        }
        // ... normal processing ...
    })
    .with_retry(3)
    .with_error_handler(|e| {
        println!("Error processing temperature: {}", e);
        None // Skip invalid temperatures
    })
}
```

### Combining Streams

```rust
// Combine multiple temperature streams
let combined = ReactiveStream::combine(
    vec![temp_stream1, temp_stream2, temp_stream3],
    |temps: Vec<Temperature>| {
        // Process multiple temperature readings
        let avg = temps.iter().map(|t| t.celsius).sum::<f64>() / temps.len() as f64;
        Temperature {
            celsius: avg,
            timestamp: Instant::now(),
        }
    }
);
```

## Best Practices

1. **Stream Management**
   - Handle backpressure appropriately
   - Clean up resources when streams are dropped
   - Use appropriate buffer sizes

2. **Error Handling**
   - Implement proper error recovery
   - Handle stream completion
   - Log errors for debugging

3. **Testing**
   - Test stream processing logic
   - Verify backpressure handling
   - Test error scenarios

## See Also

- [Reactive Node Implementation ADR](../adrs/0017-reactive-node-implementation.md)
- [Event-Driven Architecture](../guides/event_driven_architecture.md)
- [Node Lifecycle Methods](../architecture/node-lifecycle-methods.md)
