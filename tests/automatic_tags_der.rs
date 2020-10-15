pub use asn1rs::prelude::*;

asn_to_rust!(
    r"Data-Structures DEFINITIONS AUTOMATIC TAGS ::=
BEGIN
  DataStructures ::= SEQUENCE
  {
     int              INTEGER,
     limitedString    UTF8String (SIZE(1..16)),
     optionalString   UTF8String OPTIONAL,
     enumerated       ENUMERATED {value1, value2, value3},
     optionalChoice   CHOICE
     {
        int1   INTEGER,
        int2   INTEGER
     }  OPTIONAL,
     sequenceOfString SEQUENCE OF UTF8String
  }
END"
);

#[test]
fn automatic_tags_der() {
    let der_content = [
        0x30, 0x27, 0x80, 0x02, 0x30, 0x39, 0x81, 0x09, 0x53, 0x6F, 0x6D, 0x65, 0x74, 0x68, 0x69,
        0x6E, 0x67, 0x83, 0x01, 0x00, 0xA4, 0x05, 0x81, 0x03, 0x00, 0xD4, 0x31, 0xA5, 0x0C, 0x0C,
        0x04, 0x61, 0x62, 0x63, 0x64, 0x0C, 0x04, 0x65, 0x66, 0x67, 0x68,
    ];
    let mut reader = DerReader::from_bits(der_content);
    if let Ok(result) = reader.read::<DataStructures>() {
        println!("Decoded:");
        println!("{:#?}", result);

        assert_eq!(result.int, 12345u64);
        assert_eq!(result.limited_string, "Something");
        assert_eq!(result.optional_string, None);
        assert_eq!(result.enumerated, DataStructuresEnumerated::Value1);
        assert_eq!(
            result.optional_choice,
            Some(DataStructuresOptionalChoice::Int2(54321))
        );
        assert_eq!(result.sequence_of_string, vec!["abcd", "efgh"]);
    } else {
        eprintln!("Automatic tags has bugs for now")
    }
}
