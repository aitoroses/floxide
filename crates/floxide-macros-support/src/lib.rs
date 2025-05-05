pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}

/// AST and parsing logic for floxide-macros

use syn::{parse::{Parse, ParseStream}, braced, bracketed, Generics, Ident, Result, Token, Type, Visibility};

#[derive(Debug, Clone)]
pub struct CompositeArm {
    pub action_path: Ident,
    pub variant: Ident,
    pub binding: Option<Ident>,
    pub is_wildcard: bool,
    pub guard: Option<syn::Expr>,
    pub succs: Vec<Ident>,
}

#[derive(Debug, Clone)]
pub enum EdgeKind {
    Direct {
        succs: Vec<Ident>,
        on_failure: Option<Vec<Ident>>,
    },
    Composite(Vec<CompositeArm>),
}

#[derive(Debug, Clone)]
pub struct WorkflowDef {
    pub vis: Visibility,
    pub name: Ident,
    pub generics: Generics,
    pub fields: Vec<(Ident, Type, Option<Ident>)>,
    pub start: Ident,
    pub context: Type,
    pub edges: Vec<(Ident, EdgeKind)>,
}

// Parsing logic for WorkflowDef (copy from floxide-macros/src/workflow.rs, adapted to use these types)
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
                                    return Err(edges_content.error(
                                        "Unexpected identifier. Expected `on_failure` or `=>`."
                                    ));
                                }
                            }
                            // success or composite entry
                            edges_content.parse::<Token![=>]>()?;
                            if edges_content.peek(syn::token::Bracket) {
                                // direct edge: foo => [bar];
                                let succs_content;
                                bracketed!(succs_content in edges_content);
                                let succs: Vec<Ident> = succs_content
                                    .parse_terminated(Ident::parse, Token![,])?
                                    .into_iter()
                                    .collect();
                                edges_content.parse::<Token![;]>()?;
                                direct_success.insert(src.clone(), succs);
                            } else if edges_content.peek(syn::token::Brace) {
                                // composite or legacy direct edge: foo => { ... };
                                let nested;
                                braced!(nested in edges_content);
                                if nested.peek(syn::token::Bracket) {
                                    // legacy: foo => {[bar]};
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
                                        let mut binding = None;
                                        let mut is_wildcard = false;
                                        if nested.peek(syn::token::Paren) {
                                            let inner;
                                            syn::parenthesized!(inner in nested);
                                            if inner.peek(Token![_]) {
                                                inner.parse::<Token![_]>()?;
                                                is_wildcard = true;
                                            } else if inner.peek(Ident) {
                                                binding = Some(inner.parse()?);
                                            } else {
                                                return Err(inner.error("Expected identifier or _ in variant binding"));
                                            }
                                        } else {
                                            // No parens: always treat as wildcard for macro ergonomics
                                            is_wildcard = true;
                                            binding = None;
                                        }
                                        // Optional guard
                                        let guard = if nested.peek(Token![if]) {
                                            nested.parse::<Token![if]>()?;
                                            Some(nested.parse()?)
                                        } else {
                                            None
                                        };
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
                                            is_wildcard,
                                            guard,
                                            succs,
                                        });
                                    }
                                    edges_content.parse::<Token![;]>()?;
                                    composite_map.insert(src.clone(), arms);
                                }
                            } else {
                                return Err(edges_content.error("Expected [ or { after => in edge definition"));
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
