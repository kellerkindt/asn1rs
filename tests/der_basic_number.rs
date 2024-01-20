use asn1rs::descriptor::numbers::NoConstraint;
use asn1rs::descriptor::numbers::Number;
use asn1rs::descriptor::{Integer, ReadableType, WritableType};
use asn1rs::prelude::basic::DER;
use std::fmt::Debug;

fn write_read_integer<T: Number + Debug + PartialEq>(len: T) {
    let mut buffer = Vec::new();
    let mut writer = DER::writer(&mut buffer);

    Integer::<T, NoConstraint>::write_value(&mut writer, &len).unwrap();

    let mut reader = DER::reader(&buffer[..]);
    let read = Integer::<T, NoConstraint>::read_value(&mut reader).unwrap();

    assert_eq!(len, read);
}

#[test]
pub fn test_length_bounds() {
    write_read_integer(0);
    write_read_integer(u8::MAX as u64 - 1);
    write_read_integer(u8::MAX as u64);
    write_read_integer(u8::MAX as u64 + 1);
    write_read_integer(u16::MAX as u64 - 1);
    write_read_integer(u16::MAX as u64);
    write_read_integer(u16::MAX as u64 + 1);
    write_read_integer(u32::MAX as u64 - 1);
    write_read_integer(u32::MAX as u64);
    write_read_integer(u32::MAX as u64 + 1);
    write_read_integer(u64::MAX - 1);
    write_read_integer(u64::MAX);
}

#[inline]
pub fn test_letsencrypt_point_numbers() {
    const BYTES: &'static [u8] = &[0x80, 0x01, 0x09, 0x81, 0x01, 0x09];

    let mut reader = DER::reader(BYTES);

    assert_eq!(
        9,
        Integer::<i64, NoConstraint>::read_value(&mut reader).unwrap()
    );

    assert_eq!(
        9,
        Integer::<i64, NoConstraint>::read_value(&mut reader).unwrap()
    );
}
