mod test_utils;
use test_utils::*;

asn_to_rust!(
    r"BasicNull DEFINITIONS AUTOMATIC TAGS ::=
    BEGIN
    
    MyNull ::= NULL
    
    NullSeq ::= SEQUENCE {
        abc UTF8String,
        def NULL,
        ghi MyNull
    }
    
    NullChoice ::= Choice {
        abc UTF8String,
        def NULL,
        ghi MyNull
    }
    
    
    END"
);

#[test]
fn test_sequence() {
    // from playground
    serialize_and_deserialize_uper(
        8 * 4,
        &[0x03, 0x61, 0x62, 0x63],
        &NullSeq {
            abc: "abc".to_string(),
            def: Null,
            ghi: MyNull(Null),
        },
    );
}

#[test]
fn test_choice_abc() {
    // from playground
    serialize_and_deserialize_uper(
        8 * 4 + 2,
        &[0x00, 0xD8, 0x58, 0x98, 0xC0],
        &NullChoice::Abc("abc".to_string()),
    );
}

#[test]
fn test_choice_def() {
    // from playground
    serialize_and_deserialize_uper(2, &[0x40], &NullChoice::Def(Null));
}

#[test]
fn test_choice_ghi() {
    // from playground
    serialize_and_deserialize_uper(2, &[0x80], &NullChoice::Ghi(MyNull(Null)));
}
