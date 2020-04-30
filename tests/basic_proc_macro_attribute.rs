#![allow(unused)]

use asn1rs::syn::io::{UperReader, UperWriter};
use asn1rs::syn::Reader;
use asn1rs_macros::asn;

#[asn(sequence)]
#[derive(Debug, Default, PartialOrd, PartialEq)]
pub struct Potato {
    #[asn(integer)]
    size: u64,
    #[asn(integer(min..max))]
    size2: u64,
    #[asn(integer(12..128), tag(APPLICATION(4)))]
    size3: u8,
    #[asn(utf8string, tag(4))]
    string: String,
}

#[test]
fn test_compiles() {
    let p = Potato {
        size: 123,
        size2: 1234,
        size3: 234,
        string: String::from("where is the content"),
    };
}

#[test]
fn test_serialize_with_uper() {
    let p = Potato {
        size: 123,
        size2: 1234,
        size3: 128,
        string: String::from("where is the content"),
    };
    let mut uper = UperWriter::default();
    uper.write(&p).unwrap();
    assert_eq!(
        &[
            // https://asn1.io/asn1playground/
            0x01, 0x7B, 0x02, 0x04, 0xD2, 0xE8, 0x28, 0xEE, 0xD0, 0xCA, 0xE4, 0xCA, 0x40, 0xD2,
            0xE6, 0x40, 0xE8, 0xD0, 0xCA, 0x40, 0xC6, 0xDE, 0xDC, 0xE8, 0xCA, 0xDC, 0xE8
        ],
        uper.byte_content()
    );
    assert_eq!(26 * 8 + 7, uper.bit_len());
}

#[test]
fn test_deserialize_with_uper() {
    let mut uper = UperReader::from_bits(
        vec![
            // https://asn1.io/asn1playground/
            0x01, 0x7B, 0x02, 0x04, 0xD2, 0xE8, 0x28, 0xEE, 0xD0, 0xCA, 0xE4, 0xCA, 0x40, 0xD2,
            0xE6, 0x40, 0xE8, 0xD0, 0xCA, 0x40, 0xC6, 0xDE, 0xDC, 0xE8, 0xCA, 0xDC, 0xE8,
        ],
        26 * 8 + 7,
    );
    let p = uper.read::<Potato>().unwrap();
    assert_eq!(
        Potato {
            size: 123,
            size2: 1234,
            size3: 128,
            string: String::from("where is the content"),
        },
        p
    );
}

#[asn(enumerated)]
#[derive(Debug, PartialOrd, PartialEq)]
pub enum Topping {
    NotPineapple,
    EvenLessPineapple,
    NoPineappleAtAll,
}

#[test]
fn topping_test_serialize_with_uper() {
    let mut uper = UperWriter::default();
    uper.write(&Topping::NotPineapple).unwrap();
    uper.write(&Topping::EvenLessPineapple).unwrap();
    uper.write(&Topping::NoPineappleAtAll).unwrap();
    assert_eq!(&[0x00 | 0x40 >> 2 | 0x80 >> 4], uper.byte_content());
    assert_eq!(6, uper.bit_len());
}

#[test]
fn topping_test_deserialize_with_uper() {
    let mut uper = UperReader::from_bits(vec![0x00_u8 | 0x40 >> 2 | 0x80 >> 4], 6);
    assert_eq!(Topping::NotPineapple, uper.read::<Topping>().unwrap());
    assert_eq!(Topping::EvenLessPineapple, uper.read::<Topping>().unwrap());
    assert_eq!(Topping::NoPineappleAtAll, uper.read::<Topping>().unwrap());
}

#[asn(sequence)]
#[derive(Debug, PartialOrd, PartialEq)]
pub struct Pizza {
    #[asn(integer(1..4))]
    size: u8,
    #[asn(complex)]
    topping: Topping,
}

#[test]
fn pizza_test_uper_1() {
    let mut uper = UperWriter::default();
    let pizza = Pizza {
        size: 2,
        topping: Topping::NotPineapple,
    };
    uper.write(&pizza).unwrap();
    // https://asn1.io/asn1playground/
    assert_eq!(&[0x40], uper.byte_content());
    assert_eq!(4, uper.bit_len());
    let mut uper = uper.into_reader();
    assert_eq!(pizza, uper.read::<Pizza>().unwrap());
    assert_eq!(0, uper.bits_remaining());
}

