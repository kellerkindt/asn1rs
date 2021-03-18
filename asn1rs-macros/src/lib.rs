extern crate proc_macro;

use asn1rs_model::ast;

use asn1rs_model::gen::rust::RustCodeGenerator as RustGenerator;
use asn1rs_model::gen::Generator;
use asn1rs_model::model::Model;
use asn1rs_model::parser::Tokenizer;
use proc_macro::TokenStream;
use syn::parse_macro_input;
use syn::DeriveInput;
use syn::LitStr;

mod derive_protobuf_eq;

#[proc_macro]
pub fn asn_to_rust(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as LitStr).value();
    let tokens = Tokenizer::default().parse(&input);
    let model = Model::try_from(tokens)
        .expect("Failed to parse tokens")
        .try_resolve()
        .expect("Failed to resolve value references");

    let mut generator = RustGenerator::default();
    generator.add_model(model.to_rust());

    let output = generator
        .to_string()
        .unwrap()
        .into_iter()
        .map(|(_file, content)| content)
        .collect::<Vec<_>>()
        .join("\n");

    if cfg!(feature = "debug-proc-macro") {
        println!("-------- output start");
        println!("{}", output);
        println!("-------- output end");
    }

    output.parse().unwrap()
}

#[proc_macro_attribute]
pub fn asn(attr: TokenStream, item: TokenStream) -> TokenStream {
    TokenStream::from(ast::parse(attr.into(), item.into()))
}

#[proc_macro_derive(ProtobufEq)]
pub fn protobuf_eq(input: TokenStream) -> TokenStream {
    let output = derive_protobuf_eq::expand(parse_macro_input!(input as DeriveInput));

    if cfg!(feature = "debug-proc-macro") {
        println!("-------- output start");
        println!("{}", output);
        println!("-------- output end");
    }

    output
}
