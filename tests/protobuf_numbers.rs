mod test_utils;

use test_utils::*;

asn_to_rust!(
    r"MyDef DEFINITIONS AUTOMATIC TAGS ::=
    BEGIN

    ProtobufNumbers ::= SEQUENCE {
        should-be-sint32 INTEGER (-2147483648..2147483647),
        should-be-sint64-1 INTEGER (-2147483649..2147483647),
        should-be-sint64-2 INTEGER (-2147483648..2147483648),
        should-be-uint32 INTEGER (0..4294967295),
        should-be-uint64 INTEGER (0..4294967296)
    }

    END"
);

#[test]
#[cfg(feature = "protobuf")]
fn test_numbers() {
    serialize_and_deserialize_protobuf(
        // data is from the output of the legacy serializer
        &[8, 2, 16, 3, 24, 6, 32, 4, 40, 5],
        &ProtobufNumbers {
            should_be_sint32: 1_i32,
            should_be_sint64_1: -2_i64,
            should_be_sint64_2: 3_i64,
            should_be_uint32: 4_u32,
            should_be_uint64: 5_u64,
        },
    )
}
