use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    braced, bracketed, parse::{Parse, ParseStream}, parse_macro_input, Generics, Ident, Result, Token, Type, Visibility, LitStr
};
use proc_macro2::Span;

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
    Direct { succs: Vec<Ident>, on_failure: Option<Vec<Ident>> },
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
                            return Err(input.error("Duplicate 'start' field in workflow definition."));
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
                            return Err(input.error("Duplicate 'context' field in workflow definition."));
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
                            return Err(input.error("Duplicate 'edges' field in workflow definition."));
                        }
                        input.parse::<Ident>()?; // edges
                        let edges_content;
                        braced!(edges_content in input);
                        // Collect direct-success, direct-failure, and composite arms
                        let mut direct_success = std::collections::HashMap::<Ident, Vec<Ident>>::new();
                        let mut direct_failure = std::collections::HashMap::<Ident, Vec<Ident>>::new();
                        let mut composite_map = std::collections::HashMap::<Ident, Vec<CompositeArm>>::new();
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
                                    return Err(edges_content.error("Unexpected identifier. Expected `on_failure` or `=>`."));
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
                                    arms.push(CompositeArm { action_path, variant, binding, succs });
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
                            edges_vec.push((src, EdgeKind::Direct { succs, on_failure: failure }));
                        }
                        // direct-failure-only entries
                        for (src, fails) in direct_failure.into_iter() {
                            edges_vec.push((src, EdgeKind::Direct { succs: Vec::new(), on_failure: Some(fails) }));
                        }
                        // composite entries
                        for (src, arms) in composite_map.into_iter() {
                            edges_vec.push((src, EdgeKind::Composite(arms)));
                        }
                        edges = Some(edges_vec);
                        seen.insert("edges");
                    }
                    other => {
                        return Err(input.error(format!("Unexpected identifier '{}'. Expected one of: start, context, edges.", other)));
                    }
                }
            } else {
                return Err(input.error("Unexpected token in workflow definition. Expected 'start', 'context', or 'edges'."));
            }
        }
        // Check required fields
        let start = start.ok_or_else(|| input.error("Missing required 'start' field in workflow definition."))?;
        let context = context.unwrap_or_else(|| syn::parse_quote! { () });
        let edges = edges.ok_or_else(|| input.error("Missing required 'edges' field in workflow definition."))?;
        Ok(WorkflowDef { vis, name, generics, fields, start, context, edges })
    }
}


