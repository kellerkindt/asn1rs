use crate::generator::rust::RustCodeGenerator as RustGenerator;
use crate::generator::Generator;
use crate::model::Model;
use crate::parser::Tokenizer;

pub fn asn_to_rust(input: &str) -> String {
    let tokens = Tokenizer.parse(input);
    let model = Model::try_from(tokens)
        .expect("Failed to parse tokens")
        .try_resolve()
        .expect("Failed to resolve value references");

    let output = RustGenerator::from(model.to_rust())
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
