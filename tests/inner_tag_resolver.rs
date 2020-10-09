use asn1rs::prelude::*;

asn_to_rust!(
    "InnerEnumerated DEFINITIONS AUTOMATIC TAGS ::=
    BEGIN
    
    Other ::= SEQUENCE {
        field INTEGER
    }
    
    SomeSeq ::= SEQUENCE {
        content CHOICE {
            abc Other,
            def Other
        }
    }
    
    END"
);

#[test]
pub fn test_whether_it_compiles() {}
