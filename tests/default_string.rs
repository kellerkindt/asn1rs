mod test_utils;

use test_utils::*;

asn_to_rust!(
    r#"DefaultString DEFINITIONS AUTOMATIC TAGS ::=
    BEGIN
    
    MyCleverSeq ::= SEQUENCE {
        secret-message UTF8String DEFAULT "hey hee ha"
    }
    
    defaultMessage UTF8String ::= "hey hee ha"
    
    MyCleverSeqRef ::= SEQUENCE {
        secret-message UTF8String DEFAULT defaultMessage
    }
    
    END"#
);

#[test]
pub fn does_it_compile() {
    let seq = MyCleverSeq {
        secret_message: "woah".to_string(),
    };
    let mut writer = PrintlnWriter::default();

    writer.write(&seq).unwrap();
    // Writing sequence MyCleverSeq, tag=Universal(16)
    //  Writing DEFAULT (default: "hey hee ha")
    //   Some
    //    Writing Utf8String(MIN..MAX), tag=ContextSpecific(0)
    //     "woah"
}

#[test]
pub fn test_seq_with_non_default_value() {
    serialize_and_deserialize_uper(
        8 * 10 + 1,
        &[
            0x84, 0xBB, 0xB7, 0xB0, 0xB4, 0x10, 0x3C, 0xB2, 0xB0, 0xB4, 0x00,
        ],
        &MyCleverSeq {
            secret_message: "woah yeah".to_string(),
        },
    );
}

#[test]
pub fn test_seq_with_default_value() {
    serialize_and_deserialize_uper(
        8 * 0 + 1,
        &[0x00],
        &MyCleverSeq {
            secret_message: "hey hee ha".to_string(),
        },
    );
}

#[test]
pub fn test_ref_with_non_default_value() {
    serialize_and_deserialize_uper(
        8 * 10 + 1,
        &[
            0x84, 0xBB, 0xB7, 0xB0, 0xB4, 0x10, 0x3C, 0xB2, 0xB0, 0xB4, 0x00,
        ],
        &MyCleverSeqRef {
            secret_message: "woah yeah".to_string(),
        },
    );
}

#[test]
pub fn test_ref_with_default_value() {
    serialize_and_deserialize_uper(
        8 * 0 + 1,
        &[0x00],
        &MyCleverSeqRef {
            secret_message: "hey hee ha".to_string(),
        },
    );
}
