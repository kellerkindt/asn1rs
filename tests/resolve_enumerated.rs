mod test_utils;

use test_utils::*;

asn_to_rust!(
    r"BasicEnumerated DEFINITIONS AUTOMATIC TAGS ::=
    BEGIN
    
    Basic ::= ENUMERATED {
        abc,
        def,
        ghi
    }
    
    Container ::= SEQUENCE {
        the-selection Basic DEFAULT abc
    }

    
    END"
);
#[test]
pub fn does_it_compile() {
    let _ = Basic::Abc;
    let _ = Basic::Def;
    let _ = Basic::Ghi;
    let seq = Container {
        the_selection: Basic::Def,
    };

    PrintlnWriter::default().write(&seq).unwrap();
    // Writing sequence MyCleverSeq, tag=Universal(16)
    //  Writing DEFAULT (default: 1337)
    //   Some
    //    WRITING Integer(MIN..MAX), tag=ContextSpecific(0)
    //     5
}

#[test]
pub fn the_selection_abc() {
    serialize_and_deserialize_uper(
        1,
        &[0x00],
        &Container {
            the_selection: Basic::Abc,
        },
    );
}

#[test]
pub fn the_selection_def() {
    serialize_and_deserialize_uper(
        3,
        &[0xA0],
        &Container {
            the_selection: Basic::Def,
        },
    );
}

#[test]
pub fn the_selection_ghi() {
    serialize_and_deserialize_uper(
        3,
        &[0xC0],
        &Container {
            the_selection: Basic::Ghi,
        },
    );
}
