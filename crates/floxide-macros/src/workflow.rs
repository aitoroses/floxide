use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, Ident, LitStr,
};
use floxide_macros_support::{WorkflowDef, EdgeKind, CompositeArm};

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
        quote! { #var_ident(String, <#ty as ::floxide::Node<#context>>::Input) }
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

    // Map node field name to the field type (not inner node type)
    let mut node_field_types = std::collections::HashMap::new();
    for (fld, ty, _) in &fields {
        node_field_types.insert(fld.to_string(), ty.clone());
    }
    // For each direct edge, emit a type assertion comparing associated types
    let mut type_asserts = Vec::new();
    for (src, kind) in &edges {
        match kind {
            EdgeKind::Direct { succs, .. } => {
                for succ in succs {
                    let src_ty = node_field_types.get(&src.to_string());
                    let dst_ty = node_field_types.get(&succ.to_string());
                    if let (Some(src_ty), Some(dst_ty)) = (src_ty, dst_ty) {
                        // Generate a compile-time trait-based assertion so errors mention the node names
                        // Generate CamelCase identifiers for trait to satisfy Rust naming conventions
                        let src_camel = to_camel_case(&src.to_string());
                        let dst_camel = to_camel_case(&succ.to_string());
                        let trait_ident = format_ident!(
                            "AssertOutputOf{}MatchesInputOf{}",
                            src_camel,
                            dst_camel
                        );
                        let fn_ident = format_ident!(
                            "assert_equal_{}_to_{}",
                            src,
                            succ
                        );
                        type_asserts.push(quote! {
                            #[doc(hidden)]
                            pub trait #trait_ident<L, R> {}
                            #[doc(hidden)]
                            impl<T> #trait_ident<T, T> for () {}
                            const _: () = {
                            #[allow(dead_code)]
                            #[doc(hidden)]
                            const fn #fn_ident<__Left, __Right>()
                                where
                                    (): #trait_ident<__Left, __Right>,
                                {}
                                #fn_ident::<
                                    <#src_ty as ::floxide::Node<#context>>::Output,
                                    <#dst_ty as ::floxide::Node<#context>>::Input
                                >();
                            };
                        });
                    }
                }
            }
            _ => {}
        }
    }
    let type_errors = quote! { #(#type_asserts)* };

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
                let __node = ::floxide::with_retry(self.#fld.clone(), self.#pol.clone());
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
                let arm_tokens: Vec<proc_macro2::TokenStream> = composite.iter().map(|arm| {
                    let CompositeArm { action_path, variant, binding, is_wildcard, guard, succs } = arm;
                    let pat = if *is_wildcard {
                        let wildcard_ident = format_ident!("__wildcard_binding");
                        if let Some(guard) = &guard {
                            quote! { #action_path :: #variant ( #wildcard_ident ) if #guard }
                        } else {
                            quote! { #action_path :: #variant ( #wildcard_ident ) }
                        }
                    } else if let Some(binding) = &binding {
                        if let Some(guard) = &guard {
                            quote! { #action_path :: #variant ( #binding ) if #guard }
                        } else {
                            quote! { #action_path :: #variant ( #binding ) }
                        }
                    } else {
                        if let Some(guard) = &guard {
                            quote! { #action_path :: #variant if #guard }
                        } else {
                            quote! { #action_path :: #variant }
                        }
                    };
                    // Debug log removed: generated match pattern
                    let body = if succs.is_empty() {
                        if *is_wildcard {
                            let wildcard_ident = format_ident!("__wildcard_binding");
                            quote! {
                                tracing::debug!(variant = stringify!(#variant), value = ?#wildcard_ident, "Composite arm: terminal variant (wildcard)");
                                return Ok(Some(#wildcard_ident));
                            }
                        } else if let Some(binding) = &binding {
                            quote! {
                                tracing::debug!(variant = stringify!(#variant), value = ?#binding, "Composite arm: terminal variant");
                                return Ok(Some(#binding));
                            }
                        } else {
                            quote! {
                                tracing::debug!(variant = stringify!(#variant), "Composite arm: terminal variant (unit)");
                                return Ok(Some(()));
                            }
                        }
                    } else {
                        let succ_pushes = if *is_wildcard {
                            let wildcard_ident = format_ident!("__wildcard_binding");
                            succs.iter().map(|succ| {
                                let var_name = to_camel_case(&succ.to_string());
                                let succ_var = format_ident!("{}", var_name);
                                quote! { __q.push_back(#work_item_ident::#succ_var(::uuid::Uuid::new_v4().to_string(), #wildcard_ident)); }
                            }).collect::<Vec<_>>()
                        } else if let Some(binding) = &binding {
                            succs.iter().map(|succ| {
                                let var_name = to_camel_case(&succ.to_string());
                                let succ_var = format_ident!("{}", var_name);
                                quote! { __q.push_back(#work_item_ident::#succ_var(::uuid::Uuid::new_v4().to_string(), #binding)); }
                            }).collect::<Vec<_>>()
                        } else {
                            succs.iter().map(|succ| {
                                let var_name = to_camel_case(&succ.to_string());
                                let succ_var = format_ident!("{}", var_name);
                                quote! { __q.push_back(#work_item_ident::#succ_var(::uuid::Uuid::new_v4().to_string(), Default::default())); }
                            }).collect::<Vec<_>>()
                        };
                        if *is_wildcard {
                            let wildcard_ident = format_ident!("__wildcard_binding");
                            quote! {
                                tracing::debug!(variant = stringify!(#variant), value = ?#wildcard_ident, "Composite arm: scheduling successors (wildcard)");
                                #(#succ_pushes)*
                                return Ok(None);
                            }
                        } else if let Some(binding) = &binding {
                            quote! {
                                tracing::debug!(variant = stringify!(#variant), value = ?#binding, "Composite arm: scheduling successors");
                                #(#succ_pushes)*
                                return Ok(None);
                            }
                        } else {
                            quote! {
                                tracing::debug!(variant = stringify!(#variant), "Composite arm: scheduling successors (unit)");
                                #(#succ_pushes)*
                                return Ok(None);
                            }
                        }
                    };
                    quote! { #pat => { #body } }
                }).collect();
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
                            match action {
                                #(#arm_tokens)*
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
        #type_errors
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
        impl ::floxide::WorkItem for #work_item_ident {
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
        impl #impl_generics ::floxide::Workflow<#context> for #name #ty_generics #where_clause {
            type Input = <#start_ty as ::floxide::Node<#context>>::Input;
            type Output = <#terminal_ty as ::floxide::Node<#context>>::Output;
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
                ctx: &'a ::floxide::WorkflowCtx<#context>,
                item: Self::WorkItem,
                __q: &mut std::collections::VecDeque<Self::WorkItem>
            ) -> Result<Option<Self::Output>, ::floxide::FloxideError>
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
