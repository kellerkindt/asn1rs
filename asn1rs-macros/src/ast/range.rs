use proc_macro2::Delimiter;
use syn::parse::{Parse, ParseBuffer};

pub struct MaybeRanged(pub Option<(i64, i64)>);

impl Parse for MaybeRanged {
    fn parse<'a>(input: &'a ParseBuffer<'a>) -> syn::Result<Self> {
        if input.peek(syn::token::Paren) {
            input.step(|stepper| {
                let (a, _span, outer) = stepper
                    .group(Delimiter::Parenthesis)
                    .ok_or_else(|| stepper.error("Expected range"))?;

                let (min, c) = a
                    .ident()
                    .map(|(a, b)| (a.to_string(), b))
                    .or_else(|| a.literal().map(|(a, b)| (a.to_string(), b)))
                    .ok_or_else(|| stepper.error("Expected min value"))?;

                let (_, c) = c.punct().ok_or_else(|| stepper.error("Expected dot"))?;
                let (_, c) = c.punct().ok_or_else(|| stepper.error("Expected dot"))?;

                let (max, _c) = c
                    .ident()
                    .map(|(a, b)| (a.to_string(), b))
                    .or_else(|| c.literal().map(|(a, b)| (a.to_string(), b)))
                    .ok_or_else(|| stepper.error("Expected max value"))?;

                let min = min.to_lowercase();
                let max = max.to_lowercase();

                if min == "min" && max == "max" {
                    Ok((MaybeRanged(None), outer))
                } else {
                    let min = min.parse::<i64>().map_err(|_| stepper.error("Not i64"))?;
                    let max = max.parse::<i64>().map_err(|_| stepper.error("Not i64"))?;
                    Ok((MaybeRanged(Some((min, max))), outer))
                }
            })
        } else {
            Ok(MaybeRanged(None))
        }
    }
}
