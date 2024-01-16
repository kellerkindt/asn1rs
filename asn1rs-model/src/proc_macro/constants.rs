use syn::parenthesized;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, Lit};

#[derive(Debug)]
pub enum ConstLit {
    I64(String, i64),
}

impl Parse for ConstLit {
    fn parse<'a>(input: ParseStream) -> syn::Result<Self> {
        let name = input.parse::<Ident>()?.to_string();
        let content;
        parenthesized!(content in input);
        let value = content.parse::<Lit>()?;
        match value {
            Lit::Int(int) => Ok(ConstLit::I64(
                name,
                int.base10_digits()
                    .parse()
                    .map_err(|_| syn::Error::new(int.span(), "Not an integer"))?,
            )),
            _ => Err(syn::Error::new(value.span(), "Unsupported literal")),
        }
    }
}
