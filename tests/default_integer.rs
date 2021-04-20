mod test_utils;

use test_utils::*;

asn_to_rust!(
    r#"DefaultInteger DEFINITIONS AUTOMATIC TAGS ::=
    BEGIN

    MyCleverSeq ::= SEQUENCE {
        secret-code INTEGER DEFAULT 1337
    }
    
    theRefValue INTEGER(-9999..9999) ::= -1337
    
    MyCleverSeqRef ::= SEQUENCE {
        secret-code INTEGER(-9999..9999) DEFAULT theRefValue
    }
    
    MyWrappedInteger ::= Integer {
        some-unit(1)
    }
    
    MyCleverSeqWrapped ::= SEQUENCE {
        secret-code MyWrappedInteger DEFAULT 1337
    }
    
    END"#
);

#[test]
pub fn does_it_compile() {
    let seq = MyCleverSeq { secret_code: 5 };
    let mut writer = PrintlnWriter::default();

    writer.write(&seq).unwrap();
    // Writing sequence MyCleverSeq, tag=Universal(16)
    //  Writing DEFAULT (default: 1337)
    //   Some
    //    WRITING Integer(MIN..MAX), tag=ContextSpecific(0)
    //     5

    let _ = MyCleverSeqWrapped {
        secret_code: MyWrappedInteger(1337),
    };
}

#[test]
pub fn test_seq_with_non_default_value_0() {
    serialize_and_deserialize_uper(
        8 * 2 + 1,
        &[0x80, 0x80, 0x00],
        &MyCleverSeq { secret_code: 0 },
    );
}

#[test]
pub fn test_seq_with_non_default_value_1500() {
    serialize_and_deserialize_uper(
        8 * 3 + 1,
        &[0x81, 0x02, 0xEE, 0x00],
        &MyCleverSeq { secret_code: 1500 },
    );
}

#[test]
pub fn test_seq_with_default_value() {
    serialize_and_deserialize_uper(8 * 0 + 1, &[0x00], &MyCleverSeq { secret_code: 1337 });
}

#[test]
pub fn test_ref_with_non_default_value_0() {
    serialize_and_deserialize_uper(8 * 2 + 0, &[0xA7, 0x0F], &MyCleverSeqRef { secret_code: 0 });
}

#[test]
pub fn test_ref_with_non_default_value_1500() {
    serialize_and_deserialize_uper(
        8 * 2 + 0,
        &[0xAC, 0xEB],
        &MyCleverSeqRef { secret_code: 1500 },
    );
}

#[test]
pub fn test_ref_with_default_value() {
    serialize_and_deserialize_uper(8 * 0 + 1, &[0x00], &MyCleverSeqRef { secret_code: -1337 });
}
