use asn1rs::ast::asn_to_rust;
use proc_macro2::TokenStream;
use quote::ToTokens;
use std::io::Read;
use std::panic::AssertUnwindSafe;
use std::path::PathBuf;
use std::str::FromStr;
use std::{env, fs, panic};
use syn::Attribute;
use syn::ItemEnum;
use syn::ItemStruct;

#[test]
fn cover_asn_to_rust_macro() {
    walk_dir({
        // This code doesn't check much. Instead, it does macro expansion at run time to let
        // tarpaulin measure code coverage for the macro.
        let mut path = env::current_dir().unwrap();
        path.push("tests");
        path
    });
}

fn walk_dir(path: PathBuf) {
    for entry in fs::read_dir(path).unwrap() {
        match entry {
            Ok(entry) if entry.file_type().unwrap().is_dir() => {
                walk_dir(entry.path());
            }
            Ok(entry)
                if entry.file_type().unwrap().is_file()
                    && entry.path().to_str().map_or(false, |s| s.ends_with(".rs")) =>
            {
                println!("Feeding {:?}", entry.path());
                let file = fs::File::open(entry.path()).unwrap();
                emulate_macro_expansion_fallible(file);
            }
            _ => {}
        }
    }
}

/// Based on https://github.com/jeremydavis519/runtime-macros/blob/master/src/lib.rs
///
///
/// This awfully great hack allows tarpaulin to track the proc-macro related function
/// calls for a better line coverage. It manually parses each test file, takes the literal
/// String of the 'asn_to_rust!()' macro and generates the models and then rust code for it.
/// It then takes this intermediate result and feeds it to the proc-attribute expander to
/// also track it.
///
/// WARNING: This does *NO* logic check and does *NOT* unit tests. It just helps to track the
///          called functions and executed lines.
///
pub fn emulate_macro_expansion_fallible(mut file: fs::File) {
    fn ast_parse_str(attr: &str, item: &str) -> TokenStream {
        asn1rs::ast::parse(
            TokenStream::from_str(attr).unwrap(),
            TokenStream::from_str(item).unwrap(),
        )
    }

    fn asn_to_rust_fn2(input: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        let input = syn::parse2::<syn::LitStr>(input).unwrap();
        let result = asn_to_rust(&input.value());
        proc_macro2::TokenStream::from_str(&result).unwrap()
    }

    fn feed_derive_parser(
        attributes: &[Attribute],
        attribute_path: &syn::Path,
        item: impl Fn() -> String,
        body_start_marker: &str,
    ) {
        for attr in attributes {
            if attr.path == *attribute_path {
                let attribute_meta = attr.parse_meta().unwrap();
                let attribute_meta = attribute_meta.into_token_stream().to_string();

                let item = item();
                // skip 'asn (' and ')'
                let start = attribute_meta.find('(').unwrap();
                let end = attribute_meta.rfind(')').unwrap();
                let header = &attribute_meta[start + 1..end];
                let body = {
                    let body_start = item.find(body_start_marker).unwrap();
                    &item[body_start..]
                };

                if cfg!(feature = "debug-proc-macro") {
                    println!("##########: {}", item);
                    println!("      meta: {}", attribute_meta);
                    println!("    header: {}", header);
                    println!("      body: {}", body);
                    println!();
                }

                let result = ast_parse_str(header, body).to_string();

                if result.contains("compile_error") {
                    panic!("{}", result);
                } else {
                    syn::parse_str::<proc_macro2::TokenStream>(&result).expect("Result is invalid");
                }
                break;
            }
        }
    }

    struct MacroVisitor {
        macro_path: syn::Path,
        attribute_path: syn::Path,
    }
    impl<'ast> syn::visit::Visit<'ast> for MacroVisitor {
        fn visit_item_enum(&mut self, i: &'ast ItemEnum) {
            feed_derive_parser(
                &i.attrs[..],
                &self.attribute_path,
                || i.into_token_stream().to_string(),
                " pub enum ",
            );
        }

        fn visit_item_struct(&mut self, i: &'ast ItemStruct) {
            feed_derive_parser(
                &i.attrs[..],
                &self.attribute_path,
                || i.into_token_stream().to_string(),
                " pub struct ",
            );
        }

        fn visit_macro(&mut self, macro_item: &'ast syn::Macro) {
            if macro_item.path == self.macro_path {
                let result = asn_to_rust_fn2(macro_item.tokens.clone());
                let ast = AssertUnwindSafe(syn::parse_file(&result.to_string()).unwrap());
                syn::visit::visit_file(self, &*ast);
            }
        }
    }

    let mut content = String::new();
    file.read_to_string(&mut content).unwrap();

    syn::visit::visit_file(
        &mut MacroVisitor {
            macro_path: syn::parse_str("asn_to_rust").unwrap(),
            attribute_path: syn::parse_str("asn").unwrap(),
        },
        &syn::parse_file(content.as_str()).unwrap(),
    )
}
