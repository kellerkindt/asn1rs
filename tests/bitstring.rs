#![recursion_limit = "512"]

mod test_utils;

use asn1rs::syn::bitstring::BitVec;
use test_utils::*;

asn_to_rust!(
    r"BasicBitString DEFINITIONS AUTOMATIC TAGS ::=
    BEGIN
    
    BasicConstrained ::= SEQUENCE {
        abc BIT STRING (SIZE(8))
    }
    
    
    END"
);

#[test]
fn test_fixed_size() {
    // from playground
    serialize_and_deserialize_uper(
        8,
        &[0x12],
        &BasicConstrained {
            abc: BitVec::from_all_bytes(vec![0x12]),
        },
    );
}
