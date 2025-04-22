// crates/floxide-macros/src/workflow.rs

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    bracketed, braced,
    parse::{Parse, ParseStream},
    parse_macro_input, Ident, Result, Token,
    Visibility, Generics, Type
};

/// AST for struct-based workflow: struct fields, start field, and per-node edges
// Internal representation of a composite edge arm: matches Output enum variant
struct CompositeArm {
    action_path: Ident,
    variant: Ident,
    binding: Ident,
    succs: Vec<Ident>,
}
// AST for struct-based workflow: struct fields, start field, and routing edges
enum EdgeKind {
    Direct(Vec<Ident>),
    Composite(Vec<CompositeArm>),
}
struct WorkflowDef {
    vis: Visibility,
    name: Ident,
    generics: Generics,
    fields: Vec<(Ident, Type)>,
    start: Ident,
    // for each source field, direct successors or composite arms
    edges: Vec<(Ident, EdgeKind)>,
}

impl Parse for WorkflowDef {
    fn parse(input: ParseStream) -> Result<Self> {
        // parse optional visibility
        let vis: Visibility = if input.peek(Token![pub]) {
            input.parse()?  // pub or pub(...) etc
        } else {
            Visibility::Inherited
        };
        // parse struct definition
        input.parse::<Token![struct]>()?;
        let name: Ident = input.parse()?;
        let generics: Generics = input.parse()?;
        // parse struct fields
        let content;
        braced!(content in input);
        let mut fields = Vec::new();
        while !content.is_empty() {
            let fld: Ident = content.parse()?;
            content.parse::<Token![:]>()?;
            let ty: Type = content.parse()?;
            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            }
            fields.push((fld, ty));
        }
        // parse start = field;
        input.parse::<Ident>()?; // start
        input.parse::<Token![=]>()?;
        let start: Ident = input.parse()?;
        input.parse::<Token![;]>()?;
        // parse edges { ... }
        input.parse::<Ident>()?; // edges
        let edges_content;
        braced!(edges_content in input);
        let mut edges = Vec::new();
        while !edges_content.is_empty() {
            let src: Ident = edges_content.parse()?;
            edges_content.parse::<Token![=>]>()?;
            // parse either direct successors or composite arms
            let nested;
            braced!(nested in edges_content);
            let edge_kind = if nested.peek(syn::token::Bracket) {
                // direct successors: [ succ1, succ2, ... ]
                let succs_content;
                bracketed!(succs_content in nested);
                let succs: Vec<Ident> = succs_content
                    .parse_terminated(Ident::parse, Token![,])?
                    .into_iter()
                    .collect();
                EdgeKind::Direct(succs)
            } else {
                // composite arms: Enum::Variant(binding) => [succs]; ...
                let mut composite = Vec::new();
                while !nested.is_empty() {
                    let action_path: Ident = nested.parse()?;
                    nested.parse::<Token![::]>()?;
                    let variant: Ident = nested.parse()?;
                    let inner;
                    syn::parenthesized!(inner in nested);
                    let binding: Ident = inner.parse()?;
                    nested.parse::<Token![=>]>()?;
                    let succs_content;
                    bracketed!(succs_content in nested);
                    let succs: Vec<Ident> = succs_content
                        .parse_terminated(Ident::parse, Token![,])?
                        .into_iter()
                        .collect();
                    nested.parse::<Token![;]>()?;
                    composite.push(CompositeArm { action_path, variant, binding, succs });
                }
                EdgeKind::Composite(composite)
            };
            // consume semicolon after edge block
            edges_content.parse::<Token![;]>()?;
            edges.push((src, edge_kind));
        }
        Ok(WorkflowDef { vis, name, generics, fields, start, edges })
    }
}

