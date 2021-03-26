use syn::buffer::Cursor;
use syn::parse::{Parse, ParseStream};
use syn::Ident;
use syn::Lit;
use syn::Token;

#[derive(Debug)]
#[allow(clippy::upper_case_acronyms)]
enum MMV {
    MinMax,
    Value(i64),
}

impl MMV {
    pub fn try_parse(input: ParseStream) -> syn::Result<Option<Self>> {
        if let Ok(Lit::Int(int)) = input.parse::<Lit>() {
            Ok(Some(MMV::Value(
                int.base10_digits()
                    .parse::<i64>()
                    .map_err(|_| input.error("Expected int literal for from value of range"))?,
            )))
        } else if let Ok(ident) = input.parse::<Ident>() {
            let lc = ident.to_string().to_lowercase();
            if lc == "min" || lc == "max" {
                Ok(Some(MMV::MinMax))
            } else {
                Err(input.error(format!(
                    "Invalid identifier, accepted identifiers are: min, max but got: {}",
                    lc
                )))
            }
        } else {
            Err(input.error("Cannot parse token"))
        }
    }
}

#[derive(Debug)]
pub struct IntegerRange(pub Option<(i64, i64)>, pub bool);

impl Parse for IntegerRange {
    fn parse<'a>(input: ParseStream) -> syn::Result<Self> {
        let min = MMV::try_parse(input)?.ok_or_else(|| input.error("invalid min"))?;
        let _ = input.parse::<Token![.]>()?;
        let _ = input.parse::<Token![.]>()?;
        let max = MMV::try_parse(input)?.ok_or_else(|| input.error("invalid max"))?;
        let extensible = if input.peek(Token![,]) {
            let _ = input.parse::<Token![,]>()?;
            let _ = input.parse::<Token![.]>()?;
            let _ = input.parse::<Token![.]>()?;
            let _ = input.parse::<Token![.]>()?;
            true
        } else {
            false
        };

        match (min, max) {
            (MMV::MinMax, MMV::MinMax) | (MMV::Value(0), MMV::MinMax) => {
                Ok(IntegerRange(None, extensible))
            }
            (MMV::Value(min), MMV::Value(max)) => Ok(IntegerRange(Some((min, max)), extensible)),
            (MMV::MinMax, MMV::Value(max)) => Ok(IntegerRange(
                Some((
                    if max.is_positive() {
                        0
                    } else {
                        i64::max_value().wrapping_add(1)
                    },
                    max,
                )),
                extensible,
            )),
            (MMV::Value(min), MMV::MinMax) => {
                Ok(IntegerRange(Some((min, i64::max_value())), extensible))
            }
        }
    }
}

pub fn ident_or_literal_or_punct(a: Cursor<'_>) -> Option<(String, Cursor<'_>)> {
    a.ident()
        .map(|(a, b)| (a.to_string(), b))
        .or_else(|| a.literal().map(|(a, b)| (a.to_string(), b)))
        .or_else(|| a.punct().map(|(a, b)| (a.to_string(), b)))
}
