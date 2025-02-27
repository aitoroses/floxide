# ADR-0005: State Serialization and Deserialization

## Status

Accepted

## Date

2025-02-27

## Context

A workflow system often needs to persist its state, either for long-running processes, to support restart after failure, or to enable distributed execution. The floxide framework needs a well-defined approach to serialize and deserialize workflow state, particularly:

1. The workflow configuration (nodes and connections)
2. The execution state (current position, context data)
3. Partial execution results for resumability

The design of serialization and deserialization affects:

- Persistence capabilities
- Workflow restartability
- Distributed execution
- State migration during version upgrades
- Performance of checkpointing
- Integration with external storage systems

## Decision

We will implement a comprehensive serialization and deserialization strategy with the following components:

### 1. Context Serialization Trait

We'll define a trait that contexts must implement to be serializable:

```rust
/// Trait for contexts that can be serialized and deserialized
pub trait SerializableContext: Send + Sync + 'static {
    /// Serialize this context to bytes
    fn serialize(&self) -> Result<Vec<u8>, FloxideError>;

    /// Deserialize from bytes into a context
    fn deserialize(bytes: &[u8]) -> Result<Self, FloxideError>
    where
        Self: Sized;

    /// Get a serialization format identifier
    fn format_id(&self) -> &'static str {
        "json" // Default implementation
    }
}
```

### 2. Automatic Serde Implementation

For contexts that use standard types, we'll provide a derive macro:

```rust
#[derive(Serialize, Deserialize, SerializableContext)]
pub struct MyWorkflowContext {
    // Fields...
}
```

The macro will implement `SerializableContext` using serde_json by default:

```rust
impl SerializableContext for MyWorkflowContext {
    fn serialize(&self) -> Result<Vec<u8>, FloxideError> {
        serde_json::to_vec(self)
            .map_err(|e| FloxideError::SerializationError(e.to_string()))
    }

    fn deserialize(bytes: &[u8]) -> Result<Self, FloxideError> {
        serde_json::from_slice(bytes)
            .map_err(|e| FloxideError::DeserializationError(e.to_string()))
    }
}
```

### 3. Workflow State Snapshot

We'll define a structure to represent the complete workflow state:

```rust
pub struct WorkflowSnapshot<C, A = DefaultAction>
where
    C: SerializableContext,
    A: ActionType,
{
    /// The serialized context
    context_data: Vec<u8>,
    /// Format identifier for the serialized context
    context_format: String,
    /// Current position in the workflow
    current_node_id: NodeId,
    /// Metadata about the snapshot
    metadata: HashMap<String, String>,
    /// Version information to support migrations
    version: String,
    /// Timestamp when the snapshot was created
    created_at: DateTime<Utc>,
    /// Hash of the workflow graph at the time of snapshot
    workflow_hash: String,
}
```

### 4. Workflow Serialization and Deserialization

The `Workflow` struct will have methods for serialization and deserialization:

```rust
impl<C, A> Workflow<C, A>
where
    C: SerializableContext,
    A: ActionType,
{
    /// Create a snapshot of the current workflow state
    pub fn create_snapshot(&self, context: &C) -> Result<WorkflowSnapshot<C, A>, FloxideError> {
        // Implementation...
    }

    /// Resume execution from a snapshot
    pub async fn resume_from_snapshot(
        &self,
        snapshot: WorkflowSnapshot<C, A>
    ) -> Result<(), FloxideError> {
        // Implementation...
    }

    /// Save a snapshot to a storage location
    pub async fn save_snapshot(
        &self,
        context: &C,
        storage: &impl SnapshotStorage,
    ) -> Result<String, FloxideError> {
        let snapshot = self.create_snapshot(context)?;
        storage.store_snapshot(&snapshot).await
    }

    /// Load a snapshot from a storage location
    pub async fn load_and_resume(
        &self,
        snapshot_id: &str,
        storage: &impl SnapshotStorage,
    ) -> Result<(), FloxideError> {
        let snapshot = storage.load_snapshot::<C, A>(snapshot_id).await?;
        self.resume_from_snapshot(snapshot).await
    }
}
```

### 5. Storage Abstraction

We'll define a trait for snapshot storage backends:

