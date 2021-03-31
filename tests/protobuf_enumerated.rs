mod test_utils;

use test_utils::*;

asn_to_rust!(
    r"MyDef DEFINITIONS AUTOMATIC TAGS ::=
    BEGIN

    ProtobufEnum ::= SEQUENCE {
        some-enum ENUMERATED {
            A,
            B,
            C
        }
    }

    ProtobufEnumExt ::= SEQUENCE {
        lone-bool BOOLEAN,
        some-enum ENUMERATED {
            A,
            B,
            C
        },
        lone-int INTEGER
    }

    ProtobufOuterEnum ::= ENUMERATED {
        A,
        B,
        C
    }

    END"
);

#[test]
#[cfg(feature = "protobuf")]
fn test_enumeration_a() {
    serialize_and_deserialize_protobuf(
        &[8, 0],
        &ProtobufEnum {
            some_enum: ProtobufEnumSomeEnum::A,
        },
    )
}

#[test]
#[cfg(feature = "protobuf")]
fn test_enumeration_b() {
    serialize_and_deserialize_protobuf(
        &[8, 1],
        &ProtobufEnum {
            some_enum: ProtobufEnumSomeEnum::B,
        },
    )
}

#[test]
#[cfg(feature = "protobuf")]
fn test_enumeration_c() {
    serialize_and_deserialize_protobuf(
        &[8, 2],
        &ProtobufEnum {
            some_enum: ProtobufEnumSomeEnum::C,
        },
    )
}

#[test]
#[cfg(feature = "protobuf")]
fn test_enumeration_ext_b() {
    serialize_and_deserialize_protobuf(
        &[8, 0, 16, 1, 24, 217, 2],
        &ProtobufEnumExt {
            lone_bool: false,
            some_enum: ProtobufEnumExtSomeEnum::B,
            lone_int: 345_u64,
        },
    )
}

#[test]
#[cfg(feature = "protobuf")]
fn test_enumeration_outer_a() {
    serialize_and_deserialize_protobuf(&[0], &ProtobufOuterEnum::A)
}

#[test]
#[cfg(feature = "protobuf")]
fn test_enumeration_outer_b() {
    serialize_and_deserialize_protobuf(&[1], &ProtobufOuterEnum::B)
}

#[test]
#[cfg(feature = "protobuf")]
fn test_enumeration_outer_c() {
    serialize_and_deserialize_protobuf(&[2], &ProtobufOuterEnum::C)
}
