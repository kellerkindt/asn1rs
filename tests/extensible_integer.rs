mod test_utils;

use test_utils::*;

asn_to_rust!(
    r"BasicInteger DEFINITIONS AUTOMATIC TAGS ::=
    BEGIN
    
    NotRanged ::= Integer
    
    RangedAndExtensible ::= Integer (0..255,...)
    
    RangedOptional ::= SEQUENCE {
        value Integer (0..255,...) OPTIONAL
    }
    
    END"
);

#[asn(transparent)]
#[derive(Default, Debug, Clone, PartialEq, Hash)]
pub struct RangedAndExtensiblePureRust(#[asn(integer(0..255), extensible)] pub u64);

#[test]
fn test_extensible_range() {
    use asn1rs::syn::numbers::Constraint;
    assert_eq!(
        Some(0_u64),
        ___asn1rs_RangedAndExtensiblePureRustField0Constraint::MIN
    );
    assert_eq!(
        Some(255_u64),
        ___asn1rs_RangedAndExtensiblePureRustField0Constraint::MAX
    );
    assert_eq!(
        Some(0_u64),
        ___asn1rs_RangedAndExtensibleField0Constraint::MIN
    );
    assert_eq!(
        Some(255_u64),
        ___asn1rs_RangedAndExtensibleField0Constraint::MAX
    );
}

#[test]
fn test_extensible_flag() {
    use asn1rs::syn::numbers::Constraint;
    assert!(___asn1rs_RangedAndExtensiblePureRustField0Constraint::EXTENSIBLE);
    assert!(___asn1rs_RangedAndExtensibleField0Constraint::EXTENSIBLE);
}

#[test]
fn test_extensible_type() {
    let _ = RangedAndExtensible(1024); // does not compile if extensible is ignored
    let _ = RangedAndExtensiblePureRust(1024); // does not compile if extensible is ignored
}

#[test]
fn test_uper_std_0() {
    serialize_and_deserialize_uper(9, &[0x00, 0x00], &RangedAndExtensible(0));
}

#[test]
fn test_uper_opt_std_0() {
    serialize_and_deserialize_uper(10, &[0x80, 0x00], &RangedOptional { value: Some(0) });
}

#[test]
fn test_uper_opt_std_254() {
    serialize_and_deserialize_uper(10, &[0xBF, 0x80], &RangedOptional { value: Some(254) });
}

#[test]
fn test_uper_opt_std_255() {
    serialize_and_deserialize_uper(10, &[0xBF, 0xC0], &RangedOptional { value: Some(255) });
}

#[test]
fn test_uper_opt_std_256() {
    serialize_and_deserialize_uper(
        26,
        &[0xC0, 0x80, 0x40, 0x00],
        &RangedOptional { value: Some(256) },
    );
}
