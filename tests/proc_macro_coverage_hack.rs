use asn1rs::ast::asn_to_rust;
use old_proc_macro2;
use old_syn;
use std::path::PathBuf;
use std::str::FromStr;
use std::{env, fs};

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

fn asn_to_rust_fn2(input: old_proc_macro2::TokenStream) -> old_proc_macro2::TokenStream {
    let input = old_syn::parse2::<old_syn::LitStr>(input.into()).unwrap();
    let result = asn_to_rust(&input.value());
    old_proc_macro2::TokenStream::from_str(&result).unwrap()
}
