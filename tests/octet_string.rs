mod test_utils;

use test_utils::*;

asn_to_rust!(
    r"MyDef DEFINITIONS AUTOMATIC TAGS ::=
    BEGIN

    MyCleverSeq ::= SEQUENCE {
        secret-codes OCTET STRING (SIZE(5..12))
    }
    
    MyCleverSeq2 ::= SEQUENCE {
        secret-codes OCTET STRING (SIZE(1..2))
    }     
    
    MyCleverSeq3 ::= SEQUENCE {
        secret-codes OCTET STRING (SIZE(1..2,...))
    }
    
    MyCleverSeq4 ::= SEQUENCE {
        secret-codes OCTET STRING (SIZE(2..2,...))
    }
    
    MyCleverSeq5 ::= SEQUENCE {
        secret-codes OCTET STRING (SIZE(2..2))
    }

    END"
);

#[test]
fn test_my_clever_seq_min_max() {
    use asn1rs::descriptor::octetstring::Constraint;
    assert_eq!(
        Some(5),
        ___asn1rs_MyCleverSeqFieldSecretCodesConstraint::MIN
    );
    assert_eq!(
        Some(12),
        ___asn1rs_MyCleverSeqFieldSecretCodesConstraint::MAX
    );
}

#[test]
fn test_octet_string_constraint() {
    serialize_and_deserialize_uper(
        43,
        &[0x02, 0x46, 0x8A, 0xCF, 0x12, 0x00],
        &MyCleverSeq {
            secret_codes: vec![0x12, 0x34, 0x56, 0x78, 0x90],
        },
    )
}

#[test]
fn test_octet_string_very_short() {
    serialize_and_deserialize_uper(
        9,
        &[0x09, 0x00],
        &MyCleverSeq2 {
            secret_codes: vec![0x12],
        },
    )
}

#[test]
fn test_octet_string_extended() {
    serialize_and_deserialize_uper(
        41,
        &[0x82, 0x09, 0x1A, 0x2B, 0x3C, 0x00],
        &MyCleverSeq3 {
            secret_codes: vec![0x12, 0x34, 0x56, 0x78],
        },
    )
}

#[test]
fn test_octet_string_fixed_extended() {
    serialize_and_deserialize_uper(
        41,
        &[0x82, 0x09, 0x1A, 0x2B, 0x3C, 0x00],
        &MyCleverSeq3 {
            secret_codes: vec![0x12, 0x34, 0x56, 0x78],
        },
    )
}

#[test]
fn test_octet_string_fixed() {
    serialize_and_deserialize_uper(
        17,
        &[0x09, 0x1A, 0x00],
        &MyCleverSeq4 {
            secret_codes: vec![0x12, 0x34],
        },
    )
}

#[test]
fn test_octet_string_fixed_unextendable() {
    serialize_and_deserialize_uper(
        16,
        &[0x12, 0x34],
        &MyCleverSeq5 {
            secret_codes: vec![0x12, 0x34],
        },
    )
}
