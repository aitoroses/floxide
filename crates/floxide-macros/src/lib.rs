use proc_macro::TokenStream;

mod merge;
mod node;
mod workflow;

/// Define a Workflow with fields and edges in one macro invocation.
///
/// Syntax:
/// workflow! {
///   pub struct Name { field1: Type1, field2: Type2, }
///   context = MyCtx;
///   start = field1;
///   edges {
///     field1 => [field2];
///     field2 => [];
///   };
/// }
#[proc_macro]
pub fn workflow(item: TokenStream) -> TokenStream {
    workflow::workflow(item)
}

/// Define a Node with fields and a process body in one macro invocation.
///
/// Syntax:
/// node! {
///   pub struct Name { field1: Type1, field2: Type2, }
///   context = MyCtx;
///   input   = InputType;
///   output  = OutputType;
///   |ctx, input_val| { /* can access self, returns Result<Transition<OutputType>, FloxideError> */ }
/// }
#[proc_macro]
pub fn node(item: TokenStream) -> TokenStream {
    node::node(item)
}
/// Derive implementation for the `Merge` trait.
///
/// Automatically implements `Merge` by merging each field of a struct using its own `merge` method.
#[proc_macro_derive(Merge)]
pub fn derive_merge(item: TokenStream) -> TokenStream {
    merge::derive(item)
}
