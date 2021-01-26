mod test_utils;

use test_utils::*;

asn_to_rust!(
    r"MyDef DEFINITIONS AUTOMATIC TAGS ::=
    BEGIN

    ProtobufChoice ::= SEQUENCE {
        some-choice CHOICE {
            A INTEGER,
            B BOOLEAN,
            C UTF8String
        }
    }

    ProtobufChoiceExt ::= SEQUENCE {
        lone-bool BOOLEAN,
        some-choice CHOICE {
            A INTEGER,
            B BOOLEAN,
            C UTF8String
        },
        lone-int INTEGER
    }
    
    ProtobufOuterChoice ::= CHOICE {
        A INTEGER,
        B BOOLEAN,
        C UTF8String
    }
        

    END"
);

#[test]
#[cfg(feature = "protobuf")]
fn test_choice_a() {
    serialize_and_deserialize_protobuf(
        // data is from the output of the legacy serializer
        &[8, 8, 123],
        &ProtobufChoice {
            some_choice: ProtobufChoiceSomeChoice::A(123_u64),
        },
    )
}

#[test]
#[cfg(feature = "protobuf")]
fn test_choice_b() {
    serialize_and_deserialize_protobuf(
        // data is from the output of the legacy serializer
        &[8, 16, 0],
        &ProtobufChoice {
            some_choice: ProtobufChoiceSomeChoice::B(false),
        },
    )
}

#[test]
#[cfg(feature = "protobuf")]
fn test_choice_c() {
    serialize_and_deserialize_protobuf(
        // data is from the output of the legacy serializer
        &[10, 3, 26, 1, 99],
        &ProtobufChoice {
            some_choice: ProtobufChoiceSomeChoice::C("c".into()),
        },
    )
}

#[test]
#[cfg(feature = "protobuf")]
fn test_choice_ext_b() {
    serialize_and_deserialize_protobuf(
        // data is from the output of the legacy serializer
        &[8, 0, 16, 16, 1, 24, 149, 6],
        &ProtobufChoiceExt {
            lone_bool: false,
            some_choice: ProtobufChoiceExtSomeChoice::B(true),
            lone_int: 789_u64,
        },
    )
}

#[test]
#[cfg(feature = "protobuf")]
fn test_choice_outer_a() {
    serialize_and_deserialize_protobuf(
        // data is from the output of the legacy serializer
        &[8, 250, 6],
        &ProtobufOuterChoice::A(890_u64),
    )
}

#[test]
#[cfg(feature = "protobuf")]
fn test_choice_outer_b() {
    serialize_and_deserialize_protobuf(
        // data is from the output of the legacy serializer
        &[16, 1],
        &ProtobufOuterChoice::B(true),
    )
}

#[test]
#[cfg(feature = "protobuf")]
fn test_choice_outer_c() {
    serialize_and_deserialize_protobuf(
        // data is from the output of the legacy serializer
        &[26, 11, 111, 117, 116, 101, 114, 32, 115, 112, 97, 99, 101],
        &ProtobufOuterChoice::C("outer space".into()),
    )
}
