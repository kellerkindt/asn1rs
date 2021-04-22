mod test_utils;

use test_utils::*;

asn_to_rust!(
    r#"TransparentConsts DEFINITIONS AUTOMATIC TAGS ::=
    BEGIN
    
    
    WrappedType ::= INTEGER (0..255)
    
    WrappedZero     WrappedType ::= 0
    wrapped-one     WrappedType ::= 1
    wrappedTwo      WrappedType ::= 2
    Wrapped-Three   WrappedType ::= 3
    
    END"#
);

#[test]
pub fn does_it_compile() {
    assert_eq!(WrappedType(0u8), WRAPPED_ZERO);
    assert_eq!(WrappedType(1u8), WRAPPED_ONE);
    assert_eq!(WrappedType(2u8), WRAPPED_TWO);
    assert_eq!(WrappedType(3u8), WRAPPED_THREE);
}
