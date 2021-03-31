mod test_utils;

use test_utils::*;

asn_to_rust!(
    r"MyDef DEFINITIONS AUTOMATIC TAGS ::=
    BEGIN

    ProtobufBools ::= SEQUENCE {
        one-bool BOOLEAN,
        two-bool BOOLEAN
    }

    END"
);

#[test]
#[cfg(feature = "protobuf")]
fn test_bools() {
    serialize_and_deserialize_protobuf(
        &[8, 1, 16, 0],
        &ProtobufBools {
            one_bool: true,
            two_bool: false,
        },
    )
}
