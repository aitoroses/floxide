use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

/// Entry point for the `#[derive(Merge)]` macro.
pub fn derive(input: TokenStream) -> TokenStream {
    let input_ast = parse_macro_input!(input as DeriveInput);
    // Clone the type name to avoid moving it out of the AST
    let name = input_ast.ident.clone();
    // Prepare generics for impl
    let (impl_generics, ty_generics, where_clause) = input_ast.generics.split_for_impl();

    // Build merge implementation by iterating over named fields
    let merge_body = match &input_ast.data {
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(fields_named) => {
                let stmts = fields_named.named.iter().map(|f| {
                    let ident = f
                        .ident
                        .as_ref()
                        .expect("Named field always has an identifier");
                    quote! { self.#ident.merge(other.#ident); }
                });
                quote! { #(#stmts)* }
            }
            _ => {
                return syn::Error::new_spanned(
                    &input_ast,
                    "Merge derive only supports structs with named fields",
                )
                .to_compile_error()
                .into();
            }
        },
        _ => {
            return syn::Error::new_spanned(
                &input_ast,
                "Merge derive can only be applied to structs",
            )
            .to_compile_error()
            .into();
        }
    };

    // Generate the implementation
    let expanded = quote! {
        impl #impl_generics Merge for #name #ty_generics #where_clause {
            fn merge(&mut self, other: Self) {
                #merge_body
            }
        }
    };
    TokenStream::from(expanded)
}
