use asn1rs_model::ast;
use proc_macro::TokenStream;
use syn::parse_macro_input;
use syn::DeriveInput;
use syn::LitStr;

mod derive_protobuf_eq;

#[proc_macro]
pub fn asn_to_rust(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as LitStr).value();
    asn1rs_model::ast::asn_to_rust(&input).parse().unwrap()
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
