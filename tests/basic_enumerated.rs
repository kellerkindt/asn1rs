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

#[test]
fn test_uper_predefined_numbers() {
    assert_eq!((2, vec![0x00_u8]), serialize_uper(&PredefinedNumbers::Abc));
    assert_eq!((2, vec![0x40_u8]), serialize_uper(&PredefinedNumbers::Def));
    assert_eq!((8, vec![0x80_u8]), serialize_uper(&PredefinedNumbers::Ghi));
    assert_eq!((8, vec![0x81_u8]), serialize_uper(&PredefinedNumbers::Jkl));

    assert_eq!(PredefinedNumbers::Abc, deserialize_uper(&[0x00_u8], 2,));
    assert_eq!(PredefinedNumbers::Def, deserialize_uper(&[0x40_u8], 2,));
    assert_eq!(PredefinedNumbers::Ghi, deserialize_uper(&[0x80_u8], 8,));
    assert_eq!(PredefinedNumbers::Jkl, deserialize_uper(&[0x81_u8], 8,));
}

#[test]
fn test_uper_basic_variants_parsed() {
    let _abc = Basic::Abc;
    let _def = Basic::Def;
    let _ghi = Basic::Ghi;

    match Basic::Abc {
        // this does not compile if there are additional unexpected variants
        Basic::Abc | Basic::Def | Basic::Ghi => {}
    }
}

#[test]
pub fn test_uper_basic() {
    let mut writer = UperWriter::default();
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
fn test_uper_some_enum_with_skipped_numbers() {
    serialize_and_deserialize_uper(3, &[0x00], &SomeEnum::Abc);
    serialize_and_deserialize_uper(3, &[0x20], &SomeEnum::Def);
    serialize_and_deserialize_uper(3, &[0x40], &SomeEnum::Ghi);
    serialize_and_deserialize_uper(3, &[0x60], &SomeEnum::Jkl);
    serialize_and_deserialize_uper(3, &[0x80], &SomeEnum::Mno);
    serialize_and_deserialize_uper(3, &[0xA0], &SomeEnum::Qrs);
}

#[test]
fn test_der_basic() {
    serialize_and_deserialize_der(&[0x0A, 0x01, 0x00], &Basic::Abc);
    serialize_and_deserialize_der(&[0x0A, 0x01, 0x01], &Basic::Def);
    serialize_and_deserialize_der(&[0x0A, 0x01, 0x02], &Basic::Ghi);
}
