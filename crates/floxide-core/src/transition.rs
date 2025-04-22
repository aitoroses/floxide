/// Resultado de un nodo: Next->siguiente nodo, Finish->fin exitoso, Abort->error  
pub enum Transition<Output> {
    Next(Output),
    Finish,
    Abort(crate::error::FloxideError),
} 