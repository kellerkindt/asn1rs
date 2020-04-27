#![allow(unused)]

use asn1rs_macros::asn;

#[asn(sequence)]
#[derive(Default)]
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
