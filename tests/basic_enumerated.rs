use asn1rs::rw::UperReader as NewUperReader;
use asn1rs::rw::UperWriter as NewUperWriter;

mod test_utils;
use test_utils::*;

asn_to_rust!(
    r"BasicEnumerated DEFINITIONS AUTOMATIC TAGS ::=
    BEGIN
    
    Basic ::= ENUMERATED {
        abc,
        def,
        ghi
    }
    
    PredefinedNumbers ::= ENUMERATED {
        abc(0),
        def(5),
        ..., -- whatever reserved blubber comment
        ghi(8),
        jkl(9)
    }

    SomeEnum ::= ENUMERATED {
        abc(0),
        def(1),
        ghi(2),
        jkl(3),
        mno(4),
        qrs(15)
    }

    
    END"
);

fn serialize_uper(to_uper: impl Writable) -> (usize, Vec<u8>) {
    let mut writer = NewUperWriter::default();
    writer.write(&to_uper).unwrap();
    let bits = writer.bit_len();
    (bits, writer.into_bytes_vec())
}

fn deserialize_uper<T: Readable>(bytes: &[u8], bit_len: usize) -> T {
    let mut reader = NewUperReader::from((bytes, bit_len));
    reader.read::<T>().unwrap()
}

#[test]
fn test_predefined_numbers() {
    assert_eq!((2, vec![0x00_u8]), serialize_uper(PredefinedNumbers::Abc));
    assert_eq!((2, vec![0x40_u8]), serialize_uper(PredefinedNumbers::Def));
    assert_eq!((8, vec![0x80_u8]), serialize_uper(PredefinedNumbers::Ghi));
    assert_eq!((8, vec![0x81_u8]), serialize_uper(PredefinedNumbers::Jkl));

    assert_eq!(PredefinedNumbers::Abc, deserialize_uper(&[0x00_u8], 2,));
    assert_eq!(PredefinedNumbers::Def, deserialize_uper(&[0x40_u8], 2,));
    assert_eq!(PredefinedNumbers::Ghi, deserialize_uper(&[0x80_u8], 8,));
    assert_eq!(PredefinedNumbers::Jkl, deserialize_uper(&[0x81_u8], 8,));
}

#[test]
fn test_basic_variants_parsed() {
    let _abc = Basic::Abc;
    let _def = Basic::Def;
    let _ghi = Basic::Ghi;

    match Basic::Abc {
        // this does not compile if there are additional unexpected variants
        Basic::Abc | Basic::Def | Basic::Ghi => {}
    }
}

#[test]
pub fn test_basic_uper() {
    let mut writer = NewUperWriter::default();
    writer.write(&Basic::Abc).unwrap();
    writer.write(&Basic::Def).unwrap();
    writer.write(&Basic::Ghi).unwrap();

    assert_eq!(
        &[
            0b00 << 6 // Abc 
                | 0b01 << 4 // Def 
                | 0b10 << 2 // Ghi
        ],
        writer.byte_content()
    );
}

#[test]
fn test_some_enum_with_skipped_numbers() {
    test_utils::serialize_and_deserialize_uper(3, &[0x00], &SomeEnum::Abc);
    test_utils::serialize_and_deserialize_uper(3, &[0x20], &SomeEnum::Def);
    test_utils::serialize_and_deserialize_uper(3, &[0x40], &SomeEnum::Ghi);
    test_utils::serialize_and_deserialize_uper(3, &[0x60], &SomeEnum::Jkl);
    test_utils::serialize_and_deserialize_uper(3, &[0x80], &SomeEnum::Mno);
    test_utils::serialize_and_deserialize_uper(3, &[0xA0], &SomeEnum::Qrs);
}
