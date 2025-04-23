use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    braced, bracketed, parse::{Parse, ParseStream}, parse_macro_input, Generics, Ident, Result, Token, Type, Visibility
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
        // Variant name: capitalized field name
        let fld_str = fld.to_string();
        let mut chars = fld_str.chars();
        let first = chars.next().unwrap().to_uppercase().to_string();
        let rest: String = chars.collect();
        let var_ident = format_ident!("{}{}", first, rest);
        quote! { #var_ident(<#ty as floxide_core::node::Node<#context>>::Input) }
    });
    // Compute the WorkItem variant name for the terminal field
    let terminal_var = {
        let s = terminal_src.to_string();
        let mut cs = s.chars();
        let first = cs.next().unwrap().to_uppercase().to_string();
        let rest: String = cs.collect();
        format_ident!("{}{}", first, rest)
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
        let fld_str = fld.to_string();
        let mut chars = fld_str.chars();
        let first = chars.next().unwrap().to_uppercase().to_string();
        let rest: String = chars.collect();
        let var_ident = format_ident!("{}{}", first, rest);
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
                    quote! { return Ok(action) }
                } else {
                    let pushes = succs.iter().map(|succ| {
                        let succ_var = format_ident!("{}{}",
                            succ.to_string().chars().next().unwrap().to_uppercase().to_string(),
                            succ.to_string().chars().skip(1).collect::<String>()
                        );
                        quote! { __q.push_back(#work_item_ident::#succ_var(action.clone())); }
                    });
                    quote! { #(#pushes)* }
                };
                // Build tokens for failure/fallback path
                let push_failure = if let Some(fails) = on_failure {
                    let failure_pushes = fails.iter().map(|succ| {
                        let succ_var = format_ident!("{}{}",
                            succ.to_string().chars().next().unwrap().to_uppercase().to_string(),
                            succ.to_string().chars().skip(1).collect::<String>()
                        );
                        // fallback receives no input (unit)
                        quote! { __q.push_back(#work_item_ident::#succ_var(())); }
                    });
                    quote! { #(#failure_pushes)* }
                } else {
                    quote! { return Err(e) }
                };
                // Generate match with explicit error handling
                quote! {
                    #wrapper
                    let __store = &ctx.store;
                    match ctx.run_future(__node.process(__store, x)).await {
                        Ok(Transition::Next(action)) => { #push_success },
                        Ok(Transition::Abort(e)) | Err(e) => { #push_failure },
                    }
                }
            }
            EdgeKind::Composite(composite) => {
                if composite.is_empty() {
                    // terminal composite branch: return the output value
                    quote! {
                        #wrapper
                        let __store = &ctx.store;
                        match ctx.run_future(__node.process(__store, x)).await? {
                            Transition::Next(action) => return Ok(action),
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
                            quote! { __q.push_back(#work_item_ident::#succ_var(#binding)); }
                        });
                        if succs.is_empty() {
                            quote! { #pat => { return Ok(#binding); } }
                        } else {
                            quote! { #pat => { #(#succ_pushes)* } }
                        }
                    });
                    quote! {
                        #wrapper
                        let __store = &ctx.store;
                        match ctx.run_future(__node.process(__store, x)).await? {
                            Transition::Next(action) => {
                                match action { #(#pats),* _ => {} }
                            }
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
    }).collect();

    // Start variant
    let start_var = {
        let s = start.to_string();
        let mut cs = s.chars();
        let first = cs.next().unwrap().to_uppercase().to_string();
        let rest: String = cs.collect();
        format_ident!("{}{}", first, rest)
    };
    // Start field type for Input
    let start_ty = fields.iter().find(|(fld,_,_)| fld == &start)
        .map(|(_, ty, _)| ty).expect("start field not found");

    // Assemble the expanded code
    let expanded = quote! {

        #[derive(Debug, Clone)]
        #struct_def

        #[allow(non_camel_case_types)]
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        enum #work_item_ident {
            #(#work_variants),*
        }


        impl #name #generics {
            /// Execute the workflow, returning the output of the terminal branch
            #[allow(unreachable_code)]
            pub async fn run(
                &self,
                ctx: &floxide_core::WorkflowCtx<#context>,
                input: <#start_ty as floxide_core::node::Node<#context>>::Input,
            ) -> Result<<#terminal_ty as floxide_core::node::Node<#context>>::Output, floxide_core::error::FloxideError>
            {
                use std::collections::VecDeque;
                use floxide_core::transition::Transition;
                let mut __q = VecDeque::new();
                __q.push_back(#work_item_ident::#start_var(input));
                while let Some(item) = __q.pop_front() {
                    match item {
                        #(#run_arms),*
                    }
                }
                unreachable!("Workflow did not reach terminal branch");
            }

            #[allow(unreachable_code)]
            /// Execute the workflow, checkpointing state after each step.
            pub async fn run_with_checkpoint<CS: floxide_core::checkpoint::CheckpointStore>(
                &self,
                ctx: &floxide_core::WorkflowCtx<#context>,
                input: <#start_ty as floxide_core::node::Node<#context>>::Input,
                store: &CS,
                id: &str,
            ) -> Result<<#terminal_ty as floxide_core::node::Node<#context>>::Output, floxide_core::error::FloxideError>
            {
                // Fully qualify Checkpoint to avoid import collisions
                use std::collections::VecDeque;
                use floxide_core::{Checkpoint, CheckpointStore};

                // load existing checkpoint or start new
                let mut cp: floxide_core::Checkpoint<#context, #work_item_ident> = match store.load(id)
                    .map_err(|e| floxide_core::error::FloxideError::Generic(e.to_string()))? {
                    Some(bytes) => floxide_core::Checkpoint::from_bytes(&bytes)
                        .map_err(|e| floxide_core::error::FloxideError::Generic(e.to_string()))?,
                    None => {
                        let mut q = VecDeque::new();
                        q.push_back(#work_item_ident::#start_var(input));
                        floxide_core::Checkpoint { context: ctx.store.clone(), queue: q }
                    }
                };
                // initialize working queue from checkpoint
                let mut __q = cp.queue.clone();
                // if there's no pending work, the workflow has already completed
                if __q.is_empty() {
                    return Err(floxide_core::error::FloxideError::AlreadyCompleted);
                }
                // process loop with persistence
                while let Some(item) = __q.pop_front() {
                    match item {
                        #(#run_arms),*
                    }
                    // update checkpoint state and persist
                    cp.queue = __q.clone();
                    let bytes = cp.to_bytes()
                        .map_err(|e| floxide_core::error::FloxideError::Generic(e.to_string()))?;
                    store.save(id, &bytes)
                        .map_err(|e| floxide_core::error::FloxideError::Generic(e.to_string()))?;
                }
                unreachable!("Workflow did not reach terminal branch");
            }

            #[allow(unreachable_code)]
            /// Resume a workflow run from its last checkpoint; context and queue are restored from store.
            pub async fn resume<CS: floxide_core::checkpoint::CheckpointStore>(
                &self,
                store: &CS,
                id: &str,
            ) -> Result<<#terminal_ty as floxide_core::node::Node<#context>>::Output, floxide_core::error::FloxideError>
            {
                use std::collections::VecDeque;
                // Load persisted checkpoint or error if never run
                let bytes = store.load(id)
                    .map_err(|e| floxide_core::error::FloxideError::Generic(e.to_string()))?
                    .ok_or_else(|| floxide_core::error::FloxideError::NotStarted)?;
                let cp: floxide_core::Checkpoint<#context, #work_item_ident> =
                    floxide_core::Checkpoint::from_bytes(&bytes)
                        .map_err(|e| floxide_core::error::FloxideError::Generic(e.to_string()))?;
                // Rebuild WorkflowCtx from saved context
                let wf_ctx = floxide_core::WorkflowCtx::new(cp.context.clone());
                let ctx = &wf_ctx;
                // Restore work queue from checkpoint
                let mut __q: VecDeque<#work_item_ident> = cp.queue.clone();
                // If there is no pending work, the workflow has already completed
                if __q.is_empty() {
                    return Err(floxide_core::error::FloxideError::AlreadyCompleted);
                }
                // if the only pending work is the terminal node, treat as completed
                if __q.len() == 1 {
                    if let #work_item_ident::#terminal_var(_) = __q.front().unwrap() {
                        return Err(floxide_core::error::FloxideError::AlreadyCompleted);
                    }
                }
                // Process remaining items
                while let Some(item) = __q.pop_front() {
                    match item {
                        #(#run_arms),*
                    }
                }
                unreachable!("Workflow did not reach terminal branch");
            }
        }

        #[async_trait::async_trait]
        impl floxide_core::workflow::Workflow<#context> for #name #generics
        {
            type Input = <#start_ty as floxide_core::node::Node<#context>>::Input;
            type Output = <#terminal_ty as floxide_core::node::Node<#context>>::Output;

            async fn run(
                &self,
                ctx: &#context,
                input: Self::Input,
            ) -> Result<Self::Output, floxide_core::error::FloxideError> {
                // build a full WorkflowCtx<C> from the store
                let wf_ctx = floxide_core::WorkflowCtx::new(ctx.clone());
                // delegate to the inherent run method
                self.run(&wf_ctx, input).await
            }
        }
    };
    TokenStream::from(expanded)
}