```rust
#[async_trait]
pub trait SnapshotStorage: Send + Sync {
    /// Store a snapshot and return a unique identifier
    async fn store_snapshot<C, A>(
        &self,
        snapshot: &WorkflowSnapshot<C, A>,
    ) -> Result<String, FloxideError>
    where
        C: SerializableContext,
        A: ActionType;

    /// Load a snapshot by its identifier
    async fn load_snapshot<C, A>(
        &self,
        id: &str,
    ) -> Result<WorkflowSnapshot<C, A>, FloxideError>
    where
        C: SerializableContext,
        A: ActionType;

    /// List available snapshots
    async fn list_snapshots(&self) -> Result<Vec<SnapshotMetadata>, FloxideError>;

    /// Delete a snapshot
    async fn delete_snapshot(&self, id: &str) -> Result<(), FloxideError>;
}
```

### 6. Built-in Storage Implementations

We'll provide several built-in storage implementations:

```rust
/// File system storage for snapshots
pub struct FileSystemStorage {
    base_path: PathBuf,
}

/// In-memory storage for testing
pub struct InMemoryStorage {
    snapshots: RwLock<HashMap<String, Vec<u8>>>,
}

/// Redis storage implementation
#[cfg(feature = "redis-storage")]
pub struct RedisStorage {
    client: redis::Client,
    prefix: String,
}
```

### 7. Checkpointing Support

We'll add automatic checkpointing functionality:

```rust
/// Configuration for automatic checkpointing
pub struct CheckpointConfig {
    /// How often to create checkpoints (by node count or time)
    frequency: CheckpointFrequency,
    /// Storage backend to use
    storage: Box<dyn SnapshotStorage>,
    /// Whether to keep all checkpoints or only the latest
    keep_all: bool,
}

/// Extension to Workflow for checkpointing
impl<C, A> Workflow<C, A>
where
    C: SerializableContext,
    A: ActionType,
{
    /// Enable automatic checkpointing during execution
    pub fn with_checkpointing(
        mut self,
        config: CheckpointConfig
    ) -> Self {
        self.checkpoint_config = Some(config);
        self
    }
}
```

### 8. Workflow Graph Serialization

For serializing the workflow definition itself:

```rust
impl<C, A> Workflow<C, A>
where
    C: SerializableContext,
    A: ActionType + Serialize + DeserializeOwned,
{
    /// Serialize the workflow definition
    pub fn serialize_definition(&self) -> Result<Vec<u8>, FloxideError> {
        // Implementation...
    }

    /// Create a workflow from a serialized definition
    pub fn from_serialized_definition(data: &[u8]) -> Result<Self, FloxideError> {
        // Implementation...
    }
}
```

## Consequences

### Positive

1. **Persistence**: Workflows can be saved and resumed across process restarts
2. **Reliability**: Failed workflows can be restarted from the latest checkpoint
3. **Distributed Execution**: Workflow state can be transferred between different machines
4. **Flexibility**: Multiple storage backends allow for different deployment scenarios
5. **Versioning**: Support for migrations when workflow definitions change
6. **Transparency**: Clear semantics for what happens during serialization and deserialization
7. **Performance Options**: Users can choose serialization formats based on their needs
8. **Testability**: In-memory storage simplifies testing of persistence scenarios

### Negative

1. **Increased Complexity**: Adds complexity to the framework and API
2. **Serialization Constraints**: Contexts must be serializable, which may limit what they can contain
3. **Additional Dependencies**: May require additional crates like serde, uuid, chrono
4. **Performance Impact**: Frequent checkpointing could impact workflow execution performance
5. **Storage Management**: Users need to manage storage for snapshots (cleanup, etc.)
6. **Format Compatibility**: May require migration tools for format changes

## Alternatives Considered

### 1. Protocol Buffers / Cap'n Proto

- **Pros**:
  - Schema evolution
  - Compact binary format
  - Cross-language compatibility
- **Cons**:
  - More complex tooling
  - Less flexible for dynamic types
  - Additional build dependencies

### 2. Custom Binary Format

- **Pros**:
  - Could be optimized for workflow-specific needs
  - Potentially smaller size
- **Cons**:
  - Significant implementation effort
  - Limited ecosystem tools
  - Harder to debug serialized data

### 3. Full Workflow Serialization

- **Pros**:
  - Simpler conceptual model
  - Could serialize node implementations too
- **Cons**:
  - Much harder to implement correctly
  - Would require significant restrictions on node implementations
  - Less efficient for large workflows with little state

### 4. Event Sourcing Approach

- **Pros**:
  - Complete history of workflow execution
  - Ability to replay workflows
  - Better auditability
- **Cons**:
  - More complex implementation
  - Higher storage requirements
  - Potentially slower restoration for long workflows

We chose the snapshot-based approach with a storage abstraction because it provides a good balance between flexibility, performance, and implementation complexity. The ability to plug in different storage backends and serialization formats allows users to tailor the persistence behavior to their specific needs.
