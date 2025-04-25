/// Transition result of a node: Next->next node, Abort->error
/// Transition result of a node.
pub enum Transition<Output> {
    /// Emit a single output to successors.
    Next(Output),
    /// Emit multiple outputs to successors (split / fan-out).
    NextAll(Vec<Output>),
    /// Hold this work item; do not emit any outputs until a condition is met.
    Hold,
    /// Abort the workflow with an error.
    Abort(crate::error::FloxideError),
}