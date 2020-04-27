#![allow(unused)]

use asn1rs::syn::io::{UperReader, UperWriter};
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