pub fn workflow(item: TokenStream) -> TokenStream {
    // parse the struct-based workflow definition
    let WorkflowDef { vis, name, generics, fields, start, context, edges } =
        parse_macro_input!(item as WorkflowDef);

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
    let terminal_ty = fields.iter()
        .find(|(fld, _, _)| fld == &terminal_src)
        .map(|(_, ty, _)| ty.clone())
        .expect("Terminal field not found among fields");
    // Identify policy fields (names used in #[retry = policy])
    let policy_idents: Vec<Ident> = fields.iter()
        .filter_map(|(_, _, retry)| retry.clone())
        .collect();
    // Only actual workflow node fields should produce variants and arms
    let node_fields: Vec<_> = fields.iter()
        .filter(|(fld, _, _)| !policy_idents.iter().any(|p| p == fld))
        .collect();
    // Generate WorkItem variants for node fields, parameterized by context C
    let work_variants = node_fields.iter().map(|(fld, ty, _)| {
        // Variant name: CamelCase from field name
        let var_name = to_camel_case(&fld.to_string());
        let var_ident = format_ident!("{}", var_name);
        quote! { #var_ident(<#ty as floxide_core::node::Node<#context>>::Input) }
    });
    // Compute the WorkItem variant name for the terminal field
    let terminal_var = {
        let var_name = to_camel_case(&terminal_src.to_string());
        format_ident!("{}", var_name)
    };

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
                        quote! { __q.push_back(#work_item_ident::#succ_var(action.clone())); }
                    });
                    quote! { #(#pushes)* return Ok(None) }
                };
                // Build tokens for failure/fallback path
                let push_failure = if let Some(fails) = on_failure {
                    let succ_vars: Vec<_> = fails.iter().map(|succ| {
                        let var_name = to_camel_case(&succ.to_string());
                        format_ident!("{}", var_name)
                    }).collect();
                    quote! { #( __q.push_back(#work_item_ident::#succ_vars({})); )* return Ok(None) }
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
                            quote! { __q.push_back(#work_item_ident::#succ_var(action_item.clone())); }
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
                    tracing::debug!(ctx = ?ctx.store, ?x, "Node input and context");
                    match ctx.run_future(__node.process(__store, x)).await {
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
                        // Unexpected fan-out in non-split node
                        _ => unreachable!("Unexpected Transition variant"),
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
                        tracing::debug!(store = ?ctx.store, ?x, "Node input and store");
                        match ctx.run_future(__node.process(__store, x)).await? {
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
                                quote! { __q.push_back(#work_item_ident::#succ_var(#binding)); }
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
                        tracing::debug!(store = ?ctx.store, ?x, "Node input and store");
                        match ctx.run_future(__node.process(__store, x)).await? {
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
            #work_item_ident::#var_ident(x) => {
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
    let start_ty = fields.iter().find(|(fld,_,_)| fld == &start)
        .map(|(_, ty, _)| ty).expect("start field not found");

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
            dot.push_str("  "); dot.push_str(&var); dot.push_str(";\n");
        }
        // Edges
        for (src, kind) in &edges {
            let src_var = to_camel_case(&src.to_string());
            match kind {
                EdgeKind::Direct { succs, on_failure } => {
                    for succ in succs {
                        let succ_var = to_camel_case(&succ.to_string());
                        dot.push_str("  "); dot.push_str(&src_var);
                        dot.push_str(" -> "); dot.push_str(&succ_var); dot.push_str(";\n");
                    }
                    if let Some(fails) = on_failure {
                        for fail in fails {
                            let fail_var = to_camel_case(&fail.to_string());
                            dot.push_str("  "); dot.push_str(&src_var);
                            dot.push_str(" -> "); dot.push_str(&fail_var);
                            dot.push_str(" [style=\"dotted\" color=\"red\" label=\"fallback\"];\n");
                        }
                    }
                }
                EdgeKind::Composite(arms) => {
                    for arm in arms {
                        let label = arm.variant.to_string();
                        for succ in &arm.succs {
                            let succ_var = to_camel_case(&succ.to_string());
                            dot.push_str("  "); dot.push_str(&src_var);
                            dot.push_str(" -> "); dot.push_str(&succ_var);
                            dot.push_str(" [label=\""); dot.push_str(&label); dot.push_str("\"];\n");
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
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        #vis enum #work_item_ident {
            #(#work_variants),*
        }

        impl #name #generics {
            /// Export the workflow definition as a Graphviz DOT string.
            fn _to_dot(&self) -> &'static str {
                #dot_literal
            }

            #[allow(unreachable_code)]
            async fn _process_work_item<'a>(
                &'a self,
                ctx: &'a floxide_core::WorkflowCtx<#context>,
                item: #work_item_ident,
                __q: &mut std::collections::VecDeque<#work_item_ident>
            ) -> Result<Option<<#terminal_ty as floxide_core::node::Node<#context>>::Output>, floxide_core::error::FloxideError>           
            {
                use floxide_core::transition::Transition;
                use tracing::{debug, error, warn};
                match item {
                    #(
                        #run_arms
                    ),*
                }
                Ok(None)
            }

            /// Execute the workflow, returning the output of the terminal branch
            #[allow(unreachable_code)]
            async fn _run(
                &self,
                ctx: &floxide_core::WorkflowCtx<#context>,
                input: <#start_ty as floxide_core::node::Node<#context>>::Input,
            ) -> Result<<#terminal_ty as floxide_core::node::Node<#context>>::Output, floxide_core::error::FloxideError>
            {
                use std::collections::VecDeque;
                use tracing::{debug, error, info, span, Level};
                let span = span!(Level::INFO, "workflow_run", workflow = stringify!(#name));
                let _enter = span.enter();
                debug!(?input, store = ?ctx.store, "Starting workflow run");
                let mut __q = VecDeque::new();
                __q.push_back(#work_item_ident::#start_var(input));
                while let Some(item) = __q.pop_front() {
                    debug!(?item, queue_len = __q.len(), "Processing work item");
                    if let Some(output) = self.process_work_item(ctx, item, &mut __q).await? {
                        return Ok(output);
                    }
                    debug!(queue_len = __q.len(), "Queue state after processing");
                }
                unreachable!("Workflow did not reach terminal branch");
            }

            #[allow(unreachable_code)]
            /// Execute the workflow, checkpointing state after each step.
            async fn _run_with_checkpoint<CS: floxide_core::checkpoint::CheckpointStore<#context, #work_item_ident>>(
                &self,
                ctx: &floxide_core::WorkflowCtx<#context>,
                input: <#start_ty as floxide_core::node::Node<#context>>::Input,
                store: &CS,
                id: &str,
            ) -> Result<<#terminal_ty as floxide_core::node::Node<#context>>::Output, floxide_core::error::FloxideError>
            {
                use std::collections::VecDeque;
                use floxide_core::{Checkpoint, CheckpointStore};
                use tracing::{debug, error, info, span, Level};
                let span = span!(Level::INFO, "workflow_run_with_checkpoint", workflow = stringify!(#name), run_id = id);
                let _enter = span.enter();
                debug!(?input, "Starting workflow run with checkpoint");
                // load existing checkpoint or start new
                let mut cp: floxide_core::Checkpoint<#context, #work_item_ident> = match store.load(id)
                    .await
                    .map_err(|e| floxide_core::error::FloxideError::Generic(e.to_string()))? {
                    Some(saved) => {
                        debug!("Loaded existing checkpoint");
                        saved
                    },
                    None => {
                        debug!("No checkpoint found, starting new");
                        let mut q = VecDeque::new();
                        q.push_back(#work_item_ident::#start_var(input));
                        // create initial checkpoint
                        floxide_core::Checkpoint::new(ctx.store.clone(), q)
                    }
                };
                // initialize working queue from checkpoint
                let mut __q = cp.queue.clone();
                // if there's no pending work, the workflow has already completed
                if __q.is_empty() {
                    info!("Workflow already completed (empty queue)");
                    return Err(floxide_core::error::FloxideError::AlreadyCompleted);
                }
                // process loop with persistence
                while let Some(item) = __q.pop_front() {
                    debug!(?item, queue_len = __q.len(), "Processing work item");
                    if let Some(output) = self.process_work_item(ctx, item, &mut __q).await? {
                        return Ok(output);
                    }
                    debug!(queue_len = __q.len(), "Queue state after processing");
                    // update checkpoint state and persist
                    cp.queue = __q.clone();
                    // persist checkpoint
                    store.save(id, &cp)
                        .await
                        .map_err(|e| floxide_core::error::FloxideError::Generic(e.to_string()))?;
                    debug!("Checkpoint saved");
                }
                unreachable!("Workflow did not reach terminal branch");
            }

            #[allow(unreachable_code)]
            /// Resume a workflow run from its last checkpoint; context and queue are restored from store.
            async fn _resume<CS: floxide_core::checkpoint::CheckpointStore<#context, #work_item_ident>>(
                &self,
                store: &CS,
                id: &str,
            ) -> Result<<#terminal_ty as floxide_core::node::Node<#context>>::Output, floxide_core::error::FloxideError>
            {
                use std::collections::VecDeque;
                use tracing::{debug, error, info, span, Level};
                let span = span!(Level::INFO, "workflow_resume", workflow = stringify!(#name), checkpoint_id = id);
                let _enter = span.enter();
                // Load persisted checkpoint or error if never run
                let cp: floxide_core::Checkpoint<#context, #work_item_ident> = store.load(id)
                    .await
                    .map_err(|e| floxide_core::error::FloxideError::Generic(e.to_string()))?
                    .ok_or_else(|| floxide_core::error::FloxideError::NotStarted)?;
                debug!("Loaded checkpoint for resume");
                // Rebuild WorkflowCtx from saved context
                let wf_ctx = floxide_core::WorkflowCtx::new(cp.context.clone());
                let ctx = &wf_ctx;
                // Restore work queue from checkpoint
                let mut __q: VecDeque<#work_item_ident> = cp.queue.clone();
                // If there is no pending work, the workflow has already completed
                if __q.is_empty() {
                    info!("Workflow already completed (empty queue)");
                    return Err(floxide_core::error::FloxideError::AlreadyCompleted);
                }
                // if the only pending work is the terminal node, treat as completed
                if __q.len() == 1 {
                    if let #work_item_ident::#terminal_var(_) = __q.front().unwrap() {
                        info!("Workflow already completed (terminal node)");
                        return Err(floxide_core::error::FloxideError::AlreadyCompleted);
                    }
                }
                // Process remaining items
                while let Some(item) = __q.pop_front() {
                    debug!(?item, queue_len = __q.len(), "Processing work item");
                    if let Some(output) = self.process_work_item(ctx, item, &mut __q).await? {
                        return Ok(output);
                    }
                    debug!(queue_len = __q.len(), "Queue state after processing");
                }
                unreachable!("Workflow did not reach terminal branch");
            }
        // keep impl open for distributed methods

        // ===== Distributed API =====
        /// Orchestrator: seed the distributed workflow (checkpoint + queue) but do not execute steps.
        async fn _start_distributed<CS, Q>(
            &self,
            ctx: &floxide_core::WorkflowCtx<#context>,
            input: <#start_ty as floxide_core::node::Node<#context>>::Input,
            store: &CS,
            queue: &Q,
            id: &str,
        ) -> Result<(), floxide_core::error::FloxideError>
        where
            CS: floxide_core::checkpoint::CheckpointStore<#context, #work_item_ident>,
            Q: floxide_core::distributed::WorkQueue<#work_item_ident>,
        {
            use std::collections::VecDeque;
            use tracing::{span, Level};
            // Span for seeding the distributed run (debug level)
            let seed_span = span!(Level::DEBUG, "start_distributed",
                workflow = stringify!(#name), run_id = %id);
            let _enter = seed_span.enter();
            tracing::debug!(run_id = %id, "start_distributed seeding");
            // Seed initial checkpoint+queue if not already started
            let saved = store.load(id)
                .await
                .map_err(|e| floxide_core::error::FloxideError::Generic(e.to_string()))?;
            if saved.is_none() {
                let mut init_q = VecDeque::new();
                init_q.push_back(#work_item_ident::#start_var(input.clone()));
                let cp0 = floxide_core::Checkpoint::new(ctx.store.clone(), init_q.clone());
                store.save(id, &cp0)
                    .await
                    .map_err(|e| floxide_core::error::FloxideError::Generic(e.to_string()))?;
                queue.enqueue(id, #work_item_ident::#start_var(input))
                    .await
                    .map_err(|e| floxide_core::error::FloxideError::Generic(e.to_string()))?;
            }
            Ok(())
        }

        /// Worker: perform one distributed step (dequeue, process, enqueue successors, persist).
        async fn _step_distributed<CS, Q>(
            &self,
            store: &CS,
            queue: &Q,
            worker_id: usize,
        ) -> Result<Option<(String, <#terminal_ty as floxide_core::node::Node<#context>>::Output)>, floxide_core::distributed::StepError<#work_item_ident>>
        where
            CS: floxide_core::checkpoint::CheckpointStore<#context, #work_item_ident>,
            Q: floxide_core::distributed::WorkQueue<#work_item_ident>,
        {
            use std::collections::VecDeque;
            use tracing::{debug, span, Level};
            // 1) Dequeue one work item from any workflow
            let work = queue.dequeue().await
                .map_err(|e| floxide_core::distributed::StepError {
                    error: floxide_core::error::FloxideError::Generic(e.to_string()),
                    run_id: None,
                    work_item: None,
                })?;
            let (run_id, item) = match work {
                None => return Ok(None),
                Some((rid, it)) => (rid, it),
            };
            let step_span = span!(Level::DEBUG, "step_distributed",
                workflow = stringify!(#name), run_id = %run_id, worker = worker_id);
            let _enter = step_span.enter();
            debug!(worker = worker_id, run_id = %run_id, ?item, "Worker dequeued item");
            // 2) Load checkpoint
            let mut cp = store.load(&run_id)
                .await
                .map_err(|e| floxide_core::distributed::StepError {
                    error: floxide_core::error::FloxideError::Generic(e.to_string()),
                    run_id: Some(run_id.clone()),
                    work_item: Some(item.clone()),
                })?
                .ok_or_else(|| floxide_core::distributed::StepError {
                    error: floxide_core::error::FloxideError::NotStarted,
                    run_id: Some(run_id.clone()),
                    work_item: Some(item.clone()),
                })?;
            debug!(worker = worker_id, run_id = %run_id, queue_len = cp.queue.len(), "Loaded checkpoint");
            let wf_ctx = floxide_core::WorkflowCtx::new(cp.context.clone());
            let ctx_ref = &wf_ctx;
            let mut local_q = cp.queue.clone();
            let _ = local_q.pop_front();
            let old_tail = local_q.clone();
            let process_result = self.process_work_item(ctx_ref, item.clone(), &mut local_q).await;
            match process_result {
                Ok(Some(out)) => {
                    cp.context = wf_ctx.store.clone();
                    cp.queue = local_q.clone();
                    store.save(&run_id, &cp)
                        .await
                        .map_err(|e| floxide_core::distributed::StepError {
                            error: floxide_core::error::FloxideError::Generic(e.to_string()),
                            run_id: Some(run_id.clone()),
                            work_item: Some(item.clone()),
                        })?;
                    debug!(worker = worker_id, run_id = %run_id, queue_len = cp.queue.len(), "Checkpoint saved (terminal)");
                    return Ok(Some((run_id.clone(), out)));
                }
                Ok(None) => {
                    let mut appended = local_q.clone();
                    for _ in 0..old_tail.len() {
                        let _ = appended.pop_front();
                    }
                    for succ in appended.iter() {
                        queue.enqueue(&run_id, succ.clone())
                            .await
                            .map_err(|e| floxide_core::distributed::StepError {
                                error: floxide_core::error::FloxideError::Generic(e.to_string()),
                                run_id: Some(run_id.clone()),
                                work_item: Some(item.clone()),
                            })?;
                    }
                    cp.context = wf_ctx.store.clone();
                    cp.queue = local_q.clone();
                    store.save(&run_id, &cp)
                        .await
                        .map_err(|e| floxide_core::distributed::StepError {
                            error: floxide_core::error::FloxideError::Generic(e.to_string()),
                            run_id: Some(run_id.clone()),
                            work_item: Some(item.clone()),
                        })?;
                    debug!(worker = worker_id, run_id = %run_id, queue_len = cp.queue.len(), "Checkpoint saved");
                    Ok(None)
                }
                Err(e) => Err(floxide_core::distributed::StepError {
                    error: e,
                    run_id: Some(run_id.clone()),
                    work_item: Some(item.clone()),
                }),
            }
        }
        } // end of impl for distributed API

        #[async_trait::async_trait]
        impl floxide_core::workflow::Workflow<#context> for #name #generics
        {
            type Input = <#start_ty as floxide_core::node::Node<#context>>::Input;
            type Output = <#terminal_ty as floxide_core::node::Node<#context>>::Output;
            type WorkItem = #work_item_ident;

            async fn run<'a>(
                &'a self,
                ctx: &'a floxide_core::WorkflowCtx<#context>,
                input: Self::Input,
            ) -> Result<Self::Output, floxide_core::error::FloxideError> {
                self._run(ctx, input).await
            }

            async fn process_work_item<'a>(
                &'a self,
                ctx: &'a floxide_core::WorkflowCtx<#context>,
                item: #work_item_ident,
                __q: &mut std::collections::VecDeque<#work_item_ident>
            ) -> Result<Option<<#terminal_ty as floxide_core::node::Node<#context>>::Output>, floxide_core::error::FloxideError>           
            {
                self._process_work_item(ctx, item, __q).await
            }

            async fn run_with_checkpoint<CS: floxide_core::checkpoint::CheckpointStore<#context, #work_item_ident> + Send + Sync>(
                &self,
                ctx: &floxide_core::WorkflowCtx<#context>,
                input: Self::Input,
                store: &CS,
                id: &str,
            ) -> Result<Self::Output, floxide_core::error::FloxideError> {
                self._run_with_checkpoint(ctx, input, store, id).await
            }

            async fn resume<CS: floxide_core::checkpoint::CheckpointStore<#context, #work_item_ident> + Send + Sync>(
                &self,
                store: &CS,
                id: &str,
            ) -> Result<Self::Output, floxide_core::error::FloxideError> {
                self._resume(store, id).await
            }

            async fn start_distributed<CS, Q>(
                &self,
                ctx: &floxide_core::WorkflowCtx<#context>,
                input: Self::Input,
                store: &CS,
                queue: &Q,
                id: &str,
            ) -> Result<(), floxide_core::error::FloxideError>
            where
                CS: floxide_core::checkpoint::CheckpointStore<#context, #work_item_ident> + Send + Sync,
                Q: floxide_core::distributed::WorkQueue<#work_item_ident> + Send + Sync,
            {
                self._start_distributed(ctx, input, store, queue, id).await
            }

            async fn step_distributed<CS, Q>(
                &self,
                store: &CS,
                queue: &Q,
                worker_id: usize,
            ) -> Result<Option<(String, Self::Output)>, floxide_core::distributed::StepError<Self::WorkItem>>
            where
                CS: floxide_core::checkpoint::CheckpointStore<#context, #work_item_ident> + Send + Sync,
                Q: floxide_core::distributed::WorkQueue<#work_item_ident> + Send + Sync,
            {
                self._step_distributed(store, queue, worker_id).await
            }

            fn to_dot(&self) -> &'static str {
                self._to_dot()
            }
        }
    };
    TokenStream::from(expanded)
}