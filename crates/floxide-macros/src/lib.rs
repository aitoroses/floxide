// crates/floxide-macros/src/workflow.rs

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    bracketed, braced,
    parse::{Parse, ParseStream},
    parse_macro_input, token, Ident, Result, Token,
    Visibility, Generics, Type, Path,
};

/// AST for struct-based workflow: struct fields, start field, and per-node edges
// Internal representation of a composite edge arm: matches Output enum variant
struct CompositeArm {
    action_path: Ident,
    variant: Ident,
    binding: Ident,
    succs: Vec<Ident>,
}
// AST for struct-based workflow: struct fields, start field, and per-node composite edges
struct WorkflowDef {
    vis: Visibility,
    name: Ident,
    generics: Generics,
    fields: Vec<(Ident, Type)>,
    start: Ident,
    // for each source field, a list of composite arms
    edges: Vec<(Ident, Vec<CompositeArm>)>,
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
            // composite edges in a braced block
            let nested;
            braced!(nested in edges_content);
            let mut composite = Vec::new();
            while !nested.is_empty() {
                // parse pattern: EnumName::Variant(binding)
                let action_path: Ident = nested.parse()?;
                nested.parse::<Token![::]>()?;
                let variant: Ident = nested.parse()?;
                let inner;
                syn::parenthesized!(inner in nested);
                let binding: Ident = inner.parse()?;
                nested.parse::<Token![=>]>()?;
                // parse successors list
                let succs_content;
                bracketed!(succs_content in nested);
                let succs: Vec<Ident> = succs_content
                    .parse_terminated(Ident::parse, Token![,])?
                    .into_iter()
                    .collect();
                nested.parse::<Token![;]>()?;
                composite.push(CompositeArm { action_path, variant, binding, succs });
            }
            edges_content.parse::<Token![;]>()?;
            edges.push((src, composite));
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
    let run_arms = fields.iter().map(|(fld, ty)| {
        // WorkItem variant
        let fld_str = fld.to_string();
        let mut chars = fld_str.chars();
        let first = chars.next().unwrap().to_uppercase().to_string();
        let rest: String = chars.collect();
        let var_ident = format_ident!("{}{}", first, rest);
        // Find edges for this field
        let arm = edges.iter().find(|(src, _)| src == fld);
        let arm_tokens = if let Some((_, composite)) = arm {
            if composite.is_empty() {
                // no edges: treat Next(_) or Finish as terminal
                quote! {
                    match self.#fld.process(ctx, x).await? {
                        Transition::Next(_) | Transition::Finish => {},
                        Transition::Abort(e) => return Err(e),
                    }
                }
            } else {
                // composite edges: pattern-based
                let pats = composite.iter().map(|arm| {
                    let CompositeArm { action_path, variant, binding, succs } = arm;
                    // build pattern: Path::Variant(binding)
                    let pat = quote! { #action_path :: #variant (#binding) };
                    // push variants for succs
                    let succ_pushes = succs.iter().map(|succ| {
                        // succ variant name: capitalize succ ident
                        let succ_str = succ.to_string();
                        let mut sc = succ_str.chars();
                        let sf = sc.next().unwrap().to_uppercase().to_string();
                        let rest: String = sc.collect();
                        let succ_var = format_ident!("{}{}", sf, rest);
                        quote! { work.push_back(#work_item_ident::#succ_var(#binding)); }
                    });
                    quote! { #pat => { #(#succ_pushes)* } }
                });
                quote! {
                    match self.#fld.process(ctx, x).await? {
                        Transition::Next(action) => {
                            match action { #(#pats),* _ => {} }
                        }
                        Transition::Finish => {},
                        Transition::Abort(e) => return Err(e),
                    }
                }
            }
        } else {
            panic!("No edges defined for field {}", fld);
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
        #struct_def
        
        #[allow(non_camel_case_types)]
        enum #work_item_ident {
            #(#work_variants),*
        }

        #[async_trait]
        impl floxide_core::workflow::Workflow for #name #generics {
            type Input = <#start_ty as floxide_core::node::Node>::Input;

            async fn run(
                &mut self,
                ctx: &mut floxide_core::context::WorkflowCtx<()>,
                input: Self::Input
            ) -> Result<(), floxide_core::error::FloxideError> {
                use std::collections::VecDeque;
                use floxide_core::transition::Transition;
                let mut work = VecDeque::new();
                work.push_back(#work_item_ident::#start_var(input));
                while let Some(item) = work.pop_front() {
                    match item {
                        #(#run_arms),*
                    }
                }
                Ok(())
            }
        }
    };
    TokenStream::from(expanded)
}
