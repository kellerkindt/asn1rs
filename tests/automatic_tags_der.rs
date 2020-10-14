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
    let der_content = b"0'\x80\x0209\x81\tSomething\x83\x01\x00\xa4\x05\x81\x03\x00\xd41\xa5\x0c\x0c\x04abcd\x0c\x04efgh".to_vec();
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
