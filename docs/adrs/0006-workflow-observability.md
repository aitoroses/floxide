# ADR-0006: Workflow Observability

## Status

Accepted

## Date

2024-02-26

## Context

Observability is crucial for workflow systems, especially for complex workflows or in production environments. Users need visibility into:

1. What is happening during workflow execution
2. Where a workflow is in its execution lifecycle
3. How long individual steps are taking
4. What errors occurred and why
5. The overall performance characteristics of workflows

A comprehensive observability system is essential for:

- Debugging workflow issues
- Monitoring production workflows
- Understanding performance bottlenecks
- Auditing workflow execution
- Visualizing workflow execution
- Alerting on workflow failures or delays

## Decision

We will implement an observability system for the flowrs framework with OpenTelemetry as the primary integration point, complemented by additional observability mechanisms.

### 1. OpenTelemetry as Primary Observability Solution

OpenTelemetry will be the main framework for observability:

```rust
/// Core OpenTelemetry integration for the flowrs framework
pub struct FlowrsOtel {
    tracer: opentelemetry::trace::Tracer,
    meter: opentelemetry::metrics::Meter,
    attributes: HashMap<String, String>,
}

impl FlowrsOtel {
    /// Create a new OpenTelemetry integration with the default configuration
    pub fn new(service_name: &str) -> Result<Self, FlowrsError> {
        // Set up OpenTelemetry with appropriate exporters
        let tracer_provider = opentelemetry_sdk::trace::TracerProvider::builder()
            .with_simple_processor(opentelemetry_sdk::trace::BatchSpanProcessor::new(
                opentelemetry_otlp::new_exporter()
                    .tonic()
                    .build()?,
            ))
            .build();

        let tracer = tracer_provider.tracer(service_name);

        let meter_provider = opentelemetry_sdk::metrics::MeterProvider::builder()
            .with_reader(
                opentelemetry_sdk::metrics::reader::Builder::default()
                    .with_exporter(
                        opentelemetry_otlp::new_exporter()
                            .tonic()
                            .build()?,
                    )
                    .build(),
            )
            .build();

        let meter = meter_provider.meter(service_name);

        Ok(Self {
            tracer,
            meter,
            attributes: HashMap::new(),
        })
    }

    /// Add a common attribute to all spans and metrics
    pub fn with_attribute(mut self, key: &str, value: &str) -> Self {
        self.attributes.insert(key.to_string(), value.to_string());
        self
    }
}
```

### 2. Event Emission System

The core of our observability design will be event emission that integrates with OpenTelemetry:

```rust
/// Events emitted during workflow execution
#[derive(Clone, Debug)]
pub enum WorkflowEvent<C, A = DefaultAction>
where
    C: 'static,
    A: ActionType,
{
    /// Workflow execution started
    WorkflowStarted {
        workflow_id: String,
        timestamp: DateTime<Utc>,
        metadata: HashMap<String, String>,
    },

    /// Workflow execution completed
    WorkflowCompleted {
        workflow_id: String,
        timestamp: DateTime<Utc>,
        execution_time_ms: u64,
        metadata: HashMap<String, String>,
    },

    /// Workflow execution failed
    WorkflowFailed {
        workflow_id: String,
        timestamp: DateTime<Utc>,
        error: FlowrsError,
        metadata: HashMap<String, String>,
    },

    /// Node processing started
    NodeStarted {
        workflow_id: String,
        node_id: NodeId,
        node_type: String,
        timestamp: DateTime<Utc>,
        metadata: HashMap<String, String>,
    },

    /// Node processing completed
    NodeCompleted {
        workflow_id: String,
        node_id: NodeId,
        node_type: String,
        timestamp: DateTime<Utc>,
        execution_time_ms: u64,
        outcome: NodeOutcomeType<A>,
        metadata: HashMap<String, String>,
    },

    /// Node processing failed
    NodeFailed {
        workflow_id: String,
        node_id: NodeId,
        node_type: String,
        timestamp: DateTime<Utc>,
        error: FlowrsError,
        metadata: HashMap<String, String>,
    },

    /// Transition between nodes
    Transition {
        workflow_id: String,
        from_node_id: NodeId,
        to_node_id: NodeId,
        action: A,
        timestamp: DateTime<Utc>,
        metadata: HashMap<String, String>,
    },

    /// Retry attempt occurred
    RetryAttempt {
        workflow_id: String,
        node_id: NodeId,
        attempt_number: usize,
        reason: String,
        timestamp: DateTime<Utc>,
        backoff_ms: u64,
        metadata: HashMap<String, String>,
    },

    /// Checkpoint created
    CheckpointCreated {
        workflow_id: String,
        checkpoint_id: String,
        timestamp: DateTime<Utc>,
        metadata: HashMap<String, String>,
    },

    /// Custom application event
    Custom {
        workflow_id: String,
        event_type: String,
        payload: Value,
        timestamp: DateTime<Utc>,
        metadata: HashMap<String, String>,
    },
}

/// Type of node outcome for events (without the actual output value)
#[derive(Clone, Debug)]
pub enum NodeOutcomeType<A>
where
    A: ActionType,
{
    /// Node completed
    Complete,
    /// Node transitioned with the specified action
    Transition(A),
}
```

