// crates/floxide-macros/src/workflow.rs

use proc_macro::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::{
    bracketed,
    parse::{Parse, ParseStream},
    parse_macro_input, Ident, Result, Token,
    token::Bracket,
};
use heck::ToSnakeCase;

/// AST for `workflow! { name = X; start = [A, …]; edges { … } [acyclic;] }`
struct WorkflowDef {
    name: Ident,
    start: Vec<Ident>,
    edges: Vec<(Ident, Vec<Ident>)>,
    acyclic: bool,
}

impl Parse for WorkflowDef {
    fn parse(input: ParseStream) -> Result<Self> {
        // name = Foo;
        input.parse::<Ident>()?;        // `name`
        input.parse::<Token![=]>()?;
        let name: Ident = input.parse()?;
        input.parse::<Token![;]>()?;

        // start = X or start = [A, B, …]; allow single identifier without brackets
        input.parse::<Ident>()?;        // `start`
        input.parse::<Token![=]>()?;
        let start = if input.peek(Bracket) {
            let content;
            bracketed!(content in input);
            content.parse_terminated(Ident::parse, Token![,])?.into_iter().collect()
        } else {
            let single: Ident = input.parse()?;
            vec![single]
        };
        input.parse::<Token![;]>()?;

        // edges { A => [B, ...]; … }
        input.parse::<Ident>()?;        // `edges`
        let edges_content;
        syn::braced!(edges_content in input);
        let mut edges = Vec::new();
        while !edges_content.is_empty() {
            let from: Ident = edges_content.parse()?;
            edges_content.parse::<Token![=>]>()?;
            let succs_content;
            bracketed!(succs_content in edges_content);
            let succs = succs_content
                .parse_terminated(Ident::parse, Token![,])?
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
    let WorkflowDef { name, start, edges, acyclic } =
        parse_macro_input!(item as WorkflowDef);

    // ensure exactly one start node
    if start.len() != 1 {
        let msg = format!("`start` must specify exactly one node, found {}", start.len());
        return quote_spanned! { name.span() => compile_error!(#msg); }.into();
    }

    // Construct the name of the generated struct
    let wf_struct = format_ident!("{}Workflow", name);
    // Pick the first start as the single-entry point
    let start_ty = &start[0];

    // Optional compile‑time acyclic check
    let acyclic_check = if acyclic {
        // Build indegree map
        let mut indegree = std::collections::HashMap::new();
        for (n, succs) in &edges {
            indegree.entry(n.to_string()).or_insert(0);
            for s in succs {
                *indegree.entry(s.to_string()).or_insert(0) += 1;
            }
        }
        // Kahn's algorithm
        let mut queue: Vec<_> = indegree.iter()
            .filter(|&(_, &d)| d == 0)
            .map(|(k, _)| k.clone())
            .collect();
        let mut visited = 0;
        let mut local = indegree.clone();
        while let Some(n) = queue.pop() {
            visited += 1;
            if let Some((_, succs)) =
                edges.iter().find(|(from, _)| &from.to_string() == &n)
            {
                for s in succs {
                    if let Some(d) = local.get_mut(&s.to_string()) {
                        *d -= 1;
                        if *d == 0 { queue.push(s.to_string()); }
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

    // Generate dispatch arms for each node
    let dispatch_arms = edges.iter().map(|(from, succs)| {
        let from_ty = from;
        let succ_code = succs.iter().map(|to| {
            quote! {
                // transition to #to
                node_id = std::any::TypeId::of::<#to>();
                payload = Some(Box::new(out));
            }
        });
        quote! {
            if node_id == std::any::TypeId::of::<#from_ty>() {
                // downcast the payload to this node's Input
                let inp = match payload.take().unwrap().downcast::<<#from_ty as floxide_core::node::Node>::Input>() {
                    Ok(b) => *b,
                    Err(_) => panic!("invalid payload type"),
                };
                // call process
                let next = #from_ty {}.process(ctx, inp).await?;
                match next {
                    floxide_core::transition::Transition::Next(_, out) => {
                        #(#succ_code)*
                    }
                    floxide_core::transition::Transition::Finish => return Ok(()),
                    floxide_core::transition::Transition::Abort(e) => return Err(e),
                }
                continue;
            }
        }
    });

    // Generate compile-time type-equality checks for each edge: ensure From::Output == To::Input
    let type_checks = edges.iter().flat_map(|(from, succs)| {
        succs.iter().map(move |to| {
            // span on the successor node to highlight the failing target
            let span = to.span();
            // create a snake_case identifier for the edge assertion
            let check_ident_raw = format!("__assert_edge_{}_{}", from, to);
            // convert raw identifier string to snake_case
            let check_ident_snake = check_ident_raw.to_snake_case();
            let check_ident = format_ident!("{}", check_ident_snake);
            quote_spanned! { span =>
                #[allow(dead_code)]
                // enforce that <#from as Node>::Output == <#to as Node>::Input
                let #check_ident: fn(<#from as floxide_core::node::Node>::Output)
                                   -> <#to as floxide_core::node::Node>::Input = |x| x;
            }
        })
    });

    // The main run‑loop body
    let run_body = quote! {
        #acyclic_check

        // compile-time type checks: verify each edge's Output == Input
        #(#type_checks)*

        // initialize with start node
        let mut node_id = std::any::TypeId::of::<#start_ty>();
        // box the initial input
        let mut payload: Option<Box<dyn std::any::Any + Send>> = Some(Box::new(input));

        loop {
            #(#dispatch_arms)*

            // if we get here, no matching node was found
            return Err(floxide_core::error::FloxideError::Generic(
                format!("Unknown node id {:?}", node_id)
            ));
        }
    };

    // Emit the final code
    let expanded = quote! {
        use async_trait::async_trait;

        pub struct #wf_struct;

        #[async_trait]
        impl floxide_core::workflow::Workflow for #wf_struct {
            type Input = <#start_ty as floxide_core::node::Node>::Input;

            async fn run(
                &mut self,
                ctx: &mut floxide_core::context::WorkflowCtx<()>,
                input: Self::Input
            ) -> Result<(), floxide_core::error::FloxideError> {
                #run_body
            }
        }
    };

    TokenStream::from(expanded)
}
