use asn1rs::io::buffer::BitBuffer;
use asn1rs::macros::asn_to_rust;

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
    let _ = NotRanged(123_u64); // does not compile if the inner type is not u64
}

#[test]
fn test_uper() {
    let mut buffer = BitBuffer::default();
    let writer = &mut buffer as &mut dyn UperWriter;
    RangedMax(123).write_uper(writer).unwrap();
    assert_eq!(&[0x01, 123_u8], buffer.content());

    let mut buffer = BitBuffer::default();
    let writer = &mut buffer as &mut dyn UperWriter;
    RangedMax(66_000).write_uper(writer).unwrap();
    let bytes = 66_000_u64.to_be_bytes();
    assert_eq!(&[0x03, bytes[5], bytes[6], bytes[7]], buffer.content());
}

#[test]
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
