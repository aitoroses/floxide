/// Resultado de un nodo: Next->siguiente nodo, Finish->fin exitoso, Abort->error  
pub enum Transition<N: crate::node::Node> {
    Next(N, N::Output),
    Finish,
    Abort(crate::error::FloxideError),
} 