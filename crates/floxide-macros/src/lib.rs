// crates/floxide-macros/src/workflow.rs

use proc_macro::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::{
    bracketed,
    braced,
    parse::{Parse, ParseStream},
    parse_macro_input, Ident, Result, Token, Type,
};
use quote::ToTokens;
use syn::spanned::Spanned;

/// AST for `workflow! { name = X; start = Foo; edges { Foo => [Bar] } [acyclic;] }`
struct WorkflowDef {
    name: Ident,
    start: Type,
    edges: Vec<(Type, Vec<Type>)>,
    acyclic: bool,
}

impl Parse for WorkflowDef {
    fn parse(input: ParseStream) -> Result<Self> {
        // name = Foo;
        input.parse::<Ident>()?;        // `name`
        input.parse::<Token![=]>()?;
        let name: Ident = input.parse()?;
        input.parse::<Token![;]>()?;

        // start = Foo;
        input.parse::<Ident>()?;        // `start`
        input.parse::<Token![=]>()?;
        let start: Type = input.parse()?;
        input.parse::<Token![;]>()?;

        // edges { Foo => [Bar, Baz]; … }
        input.parse::<Ident>()?;        // `edges`
        let edges_content;
        braced!(edges_content in input);
        let mut edges = Vec::new();
        while !edges_content.is_empty() {
            let from: Type = edges_content.parse()?;
            edges_content.parse::<Token![=>]>()?;
            let succs_content;
            bracketed!(succs_content in edges_content);
            let succs: Vec<Type> = succs_content
                .parse_terminated(Type::parse, Token![,])?
                .into_iter()
                .collect();
            edges_content.parse::<Token![;]>()?;
            edges.push((from, succs));
        }

        // optional acyclic;
        let mut acyclic = false;
        if input.peek(Ident) && input.peek2(Token![;]) {
            let kw: Ident = input.parse()?;
            if kw == "acyclic" {
                acyclic = true;
                input.parse::<Token![;]>()?;
            }
        }

        Ok(WorkflowDef { name, start, edges, acyclic })
    }
}

#[proc_macro]
pub fn workflow(item: TokenStream) -> TokenStream {
    // parse the workflow definition
    let WorkflowDef { name, start, edges, acyclic } =
        parse_macro_input!(item as WorkflowDef);

    // compile-time acyclic detection (if requested)
    let acyclic_check = if acyclic {
        // build indegree map keyed by type tokens
        let mut indegree = std::collections::HashMap::new();
        for (from, succs) in &edges {
            let from_key = from.to_token_stream().to_string();
            indegree.entry(from_key.clone()).or_insert(0);
            for to in succs {
                let to_key = to.to_token_stream().to_string();
                *indegree.entry(to_key).or_insert(0) += 1;
            }
        }
        // Kahn's algorithm
        let mut queue: Vec<_> = indegree.iter()
            .filter(|&(_, &d)| d == 0)
            .map(|(k, _)| k.clone())
            .collect();
        let mut visited = 0;
        let mut local = indegree.clone();
        while let Some(node_str) = queue.pop() {
            visited += 1;
            if let Some((_, succs)) = edges.iter()
                .find(|(from, _)| from.to_token_stream().to_string() == node_str)
            {
                for to in succs {
                    let key = to.to_token_stream().to_string();
                    if let Some(count) = local.get_mut(&key) {
                        *count -= 1;
                        if *count == 0 { queue.push(key); }
                    }
                }
            }
        }
        if visited != indegree.len() {
            let msg = format!("workflow `{}` is not acyclic", name);
            quote! { compile_error!(#msg); }
        } else {
            quote! {}
        }
    } else {
        quote! {}
    };

    // compile-time type checks for each edge: <From as Node>::Output == <To as Node>::Input
    let type_checks = edges.iter().flat_map(|(from, succs)| {
        succs.iter().map(move |to| {
            let span = to.span();
            quote_spanned! { span =>
                #[allow(dead_code)]
                let _: fn(
                    <#from as floxide_core::node::Node>::Output
                ) -> 
                   <#to as floxide_core::node::Node>::Input
                   = |x| x;
            }
        })
    });

    // dispatch arms: for each (from, succs) enqueue successors into a worklist
    let dispatch_arms = edges.iter().map(|(from, succs)| {
        quote! {
            if node_id == std::any::TypeId::of::<#from>() {
                let inp = *payload
                    .downcast::< <#from as floxide_core::node::Node>::Input >()
                    .expect("invalid payload type");
                // instantiate the node and call its process method
                match (#from {}).process(ctx, inp).await? {
                    floxide_core::transition::Transition::Next(_, out) => {
                        #(
                            work.push_back((
                                std::any::TypeId::of::<#succs>(),
                                Box::new(out.clone()) as Box<dyn std::any::Any + Send>
                            ));
                        )*
                    },
                    floxide_core::transition::Transition::Finish => {},
                    floxide_core::transition::Transition::Abort(e) => return Err(e),
                }
                continue;
            }
        }
    });

    // assemble the run-body with a VecDeque worklist
    let run_body = quote! {
        use std::collections::VecDeque;
        #acyclic_check
        #(#type_checks)*
        let mut work = VecDeque::new();
        work.push_back((
            std::any::TypeId::of::<#start>(),
            Box::new(input) as Box<dyn std::any::Any + Send>,
        ));
        while let Some((node_id, payload)) = work.pop_front() {
            #(#dispatch_arms)*
            panic!("unknown node id {:?}", node_id);
        }
        Ok(())
    };

    // generate the struct and impl
    let wf_struct = format_ident!("{}Workflow", name);
    // Generate the workflow struct and implementation
    let expanded = quote! {
        pub struct #wf_struct;

        #[async_trait]
        impl floxide_core::workflow::Workflow for #wf_struct {
            type Input = <#start as floxide_core::node::Node>::Input;

            async fn run(
                &mut self,
                ctx: &mut floxide_core::context::WorkflowCtx<()>,
                input: Self::Input
            ) -> Result<(), floxide_core::error::FloxideError> {
                #run_body
            }
        }
    };

    // Return the generated code
    TokenStream::from(expanded)
}
