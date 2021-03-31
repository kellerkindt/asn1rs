mod test_utils;

use test_utils::*;

asn_to_rust!(
    r#"DefaultString DEFINITIONS AUTOMATIC TAGS ::=
    BEGIN
    
    theValue BOOLEAN ::= FALSE
    
    MyCleverSeq ::= SEQUENCE {
        secret-flag BOOLEAN DEFAULT TRUE,
        flag-secret BOOLEAN DEFAULT theValue
    }
    
    END"#
);

#[test]
pub fn does_it_compile() {
    let seq = MyCleverSeq {
        secret_flag: true,
        flag_secret: true,
    };
    let mut writer = PrintlnWriter::default();

    writer.write(&seq).unwrap();
    // Writing sequence MyCleverSeq, tag=Universal(16)
    //  Writing DEFAULT (default: true)
    //   None
    //  Writing DEFAULT (default: false)
    //   Some
    //    WRITING Boolean, tag=Universal(1)
    //     true
}

#[test]
pub fn test_seq_with_non_default_value_00() {
    serialize_and_deserialize_uper(
        8 * 0 + 3,
        &[0x80],
        &MyCleverSeq {
            secret_flag: false,
            flag_secret: false,
        },
    );
}
#[test]
pub fn test_seq_with_non_default_value_01() {
    serialize_and_deserialize_uper(
        8 * 0 + 4,
        &[0xD0],
        &MyCleverSeq {
            secret_flag: false,
            flag_secret: true,
        },
    );
}

#[test]
pub fn test_seq_with_default_value_10() {
    serialize_and_deserialize_uper(
        8 * 0 + 2,
        &[0x00],
        &MyCleverSeq {
            secret_flag: true,
            flag_secret: false,
        },
    );
}

#[test]
pub fn test_seq_with_non_default_value_11() {
    serialize_and_deserialize_uper(
        8 * 0 + 3,
        &[0x60],
        &MyCleverSeq {
            secret_flag: true,
            flag_secret: true,
        },
    );
}
