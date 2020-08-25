use crate::model::Size;
use syn::parse::{Parse, ParseStream};
use syn::Ident;
use syn::Lit;
use syn::Token;

impl Parse for Size {
    fn parse<'a>(input: ParseStream) -> syn::Result<Self> {
        let min = value(input)?.ok_or_else(|| input.error("invalid min"))?;
        if input.peek(syn::token::Paren) {
            Ok(Size::Fix(min, false))
        } else if input.peek(Token![,]) {
            let _ = input.parse::<Token![,]>()?;
            let _ = input.parse::<Token![.]>()?;
            let _ = input.parse::<Token![.]>()?;
            let _ = input.parse::<Token![.]>()?;
            Ok(Size::Fix(min, true))
        } else {
            let _ = input.parse::<Token![.]>()?;
            let _ = input.parse::<Token![.]>()?;
            let max = value(input)?.ok_or_else(|| input.error("invalid max"))?;
            let extensible = if input.peek(Token![,]) {
                let _ = input.parse::<Token![,]>()?;
                let _ = input.parse::<Token![.]>()?;
                let _ = input.parse::<Token![.]>()?;
                let _ = input.parse::<Token![.]>()?;
                true
            } else {
                false
            };

            if min == max {
                Ok(Size::Fix(min, extensible))
            } else {
                Ok(Size::Range(min, max, extensible))
            }
        }
    }
}

fn value(input: ParseStream) -> syn::Result<Option<usize>> {
    if let Ok(Lit::Int(int)) = input.parse::<Lit>() {
        Ok(Some(int.base10_digits().parse::<usize>().map_err(
            |_| input.error("Expected non-negative int literal"),
        )?))
    } else if let Ok(ident) = input.parse::<Ident>() {
        let lc = ident.to_string().to_lowercase();
        if lc == "min" || lc == "max" {
            Ok(None)
        } else {
            Err(input.error("Invalid identifier, accepted identifiers are: min, max"))
        }
    } else {
        Err(input.error("Cannot parse token"))
    }
}
