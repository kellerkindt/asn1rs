use asn1rs::prelude::*;
use asn1rs::syn::io::UperWriter as NewUperWriter;

asn_to_rust!(
    r"BasicInteger DEFINITIONS AUTOMATIC TAGS ::=
    BEGIN
    
    RangedMax ::= Integer (0..MAX)
    
    NotRanged ::= Integer
    
    END"
);

#[test]
fn test_default_range() {
    assert_eq!(RangedMax::value_min(), NotRanged::value_min());
    assert_eq!(RangedMax::value_max(), NotRanged::value_max());
    let _ = NotRanged(123_u64); // does not compile if the inner type differs
}

#[test]
fn test_uper_small() {
    let mut writer = NewUperWriter::default();
    writer.write(&RangedMax(123)).unwrap();
    assert_eq!(&[0x01, 123_u8], writer.byte_content());
}

#[test]
fn test_uper_big() {
    let mut writer = NewUperWriter::default();
    writer.write(&RangedMax(66_000)).unwrap();
    let bytes = 66_000_u64.to_be_bytes();
    assert_eq!(&[0x03, bytes[5], bytes[6], bytes[7]], writer.byte_content());
}

#[test]
#[cfg(feature = "protobuf")]
fn test_protobuf() {
    let mut buffer = Vec::default();
    let writer = &mut buffer as &mut dyn ProtobufWriter;
    RangedMax(123).write_protobuf(writer).unwrap();
    assert_eq!(&[0x08, 123_u8], &buffer[..]);

    let mut buffer = Vec::default();
    let writer = &mut buffer as &mut dyn ProtobufWriter;
    RangedMax(66_000).write_protobuf(writer).unwrap();
    assert_eq!(&[0x08, 0x80 | 80_u8, 0x80 | 3, 4], &buffer[..]);
}
