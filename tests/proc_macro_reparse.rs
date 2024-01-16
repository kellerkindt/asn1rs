//! The Tests in this module ensure that the generated rust code and its asn attributes carry all
//! the data back into the rust model. It does not cover whether all attributes from the parsed
//! ASN-Model is reflected into the Rust-Model.
//!
//! Summed up, the tests check whether the RustCodeGen path serializes all data and whether
//! the proc-macro path is the proper inverse of it - whether it deserializes all data.
//!
//!  ASN-Definition ----> ASN-Model
//!                          |
//!                          V
//!                      Rust-Model   ---> RustCodeGen  ---+
//!                          A                             V
//!                          | eq!              Rust-Code and ASN-Attributes
//!                          V                             |
//!                      Rust-Model   <--- proc-macro  <---+                                                     

use asn1rs::model::model::{Definition, Model, Rust};
use asn1rs_model::generators::RustCodeGenerator;
use asn1rs_model::parser::Tokenizer;
use codegen::Scope;
use proc_macro2::TokenStream;

#[test]
fn test_standard_enum() {
    parse_asn_map_to_rust_map_to_stringify_with_proc_macro_annotation_re_parse_check_equal(
        r#"BasicSchema DEFINITIONS AUTOMATIC TAGS ::= BEGIN

  MyType ::= [UNIVERSAL 5] ENUMERATED {
    implicit,
    number(7),
    wow
  }
  
END"#,
    )
}

#[test]
fn test_extensible_enum() {
    parse_asn_map_to_rust_map_to_stringify_with_proc_macro_annotation_re_parse_check_equal(
        r#"BasicSchema DEFINITIONS AUTOMATIC TAGS ::= BEGIN

  MyType ::= [UNIVERSAL 5] ENUMERATED {
    implicit,
    number(7),
    ...,
    wow
  }
  
END"#,
    )
}

#[test]
fn test_standard_choice() {
    parse_asn_map_to_rust_map_to_stringify_with_proc_macro_annotation_re_parse_check_equal(
        r#"BasicSchema DEFINITIONS AUTOMATIC TAGS ::= BEGIN

  MyType ::= [PRIVATE 1] CHOICE {
    abc UTF8String,
    def [APPLICATION 7] INTEGER,
    ghi UTF8String
  }
  
END"#,
    )
}
#[test]
fn test_extensible_choice() {
    parse_asn_map_to_rust_map_to_stringify_with_proc_macro_annotation_re_parse_check_equal(
        r#"BasicSchema DEFINITIONS AUTOMATIC TAGS ::= BEGIN

  MyType ::= [PRIVATE 1] CHOICE {
    abc UTF8String,
    def [APPLICATION 7] INTEGER,
    ...,
    ghi UTF8String
  }
  
END"#,
    )
}

#[test]
fn test_standard_sequence() {
    parse_asn_map_to_rust_map_to_stringify_with_proc_macro_annotation_re_parse_check_equal(
        r#"BasicSchema DEFINITIONS AUTOMATIC TAGS ::= BEGIN

  MyType ::= [5] SEQUENCE {
    abc UTF8String,
    def [APPLICATION 7] INTEGER,
    ghi UTF8String
  }
  
END"#,
    )
}
#[test]
fn test_extensible_sequence() {
    parse_asn_map_to_rust_map_to_stringify_with_proc_macro_annotation_re_parse_check_equal(
        r#"BasicSchema DEFINITIONS AUTOMATIC TAGS ::= BEGIN

  MyType ::= [5] SEQUENCE {
    abc UTF8String,
    ...,
    def [APPLICATION 7] INTEGER,
    ghi UTF8String
  }
  
END"#,
    )
}

#[test]
fn test_standard_sequence_of() {
    parse_asn_map_to_rust_map_to_stringify_with_proc_macro_annotation_re_parse_check_equal(
        r#"BasicSchema DEFINITIONS AUTOMATIC TAGS ::= BEGIN

  MyType ::= [1023] SEQUENCE OF INTEGER
  
END"#,
    )
}

#[test]
fn test_integer_constants() {
    parse_asn_map_to_rust_map_to_stringify_with_proc_macro_annotation_re_parse_check_equal(
        r#"BasicSchema DEFINITIONS AUTOMATIC TAGS ::= BEGIN

  MyType ::= [5] SEQUENCE {
    great INTEGER { abc(3), def(68) } (0..255),
    wower INTEGER { ghi(3), jkl(99) },
    def [APPLICATION 7] INTEGER,
    ghi UTF8String
  }
  
  OtherType ::= [APPLICATION 99] INTEGER {
    my-type(5),
    other-type(7)
  }
  
END"#,
    )
}

#[test]
fn test_extensible_integer() {
    parse_asn_map_to_rust_map_to_stringify_with_proc_macro_annotation_re_parse_check_equal(
        r#"BasicSchema DEFINITIONS AUTOMATIC TAGS ::= BEGIN

    RangedAndExtensible ::= Integer (0..255,...)
  
END"#,
    )
}

fn parse_asn_map_to_rust_map_to_stringify_with_proc_macro_annotation_re_parse_check_equal(
    asn: &str,
) {
    let tokens = Tokenizer::default().parse(asn);
    let asn_model = Model::try_from(tokens).unwrap().try_resolve().unwrap();
    let rust_model = asn_model.to_rust();

    for definition in rust_model.definitions {
        let stringified = generate_rust_code_with_proc_macro_attributes(&definition);
        let mut lines = stringified.lines().map(str::trim).filter(|s| !s.is_empty());

        let attribute = extract_attribute(lines.next().unwrap());
        let body = lines
            .map(|l| l.to_string())
            .collect::<Vec<_>>()
            .join("\n")
            .parse::<TokenStream>()
            .unwrap();

        println!("---");
        println!("ATTRIBUTE: {}", attribute.to_string());
        println!("BODY:      {}", body.to_string());
        println!("---");

        let re_parsed = asn1rs_model::proc_macro::parse_asn_definition(attribute, body)
            .map(|(d, _item)| d)
            .unwrap()
            .unwrap();

        let re_parsed_model = Model {
            name: rust_model.name.clone(),
            imports: rust_model.imports.clone(),
            definitions: vec![re_parsed],
            ..Default::default()
        };

        assert_eq!(vec![definition], re_parsed_model.to_rust().definitions);
        println!("{:?}", re_parsed_model.to_rust().definitions);
    }
}

fn generate_rust_code_with_proc_macro_attributes(definition: &Definition<Rust>) -> String {
    let mut scope = Scope::new();
    RustCodeGenerator::default().add_definition(&mut scope, &definition);
    scope.to_string()
}

fn extract_attribute(attr: &str) -> TokenStream {
    const PREFIX: &str = "#[asn(";
    const SUFFIX: &str = ")]";

    assert!(attr.starts_with(PREFIX));
    assert!(attr.ends_with(SUFFIX));
    let substr = attr.split_at(PREFIX.len()).1;
    let substr = substr.split_at(substr.len() - SUFFIX.len()).0;
    let attr: TokenStream = substr.parse().unwrap();
    attr
}
