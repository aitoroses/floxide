/// Transition result of a node: Next->next node, Abort->error
pub enum Transition<Output> {
    Next(Output),
    Abort(crate::error::FloxideError),
} 