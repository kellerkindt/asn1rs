#![recursion_limit = "512"]

use asn1rs::prelude::asn_to_rust;

// this should compile without noise
asn_to_rust!(
    r"SomeEmptyTypeDefinitions DEFINITIONS AUTOMATIC TAGS ::=
    BEGIN
    
    EmptySeq ::= SEQUENCE { }
    
    ChoiceWithEmptyness ::= CHOICE {
	    c1 INTEGER,
	    c2 SEQUENCE {}
    }
    
    END"
);