#[test]
fn pizza_test_uper_2() {
    let mut uper = UperWriter::default();
    let pizza = Pizza {
        size: 1,
        topping: Topping::NoPineappleAtAll,
    };
    uper.write(&pizza).unwrap();
    // https://asn1.io/asn1playground/
    assert_eq!(&[0x20], uper.byte_content());
    assert_eq!(4, uper.bit_len());
    let mut uper = uper.into_reader();
    assert_eq!(pizza, uper.read::<Pizza>().unwrap());
    assert_eq!(0, uper.bits_remaining());
}

#[test]
fn pizza_test_uper_3() {
    let mut uper = UperWriter::default();
    let pizza = Pizza {
        size: 3,
        topping: Topping::EvenLessPineapple,
    };
    uper.write(&pizza).unwrap();
    // https://asn1.io/asn1playground/
    assert_eq!(&[0x90], uper.byte_content());
    assert_eq!(4, uper.bit_len());
    let mut uper = uper.into_reader();
    assert_eq!(pizza, uper.read::<Pizza>().unwrap());
    assert_eq!(0, uper.bits_remaining());
}

#[asn(choice)]
#[derive(Debug, PartialOrd, PartialEq)]
pub enum WhatToEat {
    #[asn(complex)]
    Potato(Potato),
    #[asn(complex)]
    Pizza(Pizza),
}

#[test]
fn what_to_eat_test_uper_1() {
    let mut uper = UperWriter::default();
    let what = WhatToEat::Pizza(Pizza {
        size: 3,
        topping: Topping::EvenLessPineapple,
    });
    uper.write(&what).unwrap();
    // https://asn1.io/asn1playground/
    assert_eq!(&[0xC8], uper.byte_content());
    assert_eq!(5, uper.bit_len());
    let mut uper = uper.into_reader();
    assert_eq!(what, uper.read::<WhatToEat>().unwrap());
    assert_eq!(0, uper.bits_remaining());
}

#[test]
fn what_to_eat_test_uper_2() {
    let mut uper = UperWriter::default();
    let what = WhatToEat::Potato(Potato {
        size: 13,
        size2: 37,
        size3: 42,
        string: "such tasty potato".to_string(),
    });
    uper.write(&what).unwrap();
    // https://asn1.io/asn1playground/
    assert_eq!(
        &[
            0x00, 0x86, 0x80, 0x92, 0x9E, 0x11, 0x73, 0x75, 0x63, 0x68, 0x20, 0x74, 0x61, 0x73,
            0x74, 0x79, 0x20, 0x70, 0x6F, 0x74, 0x61, 0x74, 0x6F
        ],
        uper.byte_content()
    );
    assert_eq!(23 * 8, uper.bit_len());
    let mut uper = uper.into_reader();
    assert_eq!(what, uper.read::<WhatToEat>().unwrap());
    assert_eq!(0, uper.bits_remaining());
}

/*
BasicSchema DEFINITIONS AUTOMATIC TAGS ::=
BEGIN
  Potato ::= SEQUENCE {
    size INTEGER,
    size2 INTEGER,
    size3 INTEGER(12..128),
    string Utf8String
  }

  Topping ::= ENUMERATED
  {
    not_pineapple,
    even_less_pineapple,
    no_pineapple_at_all
  }

  Pizza ::= SEQUENCE {
    size INTEGER(1..4),
    topping Topping
  }

  WhatToEat ::= CHOICE {
    potato Potato,
    pizza Pizza
  }
END

*/

#[asn(sequence)]
#[derive(Debug, PartialOrd, PartialEq)]
pub struct AreWeBinaryYet {
    #[asn(octet_string)]
    binary: Vec<u8>,
}

#[test]
fn are_we_binary_yet_uper() {
    let mut uper = UperWriter::default();
    let are_we = AreWeBinaryYet {
        binary: vec![0x13, 0x37],
    };
    uper.write(&are_we).unwrap();
    // https://asn1.io/asn1playground/
    assert_eq!(&[02, 0x13, 0x37], uper.byte_content());
    assert_eq!(3 * 8, uper.bit_len());
    let mut uper = uper.into_reader();
    assert_eq!(are_we, uper.read::<AreWeBinaryYet>().unwrap());
    assert_eq!(0, uper.bits_remaining());
}
