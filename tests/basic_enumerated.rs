#[macro_use]
extern crate asn1rs_macros;

use asn1rs::io::buffer::BitBuffer;

asn_to_rust!(
    r"BasicEnumerated DEFINITIONS AUTOMATIC TAGS ::=
    BEGIN
    
    Basic ::= ENUMERATED {
        abc,
        def,
        ghi
    }
    
    END"
);

#[test]
fn test_all_variants_parsed() {
    let _abc = Basic::Abc;
    let _def = Basic::Def;
    let _ghi = Basic::Ghi;

    match Basic::Abc {
        // this does not compile if there are additional unexpected variants
        Basic::Abc | Basic::Def | Basic::Ghi => {}
    }
}

#[test]
pub fn test_non_extended_value() {
    let mut buffer = BitBuffer::default();
    let writer = &mut buffer as &mut dyn UperWriter;
    Basic::Abc.write_uper(writer).unwrap();
    Basic::Def.write_uper(writer).unwrap();
    Basic::Ghi.write_uper(writer).unwrap();
    assert_eq!(
        &[
            0b00 << 6 // Abc 
                | 0b01 << 4 // Def 
                | 0b10 << 2 // Ghi
        ],
        &buffer.content()
    );
}
