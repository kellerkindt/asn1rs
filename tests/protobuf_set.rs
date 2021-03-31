mod test_utils;

use test_utils::*;

asn_to_rust!(
    r"MyDef DEFINITIONS AUTOMATIC TAGS ::=
    BEGIN

    ProtobufSet ::= SET {
        inner           SET { magic-number INTEGER } 
    }

    ProtobufSetExt ::= SET {
        lone-bool       BOOLEAN,
        inner           SET { magic-number INTEGER },
        another-string  UTF8String
    }

    END"
);

#[test]
#[cfg(feature = "protobuf")]
fn test_set() {
    serialize_and_deserialize_protobuf(
        &[10, 2, 8, 42],
        &ProtobufSet {
            inner: ProtobufSetInner { magic_number: 42 },
        },
    )
}

#[test]
#[cfg(feature = "protobuf")]
fn test_set_ext() {
    serialize_and_deserialize_protobuf(
        &[8, 0, 18, 3, 8, 185, 10, 26, 3, 101, 120, 116],
        &ProtobufSetExt {
            lone_bool: false,
            inner: ProtobufSetExtInner { magic_number: 1337 },
            another_string: "ext".into(),
        },
    )
}
