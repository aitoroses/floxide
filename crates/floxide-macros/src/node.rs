use proc_macro::TokenStream;
use quote::quote;
use syn::{
    braced, parse_macro_input, Token
};

pub fn node(item: TokenStream) -> TokenStream {
    use syn::{parse_macro_input, Visibility, Ident, Type, Token, braced};
    use syn::parse::{Parse, ParseStream};
    use quote::quote;

    struct NodeDef {
        vis: Visibility,
        name: Ident,
        fields: Vec<(Ident, Type)>,
        ctx_ty: Type,
        input_ty: Type,
        output_ty: Type,
        ctx_arg: Ident,
        input_arg: Ident,
        body: syn::Block,
    }
    impl Parse for NodeDef {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            let vis: Visibility = input.parse()?;
            input.parse::<Token![struct]>()?;
            let name: Ident = input.parse()?;
            let content;
            braced!(content in input);
            let mut fields = Vec::new();
            while !content.is_empty() {
                let f: Ident = content.parse()?;
                content.parse::<Token![:]>()?;
                let ty: Type = content.parse()?;
                if content.peek(Token![,]) {
                    content.parse::<Token![,]>()?;
                }
                fields.push((f, ty));
            }
            input.parse::<Token![;]>()?;
            // context = MyCtx;
            // parse `context = Type;`
            let ctx_kw: Ident = input.parse()?;
            if ctx_kw != "context" {
                return Err(input.error("expected `context = Type;`"));
            }
            input.parse::<Token![=]>()?;
            let ctx_ty: Type = input.parse()?;
            input.parse::<Token![;]>()?;
            // input = InputType;
            // parse `input = Type;`
            let in_kw: Ident = input.parse()?;
            if in_kw != "input" {
                return Err(input.error("expected `input = Type;`"));
            }
            input.parse::<Token![=]>()?;
            let input_ty: Type = input.parse()?;
            input.parse::<Token![;]>()?;
            // output = OutputType;
            // parse `output = Type;`
            let out_kw: Ident = input.parse()?;
            if out_kw != "output" {
                return Err(input.error("expected `output = Type;`"));
            }
            input.parse::<Token![=]>()?;
            let output_ty: Type = input.parse()?;
            input.parse::<Token![;]>()?;
            // closure args
            input.parse::<Token![|]>()?;
            let ctx_arg: Ident = input.parse()?;
            input.parse::<Token![,]>()?;
            let input_arg: Ident = input.parse()?;
            input.parse::<Token![|]>()?;
            let body: syn::Block = input.parse()?;
            Ok(NodeDef { vis, name, fields, ctx_ty, input_ty, output_ty, ctx_arg, input_arg, body })
        }
    }
    let NodeDef { vis, name, fields, ctx_ty, input_ty, output_ty, ctx_arg, input_arg, body } =
        parse_macro_input!(item as NodeDef);
    let field_defs = fields.iter().map(|(f, ty)| quote!{ pub #f: #ty });
    let struct_def = quote!{ #[derive(Clone, Debug)] #vis struct #name { #(#field_defs),* } };
    let expanded = quote!{
        #struct_def
        #[::async_trait::async_trait]
        impl ::floxide_core::node::Node<#ctx_ty> for #name
        where
            #ctx_ty: Clone + Send + Sync + 'static
        {
            type Input = #input_ty;
            type Output = #output_ty;

            async fn process(
                &self,
                #ctx_arg: &#ctx_ty,
                #input_arg: #input_ty
            ) -> Result<::floxide_core::transition::Transition<#output_ty>, ::floxide_core::error::FloxideError>
            {
                let #ctx_arg = #ctx_arg;
                let #input_arg = #input_arg;
                #body
            }
        }
    };
    TokenStream::from(expanded)
}