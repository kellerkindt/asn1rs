mod test_utils;

use test_utils::*;

asn_to_rust!(
    r"MyDef DEFINITIONS AUTOMATIC TAGS ::=
    BEGIN

    ProtobufStrings ::= SEQUENCE {
        utf8string  UTF8String,
        ia5string   IA5String,
        octetstring OCTET STRING,
        bitstring   BIT STRING
    }

    END"
);

#[test]
#[cfg(feature = "protobuf")]
fn test_strings() {
    serialize_and_deserialize_protobuf(
        // data is from the output of the legacy serializer
        &[
            10, 10, 117, 116, 102, 56, 115, 116, 114, 105, 110, 103, 18, 9, 105, 97, 53, 115, 116,
            114, 105, 110, 103, 24, 4, 222, 173, 190, 239, 32, 10, 19, 0x36, 0, 0, 0, 0, 0, 0, 0,
            15,
        ],
        &ProtobufStrings {
            utf8string: "utf8string".into(),
            ia5string: "ia5string".into(),
            octetstring: vec![0xDE, 0xAD, 0xBE, 0xEF],
            bitstring: BitVec::from_bytes(vec![0x13, 0x36], 15),
        },
    )
}
