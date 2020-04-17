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
        jkl Basic,
        mno UTF8String
    }
    
    END"
);

fn serialize_uper(to_uper: &impl Uper) -> (usize, Vec<u8>) {
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

fn serialize_and_deserialize_uper<T: Uper + std::fmt::Debug + PartialEq>(
    bits: usize,
    data: &[u8],
    uper: &T,
) {
    let serialized = serialize_uper(uper);
    assert_eq!((bits, data), (serialized.0, &serialized.1[..]));
    assert_eq!(uper, &deserialize_uper::<T>(data, bits));
}

#[test]
fn test_extensible_uper() {
    serialize_and_deserialize_uper(10, &[0x00, 0x00], &Extensible::Abc(String::default()));
    serialize_and_deserialize_uper(
        106,
        &[
            0x03, 0x12, 0x19, 0x5b, 0x1b, 0x1b, 0xc8, 0x15, 0xdb, 0xdc, 0x9b, 0x19, 0x08, 0x40,
        ],
        &Extensible::Abc("Hello World!".to_string()),
    );
    serialize_and_deserialize_uper(18, &[0x40, 0x40, 0x00], &Extensible::Def(0));
    serialize_and_deserialize_uper(26, &[0x40, 0x81, 0x4e, 0x40], &Extensible::Def(1337));
    serialize_and_deserialize_uper(32, &[0x80_u8, 0x02, 0x01, 0x00], &Extensible::Ghi(0));

    serialize_and_deserialize_uper(
        40,
        &[0x80_u8, 0x03, 0x02, 0x05, 0x39],
        &Extensible::Ghi(1337),
    );

    serialize_and_deserialize_uper(
        120,
        &[
            0x82, 0x0d, 0x0c, 0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x61, 0x67, 0x61, 0x69, 0x6e,
            0x21,
        ],
        &Extensible::Mno("Hello again!".to_string()),
    );
}

#[test]
pub fn test_basic_uper() {
    serialize_and_deserialize_uper(
        106,
        &[
            0x03, 0x12, 0x19, 0x5b, 0x1b, 0x1b, 0xc8, 0x15, 0xdb, 0xdc, 0x9b, 0x19, 0x08, 0x40,
        ],
        &Basic::Abc("Hello World!".to_string()),
    );
    serialize_and_deserialize_uper(
        106,
        &[
            0x43, 0x12, 0x19, 0x5b, 0x1b, 0x1b, 0xc8, 0x18, 0x59, 0xd8, 0x5a, 0x5b, 0x88, 0x40,
        ],
        &Basic::Def("Hello again!".to_string()),
    );
    serialize_and_deserialize_uper(26, &[0x80, 0x81, 0x4e, 0x40], &Basic::Ghi(1337));
}

#[test]
fn test_extensible_choice_inner_complex() {
    let jkl = Extensible::Jkl(Basic::Ghi(1337));
    let (bits, buffer) = serialize_uper(&jkl);
    let jkl_deserialized = deserialize_uper(&buffer[..], bits);
    assert_eq!(jkl, jkl_deserialized);
}

#[test]
fn test_basic_variants_parsed() {
    let _abc = Basic::Abc(String::default());
    let _def = Basic::Def(String::default());
    let _ghi = Basic::Ghi(123_u64);

    match Basic::Abc(String::default()) {
        // this does not compile if there are additional unexpected variants
        Basic::Abc(_) | Basic::Def(_) | Basic::Ghi(_) => {}
    }
}
