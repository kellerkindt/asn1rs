use asn1rs::io::buffer::BitBuffer;
use asn1rs::io::uper::Writer;
use asn1rs::macros::asn_to_rust;

asn_to_rust!(
    r"BasicChoice DEFINITIONS AUTOMATIC TAGS ::=
    BEGIN
    
    Basic ::= CHOICE {
        abc UTF8String,
        def UTF8String,
        ghi INTEGER
    }
    
    Extensible ::= CHOICE {
        abc UTF8String,
        def INTEGER,
        ..., -- whatever reserved blubber comment
        ghi INTEGER,
        jkl INTEGER
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
fn test_extensible_uper() {
    assert_eq!(
        (18, vec![0x40, 0x40, 0x00]),
        serialize_uper(Extensible::Def(0))
    );
    assert_eq!(
        (26, vec![0x40, 0x81, 0x4e, 0x40]),
        serialize_uper(Extensible::Def(1337))
    );

    assert_eq!(
        (32, vec![0x80_u8, 0x02, 0x01, 0x00]),
        serialize_uper(Extensible::Ghi(0))
    );

    assert_eq!(
        (40, vec![0x80_u8, 0x03, 0x02, 0x05, 0x39]),
        serialize_uper(Extensible::Ghi(1337))
    );

    assert_eq!(
        Extensible::Ghi(1337),
        deserialize_uper(&[0x80_u8, 0x03, 0x02, 0x05, 0x39], 40)
    );
}

pub fn test_basic_uper() {
    let mut buffer = BitBuffer::default();
    let writer = &mut buffer as &mut dyn UperWriter;
    Basic::Def("abc".to_string()).write_uper(writer).unwrap();
    assert_eq!(
        &[
            0b00 << 6 // Abc 
                | 0b01 << 4 // Def 
                | 0b10 << 2 // Ghi
        ],
        &buffer.content()
    );
}

fn test_basic_variants_parsed() {
    let _abc = Basic::Abc(String::default());
    let _def = Basic::Def(String::default());
    let _ghi = Basic::Ghi(123_u64);

    match Basic::Abc(String::default()) {
        // this does not compile if there are additional unexpected variants
        Basic::Abc(_) | Basic::Def(_) | Basic::Ghi(_) => {}
    }
}
