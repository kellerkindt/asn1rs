use asn1rs::descriptor::boolean::NoConstraint;
use asn1rs::descriptor::{Boolean, ReadableType, WritableType};
use asn1rs::prelude::basic::DER;

#[test]
pub fn test_der_basic_boolean() {
    for bool_value in [true, false] {
        let mut buffer = Vec::new();
        let mut writer = DER::writer(&mut buffer);

        Boolean::<NoConstraint>::write_value(&mut writer, &bool_value).unwrap();

        assert_eq!(
            &[0x01, 0x41, if bool_value { 0x01 } else { 0x00 }],
            &buffer[..]
        );

        let mut reader = DER::reader(&buffer[..]);
        let result = Boolean::<NoConstraint>::read_value(&mut reader).unwrap();

        assert_eq!(bool_value, result)
    }
}
