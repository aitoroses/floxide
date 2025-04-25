/// Transition result of a node: Next->next node, Abort->error
/// Transition result of a node.
pub enum Transition<Output> {
    /// Emit a single output to successors.
    Next(Output),
    /// Emit multiple outputs to successors (split / fan-out).
    NextAll(Vec<Output>),
    /// Abort the workflow with an error.
    Abort(crate::error::FloxideError),
}