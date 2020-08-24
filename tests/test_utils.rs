pub use asn1rs::prelude::*;

pub fn serialize_uper(to_uper: &impl Writable) -> (usize, Vec<u8>) {
    let mut writer = UperWriter::default();
    writer.write(to_uper).unwrap();
    let bits = writer.bit_len();
    (bits, writer.into_bytes_vec())
}

pub fn deserialize_uper<T: Readable>(data: &[u8], bits: usize) -> T {
    let mut reader = UperReader::from_bits(data, bits);
    let result = reader.read::<T>().unwrap();
    assert_eq!(
        0,
        reader.bits_remaining(),
        "After reading, there are still bits remaining!"
    );
    result
}

pub fn serialize_and_deserialize_uper<T: Readable + Writable + std::fmt::Debug + PartialEq>(
    bits: usize,
    data: &[u8],
    uper: &T,
) {
    let serialized = serialize_uper(uper);
    assert_eq!(
        (bits, data),
        (serialized.0, &serialized.1[..]),
        "Serialized binary data does not match"
    );
    assert_eq!(
        uper,
        &deserialize_uper::<T>(data, bits),
        "Deserialized data struct does not match"
    );
}