### 3. OpenTelemetry Observer Implementation

The primary observer implementation will use OpenTelemetry:

```rust
/// Observer that emits events to OpenTelemetry
pub struct OpenTelemetryObserver {
    otel: FlowrsOtel,
}

impl OpenTelemetryObserver {
    /// Create a new OpenTelemetry observer
    pub fn new(otel: FlowrsOtel) -> Self {
        Self { otel }
    }
}

#[async_trait]
impl<C, A> WorkflowObserver<C, A> for OpenTelemetryObserver
where
    C: 'static,
    A: ActionType,
{
    async fn on_event(&self, event: WorkflowEvent<C, A>) -> Result<(), FlowrsError> {
        match &event {
            WorkflowEvent::WorkflowStarted { workflow_id, metadata, .. } => {
                let mut span = self.otel.tracer
                    .span_builder(format!("workflow:{}", workflow_id))
                    .with_kind(SpanKind::Internal)
                    .start(&self.otel.tracer);

                for (key, value) in metadata {
                    span.set_attribute(KeyValue::new(key.clone(), value.clone()));
                }

                // Set workflow ID as a global context for child spans
                let cx = Context::current_with_span(span);
                cx.attach();
            }

            // Handle other event types with appropriate spans and metrics
            // ...
        }

        Ok(())
    }
}
```

### 4. Additional Observer Implementations

We'll also provide complementary observers for specific use cases:

```rust
/// Observer that logs events using the tracing crate
pub struct TracingObserver {
    min_level: Level,
}

/// In-memory observer for testing or UI visualization
pub struct InMemoryObserver {
    events: RwLock<Vec<WorkflowEvent>>,
    max_events: usize,
}

/// Observer that publishes events to a channel
pub struct ChannelObserver<C, A = DefaultAction>
where
    C: 'static,
    A: ActionType,
{
    sender: mpsc::Sender<WorkflowEvent<C, A>>,
}
```

### 5. Observer Registry with OpenTelemetry Default

The workflow will have a registry of observers with OpenTelemetry as the default:

```rust
impl<C, A> Workflow<C, A>
where
    C: 'static,
    A: ActionType,
{
    /// Add OpenTelemetry observability to this workflow
    pub fn with_opentelemetry(mut self, service_name: &str) -> Result<Self, FlowrsError> {
        let otel = FlowrsOtel::new(service_name)?;
        self.observers.push(Box::new(OpenTelemetryObserver::new(otel)));
        Ok(self)
    }

    /// Add a custom observer to this workflow
    pub fn add_observer<O>(&mut self, observer: O) -> &mut Self
    where
        O: WorkflowObserver<C, A> + 'static,
    {
        self.observers.push(Box::new(observer));
        self
    }
}
```

### 6. Distributed Tracing with OpenTelemetry

We'll implement specific tracing support for workflow execution:

```rust
impl<C, A> Workflow<C, A>
where
    C: 'static,
    A: ActionType,
{
    async fn execute_with_tracing(&self, ctx: &mut C) -> Result<(), FlowrsError> {
        // Create a workflow execution span
        let tracer = opentelemetry::global::tracer("flowrs");
        let workflow_span = tracer.start(format!("workflow:{}", self.id()));
        let cx = Context::current_with_span(workflow_span);

        // Set the current context for propagation
        let _guard = cx.attach();

        let start = Instant::now();

        for observer in &self.observers {
            observer.on_start(self, ctx).await?;
        }

        let result = self.execute_internal(ctx).await;

        let execution_time = start.elapsed().as_millis() as u64;

        // Record metrics
        let meter = opentelemetry::global::meter("flowrs");
        let workflow_duration = meter
            .f64_histogram("workflow.duration")
            .with_description("Workflow execution duration in milliseconds")
            .init();

        workflow_duration.record(
            execution_time as f64,
            &[KeyValue::new("workflow_id", self.id().to_string())],
        );

        match &result {
            Ok(_) => {
                workflow_span.set_status(Status::Ok);
                for observer in &self.observers {
                    observer.on_complete(self, execution_time).await?;
                }
            }
            Err(err) => {
                workflow_span.set_status(Status::Error);
                workflow_span.record_error(err);
                for observer in &self.observers {
                    observer.on_failure(self, err).await?;
                }
            }
        }

        result
    }
}
```

### 7. OpenTelemetry-Based Visualization

We'll leverage OpenTelemetry for visualization capabilities:

```rust
/// Generate a visualization of workflow execution using OpenTelemetry spans
pub async fn generate_workflow_visualization(
    workflow_id: &str,
    trace_exporter_endpoint: &str,
) -> Result<String, FlowrsError> {
    // Query OpenTelemetry backend for the trace data
    // and generate visualization
    // ...
}
```

### 8. Context-Aware Logging with OpenTelemetry Integration

