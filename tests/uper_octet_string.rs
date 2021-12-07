mod test_utils;

use test_utils::*;

asn_to_rust!(
    r#"TransparentConsts DEFINITIONS AUTOMATIC TAGS ::=
    BEGIN
    
    Container ::= SEQUENCE {
        value OCTET STRING
    }
  
    END"#
);

#[test]
pub fn octet_string_fragmented_16383() {
    octet_string_fragmented(
        16383,
        8 * 16385,
        include_bytes!("uper_octet_string_fragmented_16383.uper"),
    );
}

#[test]
pub fn octet_string_fragmented_16384() {
    octet_string_fragmented(
        16384,
        8 * 16386,
        include_bytes!("uper_octet_string_fragmented_16384.uper"),
    );
}

#[test]
pub fn octet_string_fragmented_65536() {
    octet_string_fragmented(
        65536,
        8 * 65538,
        include_bytes!("uper_octet_string_fragmented_65536.uper"),
    );
}

pub fn octet_string_fragmented(value_len: usize, bits: usize, bytes: &[u8]) {
    let container = Container {
        value: vec![0u8; value_len],
    };
    serialize_and_deserialize_uper(bits, bytes, &container);
}
