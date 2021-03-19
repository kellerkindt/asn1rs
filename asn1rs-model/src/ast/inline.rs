use crate::gen::rust::RustCodeGenerator as RustGenerator;
use crate::gen::Generator;
use crate::model::Model;
use crate::parser::Tokenizer;

pub fn asn_to_rust(input: &str) -> String {
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
