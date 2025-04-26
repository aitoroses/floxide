# Floxide Core Concepts

This section covers the fundamental building blocks of the Floxide framework.

Understanding these concepts is essential for building robust and efficient workflows.

## Key Components

*   **Node**: The basic unit of work in a workflow. Represents a single step or task. (See [Node Concept](01_node.md))
*   **Workflow**: Defines the overall structure, connecting Nodes together in a directed graph. (See [Workflow Concept](02_workflow.md))
*   **Transition**: The outcome of a Node's execution, determining the next step. (See [Transition Concept](03_transition.md))
*   **Context (`WorkflowCtx`)**: Shared data or state accessible throughout a workflow run. (See [Context Concept](04_context.md))
*   **Macros (`node!`, `workflow!`)**: Declarative macros for defining Nodes and Workflows easily.

## Distributed Execution

While the concepts above form the basis of any Floxide workflow, the framework provides additional components specifically designed for running workflows across multiple processes or machines:

*   **`WorkQueue`**: A shared queue where pending tasks (`WorkItem`s) are placed. Independent `DistributedWorker` processes pick up tasks from this queue.
*   **`CheckpointStore`**: Stores the state (`Checkpoint`) of ongoing workflow runs, including the shared `Context` and the remaining tasks in the `WorkQueue`. This enables fault tolerance and recovery.
*   **`DistributedWorker`**: A separate process responsible for dequeuing `WorkItem`s, loading the corresponding `Checkpoint`, executing the `Node`, saving the updated `Checkpoint`, and enqueuing successor `WorkItem`s.
*   **`DistributedOrchestrator`**: Manages the overall lifecycle of distributed runs, providing an API to start, monitor, pause, resume, and cancel workflows. It often interacts with various **Distributed Stores** (like `RunInfoStore`, `MetricsStore`, `ErrorStore`) for observability.

These components work together to enable scalable and resilient workflow execution. You can learn more about them in the [Distributed Tutorial](../floxide-tutorial/index.md).

*Further details on each core concept will be added here.* 