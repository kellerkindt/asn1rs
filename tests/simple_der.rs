pub use asn1rs::prelude::*;

asn_to_rust!(
    r"Data-Structures DEFINITIONS ::=
BEGIN
  DataStructures ::= SEQUENCE
  {
     int              INTEGER,
     limitedString    UTF8String (SIZE(1..16)),
     optionalString   UTF8String OPTIONAL,
     enumerated       ENUMERATED {value1, value2, value3},
     optionalChoice   CHOICE
     {
        int1   [0] INTEGER,
        int2   [1] INTEGER
     }  OPTIONAL,
     sequenceOfString SEQUENCE OF UTF8String
  }
END"
);

#[test]
fn simple_der() {
    let der_content = b"0'\x02\x0209\x0c\tSomething\n\x01\x00\xa1\x05\x02\x03\x00\xd410\x0c\x0c\x04abcd\x0c\x04efgh".to_vec();
    let mut reader = DerReader::from_bits(der_content);
    let result = reader.read::<DataStructures>().unwrap();
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
}
