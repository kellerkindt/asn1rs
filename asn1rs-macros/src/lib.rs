extern crate proc_macro;
extern crate proc_macro2;

use asn1rs_model::gen::rust::RustCodeGenerator as RustGenerator;
use asn1rs_model::gen::Generator;
use asn1rs_model::model::Model;
use asn1rs_model::parser::Tokenizer;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, AttributeArgs};
use syn::{Item, LitStr};

#[proc_macro]
pub fn asn_to_rust(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as LitStr).value();
    let tokens = Tokenizer::default().parse(&input);
    let model = Model::try_from(tokens).unwrap();

    let mut generator = RustGenerator::default();
    generator.add_model(model.to_rust());

    generator
        .to_string()
        .unwrap()
        .into_iter()
        .map(|(_file, content)| content)
        .collect::<Vec<_>>()
        .join("\n")
        .parse()
        .unwrap()
}

#[proc_macro_attribute]
pub fn asn(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attributes = parse_macro_input!(attr as AttributeArgs);
    let item = parse_macro_input!(item as Item);

    let item = match item {
        Item::Struct(mut strct) => {
            for field in strct.fields.iter_mut() {
                field
                    .attrs
                    .retain(|a| !a.path.segments.first().unwrap().ident.to_string().eq("asn"));
            }
            Item::Struct(strct)
        }
        item => item,
    };

    TokenStream::from(quote! {#item})
}