We'll enhance nodes with OpenTelemetry-integrated logging capabilities:

```rust
/// Extension trait for context to add observability capabilities
pub trait ObservableContext {
    /// Get a context identifier for observability
    fn context_id(&self) -> String;

    /// Get additional context attributes for OpenTelemetry
    fn otel_attributes(&self) -> HashMap<String, String> {
        HashMap::new()
    }
}

/// OpenTelemetry instrumentation for nodes
impl<Context, A> Node<Context, A>
for OtelNodeWrapper<N, Context, A>
where
    N: Node<Context, A>,
    Context: ObservableContext,
    A: ActionType,
{
    type Output = N::Output;

    async fn process(&self, ctx: &mut Context) -> Result<NodeOutcome<Self::Output, A>, FlowrsError> {
        let context_id = ctx.context_id();
        let attributes = ctx.otel_attributes();

        let tracer = opentelemetry::global::tracer("flowrs");
        let mut span_builder = tracer.span_builder(format!("node:{}", self.node_type));

        // Add attributes to the span
        for (key, value) in &attributes {
            span_builder = span_builder.with_attribute(KeyValue::new(key.clone(), value.clone()));
        }

        span_builder = span_builder.with_attribute(KeyValue::new("context_id", context_id.clone()));
        span_builder = span_builder.with_attribute(KeyValue::new("node_type", self.node_type.clone()));

        let span = span_builder.start(&tracer);
        let cx = Context::current_with_span(span);
        let _guard = cx.attach();

        let start = Instant::now();
        let result = self.inner.process(ctx).await;
        let duration = start.elapsed();

        // Record node execution metrics
        let meter = opentelemetry::global::meter("flowrs");
        let node_duration = meter
            .f64_histogram("node.duration")
            .with_description("Node execution duration in milliseconds")
            .init();

        node_duration.record(
            duration.as_millis() as f64,
            &[KeyValue::new("node_type", self.node_type.clone())],
        );

        match &result {
            Ok(outcome) => {
                let outcome_type = match outcome {
                    NodeOutcome::Complete(_) => "complete",
                    NodeOutcome::Transition(action, _) => {
                        span.set_attribute(KeyValue::new("transition_action", format!("{:?}", action)));
                        "transition"
                    }
                };

                span.set_attribute(KeyValue::new("outcome", outcome_type.to_string()));
                span.set_attribute(KeyValue::new("duration_ms", duration.as_millis() as i64));
                span.set_status(Status::Ok);
            }
            Err(err) => {
                span.set_attribute(KeyValue::new("duration_ms", duration.as_millis() as i64));
                span.set_status(Status::Error);
                span.record_error(err);
            }
        }

        result
    }
}
```

## Consequences

### Positive

1. **Complete Observability**: OpenTelemetry provides a comprehensive solution for traces, metrics, and logs
2. **Standard Integration**: Follows industry standards for observability
3. **Ecosystem Compatibility**: OpenTelemetry supports many backends (Jaeger, Prometheus, etc.)
4. **Distributed Tracing**: Built-in support for tracing across service boundaries
5. **Minimal Overhead**: OpenTelemetry is designed for production use with minimal impact
6. **Visualization Options**: Can leverage existing OpenTelemetry visualization tools
7. **Context Propagation**: Supports propagating context across async boundaries

### Negative

1. **Dependency Size**: OpenTelemetry adds significant dependencies to the project
2. **Configuration Complexity**: Properly configuring OpenTelemetry requires additional expertise
3. **Learning Curve**: Using OpenTelemetry effectively requires understanding its concepts
4. **Backend Requirements**: Requires setting up and maintaining OpenTelemetry backends
5. **Additional Resource Usage**: Collecting and exporting telemetry data uses system resources

## Alternatives Considered

### 1. Simple Logging Approach

- **Pros**:
  - Simpler implementation
  - Fewer dependencies
- **Cons**:
  - Limited visibility into workflow execution
  - No standardized way to analyze the data
  - No metrics collection

### 2. Custom Metrics Solution

- **Pros**:
  - Could be more tailored to workflow-specific needs
  - Potentially lower overhead for specific metrics
- **Cons**:
  - Would require maintaining a custom solution
  - No standardized integration with other systems
  - Limited ecosystem tools

### 3. Multiple Separate Systems (logs, metrics, traces)

- **Pros**:
  - Could choose best-of-breed for each concern
  - More flexibility in implementation
- **Cons**:
  - No unified observability model
  - More complex integration points
  - Harder to correlate data across systems

### 4. No Built-in Observability

- **Pros**:
  - Simpler framework
  - Lower dependency footprint
- **Cons**:
  - Users would need to implement their own solutions
  - Inconsistent observability implementations
  - Poor developer experience

We chose OpenTelemetry as our primary observability solution because it provides a comprehensive, standards-based approach to observability that covers traces, metrics, and logs. It offers excellent ecosystem compatibility while maintaining reasonable performance characteristics. The event-based observer pattern allows us to integrate OpenTelemetry seamlessly while still supporting additional observability mechanisms for specific use cases.