#[proc_macro]
pub fn workflow(item: TokenStream) -> TokenStream {
    // parse the struct-based workflow definition
    let WorkflowDef { vis, name, generics, fields, start, edges } =
        parse_macro_input!(item as WorkflowDef);

    // Build WorkItem enum name
    let work_item_ident = format_ident!("{}WorkItem", name);
    // Determine terminal branch: a field with no successors (direct empty or composite empty)
    let terminal_src = edges.iter().find(|(_, kind)| {
        matches!(kind, EdgeKind::Direct(succs) if succs.is_empty())
        || matches!(kind, EdgeKind::Composite(ar) if ar.is_empty())
    }).expect("Workflow must have a terminal branch").0.clone();
    // Find the type of the terminal field
    let terminal_ty = fields.iter()
        .find(|(fld, _)| fld == &terminal_src)
        .map(|(_, ty)| ty.clone())
        .expect("Terminal field not found among fields");
    // Generate WorkItem variants
    let work_variants = fields.iter().map(|(fld, ty)| {
        // Variant name: capitalized field name
        let fld_str = fld.to_string();
        let mut chars = fld_str.chars();
        let first = chars.next().unwrap().to_uppercase().to_string();
        let rest: String = chars.collect();
        let var_ident = format_ident!("{}{}", first, rest);
        quote! { #var_ident(<#ty as floxide_core::node::Node>::Input) }
    });

    // Struct definition
    let struct_def = {
        let field_defs = fields.iter().map(|(fld, ty)| quote! { pub #fld: #ty });
        quote! { #vis struct #name #generics { #(#field_defs),* } }
    };

    // Generate run method arms for each field
    let run_arms = fields.iter().map(|(fld, _ty)| {
        // WorkItem variant
        let fld_str = fld.to_string();
        let mut chars = fld_str.chars();
        let first = chars.next().unwrap().to_uppercase().to_string();
        let rest: String = chars.collect();
        let var_ident = format_ident!("{}{}", first, rest);
        // Find edges for this field
        let kind = edges.iter().find(|(src, _)| src == fld)
            .map(|(_, k)| k)
            .unwrap_or_else(|| panic!("No edges defined for field {}", fld));
        // Generate code based on edge kind
        let arm_tokens = match kind {
            EdgeKind::Direct(succs) => {
                if succs.is_empty() {
                    // terminal direct branch: return the output value
                    quote! {
                        // access only the store for this node
                        let __store = &ctx.store;
                        match ctx.run_future(self.#fld.process(__store, x)).await? {
                            Transition::Next(action) => return Ok(action),
                            Transition::Finish => return Ok(Default::default()),
                            Transition::Abort(e) => return Err(e),
                        }
                    }
                } else {
                    // direct successor(s): pass the action to next node(s)
                    let pushes = succs.iter().map(|succ| {
                        let succ_str = succ.to_string();
                        let mut sc = succ_str.chars();
                        let sf = sc.next().unwrap().to_uppercase().to_string();
                        let rest: String = sc.collect();
                        let succ_var = format_ident!("{}{}", sf, rest);
                        quote! { work.push_back(#work_item_ident::#succ_var(action.clone())); }
                    });
                    quote! {
                        // access only the store for this node
                        let __store = &ctx.store;
                        match ctx.run_future(self.#fld.process(__store, x)).await? {
                            Transition::Next(action) => { #(#pushes)* }
                            Transition::Finish => {},
                            Transition::Abort(e) => return Err(e),
                        }
                    }
                }
            }
            EdgeKind::Composite(composite) => {
                if composite.is_empty() {
                    // terminal composite branch: return the output value
                quote! {
                        // access only the store for this node
                        let __store = &ctx.store;
                        match ctx.run_future(self.#fld.process(__store, x)).await? {
                            Transition::Next(action) => return Ok(action),
                            Transition::Finish => return Ok(Default::default()),
                            Transition::Abort(e) => return Err(e),
                        }
                    }
                } else {
                    // composite edges: pattern-based
                    let pats = composite.iter().map(|arm| {
                        let CompositeArm { action_path, variant, binding, succs } = arm;
                        let pat = quote! { #action_path :: #variant (#binding) };
                        // for composite arms: if no successors, return output; else push successors
                        let succ_pushes = succs.iter().map(|succ| {
                            let succ_str = succ.to_string();
                            let mut sc = succ_str.chars();
                            let sf = sc.next().unwrap().to_uppercase().to_string();
                            let rest: String = sc.collect();
                            let succ_var = format_ident!("{}{}", sf, rest);
                            quote! { work.push_back(#work_item_ident::#succ_var(#binding)); }
                        });
                        if succs.is_empty() {
                            quote! { #pat => { return Ok(#binding); } }
                        } else {
                            quote! { #pat => { #(#succ_pushes)* } }
                        }
                    });
                    quote! {
                        // access only the store for this node
                        let __store = &ctx.store;
                        match ctx.run_future(self.#fld.process(__store, x)).await? {
                            Transition::Next(action) => {
                                match action { #(#pats),* _ => {} }
                            }
                            Transition::Finish => {},
                            Transition::Abort(e) => return Err(e),
                        }
                    }
                }
            }
        };
        quote! {
            #work_item_ident::#var_ident(x) => {
                #arm_tokens
            }
        }
    });

    // Start variant
    let start_var = {
        let s = start.to_string();
        let mut cs = s.chars();
        let first = cs.next().unwrap().to_uppercase().to_string();
        let rest: String = cs.collect();
        format_ident!("{}{}", first, rest)
    };
    // Start field type for Input
    let start_ty = fields.iter().find(|(fld,_)| fld == &start)
        .map(|(_,ty)| ty).expect("start field not found");

    // Assemble the expanded code
    let expanded = quote! {

        #[derive(Debug, Clone)]
        #struct_def

        #[allow(non_camel_case_types)]
        enum #work_item_ident {
            #(#work_variants),*
        }


        impl #name #generics {
            /// Execute the workflow, returning the output of the terminal branch
            pub async fn run<C>(
                &self,
                ctx: &floxide_core::WorkflowCtx<C>,
                input: <#start_ty as floxide_core::node::Node>::Input
            ) -> Result<<#terminal_ty as floxide_core::node::Node>::Output, floxide_core::error::FloxideError>
            where
                C: Clone + Send + Sync + 'static,
            {
                use std::collections::VecDeque;
                use floxide_core::transition::Transition;
                let mut work = VecDeque::new();
                work.push_back(#work_item_ident::#start_var(input));
                while let Some(item) = work.pop_front() {
                    match item {
                        #(#run_arms),*
                    }
                }
                unreachable!("Workflow did not reach terminal branch");
            }
        }

        #[async_trait]
        impl floxide_core::workflow::Workflow for #name #generics {
            type Input = <#start_ty as floxide_core::node::Node>::Input;
            type Output = <#terminal_ty as floxide_core::node::Node>::Output;

            async fn run<D>(
                &self,
                ctx: &D,
                input: Self::Input
            ) -> Result<Self::Output, floxide_core::error::FloxideError>
            where D: Clone + Send + Sync + 'static {
                // build a full WorkflowCtx<C> from the store
                let wf_ctx = floxide_core::WorkflowCtx::new(ctx.clone());
                // delegate to the inherent run method
                self.run(&wf_ctx, input).await
            }
        }
    };
    TokenStream::from(expanded)
}
