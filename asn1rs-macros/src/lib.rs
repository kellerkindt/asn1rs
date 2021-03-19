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
    asn_to_rust_fn(&input).parse().unwrap()
}

fn asn_to_rust_fn(input: &str) -> String {
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

    output
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

#[cfg(test)]
mod proc_macro_tarpaulin_coverage_hack {
    use super::asn_to_rust_fn;
    use old_proc_macro2;
    use old_syn;
    use std::path::PathBuf;
    use std::str::FromStr;
    use std::{env, fs};

    fn asn_to_rust_fn2(input: old_proc_macro2::TokenStream) -> old_proc_macro2::TokenStream {
        let input = old_syn::parse2::<old_syn::LitStr>(input.into()).unwrap();
        let result = asn_to_rust_fn(&input.value());
        old_proc_macro2::TokenStream::from_str(&result).unwrap()
    }

    #[test]
    fn cover_asn_to_rust_macro() {
        // This code doesn't check much. Instead, it does macro expansion at run time to let
        // tarpaulin measure code coverage for the macro.
        let mut path = env::current_dir().unwrap();
        path.pop();
        path.push("tests");
        walk_dir(path);
    }

    fn walk_dir(path: PathBuf) {
        for entry in fs::read_dir(path).unwrap() {
            match entry {
                Ok(entry) if entry.file_type().unwrap().is_dir() => {
                    walk_dir(entry.path());
                }
                Ok(entry) if entry.file_type().unwrap().is_file() => {
                    println!("{:?}", entry.path());
                    let file = fs::File::open(entry.path()).unwrap();
                    runtime_macros::emulate_macro_expansion_fallible(
                        file,
                        "asn_to_rust",
                        asn_to_rust_fn2,
                    )
                    .unwrap();
                }
                _ => {}
            }
        }
    }
}
