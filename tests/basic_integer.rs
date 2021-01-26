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
fn test_whether_it_compiles_at_all() {}

#[test]
fn test_default_range() {
    assert_eq!(RangedMax::value_min(), NotRanged::value_min());
    assert_eq!(RangedMax::value_max(), NotRanged::value_max());
    let _ = NotRanged(123_u64); // does not compile if the inner type differs
}

#[test]
fn test_readme_sample() {
    use asn1rs::syn::numbers::Constraint;
    assert_eq!(
        ___asn1rs_RangedMaxField0Constraint::MIN,
        ___asn1rs_NotRangedField0Constraint::MIN,
    );
    assert_eq!(
        ___asn1rs_RangedMaxField0Constraint::MAX,
        ___asn1rs_NotRangedField0Constraint::MAX,
    );

    let value = NotRanged(123_u64); // does not compile if the inner type is not u64

    let mut writer = NewUperWriter::default();
    writer.write(&value).expect("Failed to serialize");

    let mut reader = writer.as_reader();
    let value2 = reader.read::<NotRanged>().expect("Failed to deserialize");

    assert_eq!(value, value2);
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
