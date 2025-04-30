use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{
    braced, bracketed,
    parse::{Parse, ParseStream},
    parse_macro_input, Generics, Ident, LitStr, Result, Token, Type, Visibility,
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
    /// Direct edges: list of successor nodes on success, optional fallback on failure
    Direct {
        succs: Vec<Ident>,
        on_failure: Option<Vec<Ident>>,
    },
    /// Composite edges: match on enum variants
    Composite(Vec<CompositeArm>),
}
struct WorkflowDef {
    vis: Visibility,
    name: Ident,
    generics: Generics,
    /// Workflow fields: (name, type, optional retry-policy variable)
    fields: Vec<(Ident, Type, Option<Ident>)>,
    start: Ident,
    context: Type,
    // for each source field, direct successors or composite arms
    edges: Vec<(Ident, EdgeKind)>,
}

impl Parse for WorkflowDef {
    fn parse(input: ParseStream) -> Result<Self> {
        // parse optional visibility
        let vis: Visibility = if input.peek(Token![pub]) {
            input.parse()? // pub or pub(...) etc
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
            // Optional retry annotation: #[retry = policy]
            let mut retry_policy: Option<Ident> = None;
            if content.peek(Token![#]) {
                content.parse::<Token![#]>()?;
                let inner;
                bracketed!(inner in content);
                let attr_name: Ident = inner.parse()?;
                if attr_name == "retry" {
                    inner.parse::<Token![=]>()?;
                    let pol: Ident = inner.parse()?;
                    retry_policy = Some(pol);
                } else {
                    return Err(inner.error("unknown attribute, expected `retry`"));
                }
            }
            // Field name and type
            let fld: Ident = content.parse()?;
            content.parse::<Token![:]>()?;
            let ty: Type = content.parse()?;
            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            }
            fields.push((fld, ty, retry_policy));
        }
        // Now parse the rest in any order: start, context, edges
        let mut start: Option<Ident> = None;
        let mut context: Option<Type> = None;
        let mut edges: Option<Vec<(Ident, EdgeKind)>> = None;
        let mut seen = std::collections::HashSet::new();
        while !input.is_empty() {
            if input.peek(Ident) {
                let fork = input.fork();
                let kw = fork.parse::<Ident>()?;
                match kw.to_string().as_str() {
                    "start" => {
                        if seen.contains("start") {
                            return Err(
                                input.error("Duplicate 'start' field in workflow definition.")
                            );
                        }
                        input.parse::<Ident>()?; // start
                        input.parse::<Token![=]>()?;
                        let s: Ident = input.parse()?;
                        input.parse::<Token![;]>()?;
                        start = Some(s);
                        seen.insert("start");
                    }
                    "context" => {
                        if seen.contains("context") {
                            return Err(
                                input.error("Duplicate 'context' field in workflow definition.")
                            );
                        }
                        input.parse::<Ident>()?; // context
                        input.parse::<Token![=]>()?;
                        let ty: Type = input.parse()?;
                        input.parse::<Token![;]>()?;
                        context = Some(ty);
                        seen.insert("context");
                    }
                    "edges" => {
                        if seen.contains("edges") {
                            return Err(
                                input.error("Duplicate 'edges' field in workflow definition.")
                            );
                        }
                        input.parse::<Ident>()?; // edges
                        let edges_content;
                        braced!(edges_content in input);
                        // Collect direct-success, direct-failure, and composite arms
                        let mut direct_success =
                            std::collections::HashMap::<Ident, Vec<Ident>>::new();
                        let mut direct_failure =
                            std::collections::HashMap::<Ident, Vec<Ident>>::new();
                        let mut composite_map =
                            std::collections::HashMap::<Ident, Vec<CompositeArm>>::new();
                        while !edges_content.is_empty() {
                            let src: Ident = edges_content.parse()?;
                            if edges_content.peek(Ident) {
                                // on_failure clause
                                let kw: Ident = edges_content.parse()?;
                                if kw == "on_failure" {
                                    edges_content.parse::<Token![=>]>()?;
                                    let nested;
                                    braced!(nested in edges_content);
                                    // expect bracketed fallback list
                                    let succs_content;
                                    bracketed!(succs_content in nested);
                                    let fails: Vec<Ident> = succs_content
                                        .parse_terminated(Ident::parse, Token![,])?
                                        .into_iter()
                                        .collect();
                                    edges_content.parse::<Token![;]>()?;
                                    direct_failure.insert(src.clone(), fails);
                                    continue;
                                } else {
                                    return Err(edges_content.error(
                                        "Unexpected identifier. Expected `on_failure` or `=>`.",
                                    ));
                                }
                            }
                            // success or composite entry
                            edges_content.parse::<Token![=>]>()?;
                            let nested;
                            braced!(nested in edges_content);
                            if nested.peek(syn::token::Bracket) {
                                // direct successors
                                let succs_content;
                                bracketed!(succs_content in nested);
                                let succs: Vec<Ident> = succs_content
                                    .parse_terminated(Ident::parse, Token![,])?
                                    .into_iter()
                                    .collect();
                                edges_content.parse::<Token![;]>()?;
                                direct_success.insert(src.clone(), succs);
                            } else {
                                // composite arms
                                let mut arms = Vec::new();
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
                                    arms.push(CompositeArm {
                                        action_path,
                                        variant,
                                        binding,
                                        succs,
                                    });
                                }
                                edges_content.parse::<Token![;]>()?;
                                composite_map.insert(src.clone(), arms);
                            }
                        }
                        // Merge into final edges vector
                        let mut edges_vec = Vec::new();
                        // direct-success entries
                        for (src, succs) in direct_success.into_iter() {
                            let failure = direct_failure.remove(&src);
                            edges_vec.push((
                                src,
                                EdgeKind::Direct {
                                    succs,
                                    on_failure: failure,
                                },
                            ));
                        }
                        // direct-failure-only entries
                        for (src, fails) in direct_failure.into_iter() {
                            edges_vec.push((
                                src,
                                EdgeKind::Direct {
                                    succs: Vec::new(),
                                    on_failure: Some(fails),
                                },
                            ));
                        }
                        // composite entries
                        for (src, arms) in composite_map.into_iter() {
                            edges_vec.push((src, EdgeKind::Composite(arms)));
                        }
                        edges = Some(edges_vec);
                        seen.insert("edges");
                    }
                    other => {
                        return Err(input.error(format!(
                            "Unexpected identifier '{}'. Expected one of: start, context, edges.",
                            other
                        )));
                    }
                }
            } else {
                return Err(input.error("Unexpected token in workflow definition. Expected 'start', 'context', or 'edges'."));
            }
        }
        // Check required fields
        let start = start
            .ok_or_else(|| input.error("Missing required 'start' field in workflow definition."))?;
        let context = context.unwrap_or_else(|| syn::parse_quote! { () });
        let edges = edges
            .ok_or_else(|| input.error("Missing required 'edges' field in workflow definition."))?;
        Ok(WorkflowDef {
            vis,
            name,
            generics,
            fields,
            start,
            context,
            edges,
        })
    }
}

