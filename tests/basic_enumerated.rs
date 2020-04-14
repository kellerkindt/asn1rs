use asn1rs::io::buffer::BitBuffer;
use asn1rs::io::uper::Writer;
use asn1rs::macros::asn_to_rust;

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

    
    END"
);

fn serialize_uper(to_uper: impl Uper) -> (usize, Vec<u8>) {
    let mut buffer = BitBuffer::default();
    to_uper
        .write_uper(&mut buffer as &mut dyn UperWriter)
        .unwrap();
    let bits = buffer.bit_len();
    (bits, buffer.into())
}

fn deserialize_uper<T: Uper>(data: &[u8], bits: usize) -> T {
    let mut buffer = BitBuffer::default();
    buffer.write_bit_string(data, 0, bits).unwrap();
    T::read_uper(&mut buffer as &mut dyn UperReader).unwrap()
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
    let mut buffer = BitBuffer::default();
    let writer = &mut buffer as &mut dyn UperWriter;
    Basic::Abc.write_uper(writer).unwrap();
    Basic::Def.write_uper(writer).unwrap();
    Basic::Ghi.write_uper(writer).unwrap();
    assert_eq!(
        &[
            0b00 << 6 // Abc 
                | 0b01 << 4 // Def 
                | 0b10 << 2 // Ghi
        ],
        &buffer.content()
    );
}