pub fn workflow(item: TokenStream) -> TokenStream {
    // parse the struct-based workflow definition
    let WorkflowDef {
        vis,
        name,
        generics,
        fields,
        start,
        context,
        edges,
    } = parse_macro_input!(item as WorkflowDef);
    // Prepare generics for impl
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // Helper: convert snake_case to CamelCase for identifiers and labels
    let to_camel_case = |s: &str| -> String {
        s.split('_')
            .filter(|p| !p.is_empty())
            .map(|p| {
                let mut cs = p.chars();
                let first = cs.next().unwrap().to_uppercase().to_string();
                let rest: String = cs.collect();
                first + &rest
            })
            .collect()
    };

    // Build WorkItem enum name
    let work_item_ident = format_ident!("{}WorkItem", name);
    // Determine terminal branch: a field with no successors (direct empty or composite empty)
    let terminal_src = edges.iter().find(|(_, kind)| {
        matches!(kind, EdgeKind::Direct { succs, on_failure } if succs.is_empty() && on_failure.is_none())
        || matches!(kind, EdgeKind::Composite(ar) if ar.is_empty())
    }).expect("Workflow must have a terminal branch").0.clone();
    // Find the type of the terminal field
    let terminal_ty = fields
        .iter()
        .find(|(fld, _, _)| fld == &terminal_src)
        .map(|(_, ty, _)| ty.clone())
        .expect("Terminal field not found among fields");
    // Find the terminal variant ident(s)
    let terminal_variant_idents: Vec<_> = edges
        .iter()
        .filter_map(|(src, kind)| {
            let is_terminal = match kind {
                EdgeKind::Direct { succs, on_failure } => succs.is_empty() && on_failure.is_none(),
                EdgeKind::Composite(arms) => arms.is_empty(),
            };
            if is_terminal {
                let var_name = to_camel_case(&src.to_string());
                Some(format_ident!("{}", var_name))
            } else {
                None
            }
        })
        .collect();
    // Identify policy fields (names used in #[retry = policy])
    let policy_idents: Vec<Ident> = fields
        .iter()
        .filter_map(|(_, _, retry)| retry.clone())
        .collect();
    // Only actual workflow node fields should produce variants and arms
    let node_fields: Vec<_> = fields
        .iter()
        .filter(|(fld, _, _)| !policy_idents.iter().any(|p| p == fld))
        .collect();
    // Generate WorkItem variants for node fields, parameterized by context C
    let work_variants = node_fields.iter().map(|(fld, ty, _)| {
        // Variant name: CamelCase from field name
        let var_name = to_camel_case(&fld.to_string());
        let var_ident = format_ident!("{}", var_name);
        // Each variant carries a unique UUID string and the node input payload
        quote! { #var_ident(String, <#ty as floxide_core::node::Node<#context>>::Input) }
    });
    // Collect variant idents for Display and WorkItem impl
    let work_variant_idents: Vec<_> = node_fields
        .iter()
        .map(|(fld, _, _)| {
            let var_name = to_camel_case(&fld.to_string());
            format_ident!("{}", var_name)
        })
        .collect();

    // Struct definition

    // Define the workflow struct with optional retry-policy fields
    let struct_def = {
        // Each entry: (field_name, field_type, retry_policy)
        let field_defs = fields.iter().map(|(fld, ty, _)| quote! { pub #fld: #ty });
        quote! { #vis struct #name #generics { #(#field_defs),* } }
    };

    // Generate run method arms for each field
    // We collect into a Vec so we can reuse in multiple generated methods
    let run_arms: Vec<_> = node_fields.iter().map(|(fld, _ty, retry)| {
        // WorkItem variant
        let var_name = to_camel_case(&fld.to_string());
        let var_ident = format_ident!("{}", var_name);
        // Find edges for this field
        let kind = edges.iter().find(|(src, _)| src == fld)
            .map(|(_, k)| k)
            .unwrap_or_else(|| panic!("No edges defined for field {}", fld));
        // Build a node wrapper: apply `with_retry` if a retry policy is specified
        let wrapper = if let Some(pol) = retry {
            quote! {
                // wrap the inner node with retry policy
                let __node = floxide_core::with_retry(self.#fld.clone(), self.#pol.clone());
            }
        } else {
            quote! {
                let __node = self.#fld.clone();
            }
        };
        // Generate code based on edge kind, using __node.process instead of self.#fld.process
        let arm_tokens = match kind {
            EdgeKind::Direct { succs, on_failure } => {
                // Build tokens for success path
                let push_success = if succs.is_empty() {
                    // TERMINAL: return Ok(Some(action))
                    quote! { return Ok(Some(action)) }
                } else {
                    let pushes = succs.iter().map(|succ| {
                        let var_name = to_camel_case(&succ.to_string());
                        let succ_var = format_ident!("{}", var_name);
                        quote! { __q.push_back(#work_item_ident::#succ_var(::uuid::Uuid::new_v4().to_string(), action.clone())); }
                    });
                    quote! { #(#pushes)* return Ok(None) }
                };
                // Build tokens for failure/fallback path
                let push_failure = if let Some(fails) = on_failure {
                    // Schedule fallback successors with new UUID and same input
                    let pushes = fails.iter().map(|succ| {
                        let var_name = to_camel_case(&succ.to_string());
                        let succ_var = format_ident!("{}", var_name);
                        quote! { __q.push_back(#work_item_ident::#succ_var(::uuid::Uuid::new_v4().to_string(), input.clone())); }
                    });
                    quote! { #(#pushes)* return Ok(None) }
                } else {
                    quote! { return Err(e) }
                };
                // Build tokens for NextAll (fan-out) path
                    let push_all = {
                        if succs.is_empty() {
                            // no successors: just return
                            quote! {
                                tracing::debug!(?actions, "Node produced Transition::NextAll");
                                return Ok(None);
                            }
                        } else {
                            // For each emitted action_item, push to all successors
                            let pushes_all = succs.iter().map(|succ| {
                                let var_name = to_camel_case(&succ.to_string());
                                let succ_var = format_ident!("{}", var_name);
                                quote! { __q.push_back(#work_item_ident::#succ_var(::uuid::Uuid::new_v4().to_string(), action_item.clone())); }
                            });
                            quote! {
                                tracing::debug!(?actions, "Node produced Transition::NextAll");
                                for action_item in actions {
                                    #(#pushes_all)*
                                }
                                return Ok(None);
                            }
                        }
                    };
                // Generate match with explicit error handling and tracing logs
                quote! {
                    #wrapper
                    let __store = &ctx.store;
                    let node_span = tracing::span!(tracing::Level::DEBUG, "node_execution", node = stringify!(#var_ident));
                    let _node_enter = node_span.enter();
                    tracing::debug!(ctx = ?ctx.store, ?input, "Node input and context");
                    match ctx.run_future(__node.process(__store, input.clone())).await {
                        Ok(Transition::NextAll(actions)) => {
                            // Fan-out: multiple outputs
                            #push_all
                        },
                        Ok(Transition::Next(action)) => {
                            tracing::debug!(?action, "Node produced Transition::Next");
                            #push_success
                        },
                        Ok(Transition::Abort(e)) => {
                            tracing::warn!(error = ?e, "Node produced Transition::Abort");
                            #push_failure
                        },
                        Err(e) => {
                            tracing::error!(error = ?e, "Node process returned error");
                            #push_failure
                        },
                        // Hold: pause without emitting successors
                        Ok(Transition::Hold) => {
                            tracing::debug!("Node produced Transition::Hold");
                            return Ok(None);
                        },
                    }
                }
            }
            EdgeKind::Composite(composite) => {
                if composite.is_empty() {
                    // terminal composite branch: return the output value as Ok(Some(action))
                        quote! {
                        #wrapper
                        let __store = &ctx.store;
                        let node_span = tracing::span!(tracing::Level::DEBUG, "node_execution", node = stringify!(#var_ident));
                        let _node_enter = node_span.enter();
                        tracing::debug!(store = ?ctx.store, ?input, "Node input and store");
                        match ctx.run_future(__node.process(__store, input.clone())).await? {
                            // Hold: pause without emitting successors
                            Transition::Hold => {
                                tracing::debug!("Node produced Transition::Hold");
                                return Ok(None);
                            }
                            Transition::Next(action) => {
                                tracing::debug!(?action, "Node produced Transition::Next (terminal composite)");
                                return Ok(Some(action));
                            }
                            Transition::Abort(e) => {
                                tracing::warn!(error = ?e, "Node produced Transition::Abort (terminal composite)");
                                return Err(e);
                            }
                            Transition::NextAll(_) => unreachable!("Unexpected Transition::NextAll in terminal composite node"),
                        }
                    }
                } else {
                    // composite edges: pattern-based
                    let pats_terminal = composite.iter().filter_map(|arm| {
                        let CompositeArm { action_path, variant, binding, succs } = arm;
                        if succs.is_empty() {
                            let pat = quote! { #action_path :: #variant (#binding) };
                            Some(quote! {
                                #pat => {
                                    tracing::debug!(variant = stringify!(#variant), value = ?#binding, "Composite arm: terminal variant");
                                    return Ok(Some(#binding));
                                }
                            })
                        } else {
                            None
                        }
                    });
                    let pats_non_terminal = composite.iter().filter_map(|arm| {
                        let CompositeArm { action_path, variant, binding, succs } = arm;
                        if !succs.is_empty() {
                            let pat = quote! { #action_path :: #variant (#binding) };
                            let succ_pushes = succs.iter().map(|succ| {
                                let var_name = to_camel_case(&succ.to_string());
                                let succ_var = format_ident!("{}", var_name);
                                quote! { __q.push_back(#work_item_ident::#succ_var(::uuid::Uuid::new_v4().to_string(), #binding)); }
                            });
                            Some(quote! {
                                #pat => {
                                    tracing::debug!(variant = stringify!(#variant), value = ?#binding, "Composite arm: scheduling successors");
                                    #(#succ_pushes)*
                                    return Ok(None);
                                }
                            })
                        } else {
                            None
                        }
                    });
                    quote! {
                        #wrapper
                        let __store = &ctx.store;
                        let node_span = tracing::span!(tracing::Level::DEBUG, "node_execution", node = stringify!(#var_ident));
                        let _node_enter = node_span.enter();
                        tracing::debug!(store = ?ctx.store, ?input, "Node input and store");
                        match ctx.run_future(__node.process(__store, input.clone())).await? {
                            Transition::Hold => {
                                tracing::debug!("Node produced Transition::Hold");
                                return Ok(None);
                            }
                            Transition::Next(action) => {
                                tracing::debug!(?action, "Node produced Transition::Next (composite)");
                                match action {
                                    #(#pats_terminal)*
                                    #(#pats_non_terminal)*
                                    _ => {
                                        tracing::warn!("Composite arm: unmatched variant");
                                        return Ok(None);
                                    }
                                }
                            }
                            Transition::Abort(e) => {
                                tracing::warn!(error = ?e, "Node produced Transition::Abort (composite)");
                                return Err(e);
                            }
                            Transition::NextAll(_) => unreachable!("Unexpected Transition::NextAll in composite node"),
                        }
                    }
                }
            }
        };
        quote! {
            #work_item_ident::#var_ident(_id, input) => {
                #arm_tokens
            }
        }
    }).collect();

    // Start variant
    let start_var = {
        let var_name = to_camel_case(&start.to_string());
        format_ident!("{}", var_name)
    };
    // Start field type for Input
    let start_ty = fields
        .iter()
        .find(|(fld, _, _)| fld == &start)
        .map(|(_, ty, _)| ty)
        .expect("start field not found");

    // Generate DOT string at compile time
    let dot = {
        let mut dot = String::new();
        dot.push_str("digraph ");
        dot.push_str(&name.to_string());
        dot.push_str(" {\n");
        dot.push_str("  rankdir=LR;\n");
        dot.push_str("  node [shape=box style=filled fontname=\"Helvetica\" color=lightgray];\n");
        dot.push_str("  edge [fontname=\"Helvetica\" arrowhead=vee];\n");
        // Nodes
        for (fld, _, _) in &node_fields {
            let var = to_camel_case(&fld.to_string());
            dot.push_str("  ");
            dot.push_str(&var);
            dot.push_str(";\n");
        }
        // Edges
        for (src, kind) in &edges {
            let src_var = to_camel_case(&src.to_string());
            match kind {
                EdgeKind::Direct { succs, on_failure } => {
                    for succ in succs {
                        let succ_var = to_camel_case(&succ.to_string());
                        dot.push_str("  ");
                        dot.push_str(&src_var);
                        dot.push_str(" -> ");
                        dot.push_str(&succ_var);
                        dot.push_str(";\n");
                    }
                    if let Some(fails) = on_failure {
                        for fail in fails {
                            let fail_var = to_camel_case(&fail.to_string());
                            dot.push_str("  ");
                            dot.push_str(&src_var);
                            dot.push_str(" -> ");
                            dot.push_str(&fail_var);
                            dot.push_str(" [style=\"dotted\" color=\"red\" label=\"fallback\"];\n");
                        }
                    }
                }
                EdgeKind::Composite(arms) => {
                    for arm in arms {
                        let label = arm.variant.to_string();
                        for succ in &arm.succs {
                            let succ_var = to_camel_case(&succ.to_string());
                            dot.push_str("  ");
                            dot.push_str(&src_var);
                            dot.push_str(" -> ");
                            dot.push_str(&succ_var);
                            dot.push_str(" [label=\"");
                            dot.push_str(&label);
                            dot.push_str("\"];\n");
                        }
                    }
                }
            }
        }

        dot.push_str("}\n");
        dot
    };
    let dot_literal = LitStr::new(&dot, Span::call_site());

    // Assemble the expanded code
    let expanded = quote! {

        #[derive(Debug, Clone)]
        #struct_def

        #[allow(non_camel_case_types)]
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
        #vis enum #work_item_ident {
            #(#work_variants),*
        }
        // Display impl renders only the variant name and payload (ignoring the UUID)
        impl std::fmt::Display for #work_item_ident {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    #(
                        #work_item_ident::#work_variant_idents(_, x) => {
                            write!(f, "{}({:?})", stringify!(#work_variant_idents), x)
                        }
                    ),*
                }
            }
        }
        // WorkItem impl uses the embedded UUID as the unique instance ID
        impl floxide_core::workflow::WorkItem for #work_item_ident {
            fn instance_id(&self) -> String {
                match self {
                    #(
                        #work_item_ident::#work_variant_idents(id, _) => id.clone(),
                    )*
                }
            }
            fn is_terminal(&self) -> bool {
                match self {
                    #(
                        #work_item_ident::#terminal_variant_idents(..) => true,
                    )*
                    _ => false,
                }
            }
        }

        #[async_trait::async_trait]
        impl #impl_generics floxide_core::workflow::Workflow<#context> for #name #ty_generics #where_clause {
            type Input = <#start_ty as floxide_core::node::Node<#context>>::Input;
            type Output = <#terminal_ty as floxide_core::node::Node<#context>>::Output;
            type WorkItem = #work_item_ident;

            fn name(&self) -> &'static str {
                stringify!(#name)
            }

            fn start_work_item(&self, input: Self::Input) -> Self::WorkItem {
                #work_item_ident::#start_var(
                    ::uuid::Uuid::new_v4().to_string(),
                    input,
                )
            }

            async fn process_work_item<'a>(
                &'a self,
                ctx: &'a floxide_core::WorkflowCtx<#context>,
                item: Self::WorkItem,
                __q: &mut std::collections::VecDeque<Self::WorkItem>
            ) -> Result<Option<Self::Output>, floxide_core::error::FloxideError>
            {
                match item {
                    #(
                        #run_arms
                    ),*
                }
            }

            fn to_dot(&self) -> &'static str {
                #dot_literal
            }
        }
    };
    TokenStream::from(expanded)
}
