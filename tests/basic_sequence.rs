#![recursion_limit = "512"]

mod test_utils;

use test_utils::*;

asn_to_rust!(
    r"BasicSet DEFINITIONS AUTOMATIC TAGS ::=
    BEGIN

      Basic ::= [5] SEQUENCE {
        abc [APPLICATION 7] UTF8String,
        def INTEGER
      }
    
      Extensible ::= [5] SEQUENCE {
        abc [APPLICATION 7] UTF8String,
        def INTEGER,
        ...,
        ghi [APPLICATION 2] UTF8String
      }
      
      SomeVal ::= INTEGER (-32768..32767)
          
    END"
);

#[test]
fn does_it_compile() {
    let _ = SomeVal(13i16);
}

#[test]
fn test_basic() {
    // from playground
    serialize_and_deserialize_uper(
        8 * 15,
        &[
            0x0B, 0x68, 0x65, 0x6C, 0x6C, 0x6F, 0x20, 0x77, 0x6F, 0x72, 0x6C, 0x64, 0x02, 0x03,
            0x0A,
        ],
        &Basic {
            abc: "hello world".to_string(),
            def: 778,
        },
    );
}

#[test]
fn test_extensible() {
    // from playground
    serialize_and_deserialize_uper(
        8 * 29 + 1,
        &[
            0x83, 0xB1, 0x3C, 0xB2, 0x90, 0x31, 0x3C, 0xB2, 0x81, 0x01, 0x83, 0x00, 0x88, 0x07,
            0xB3, 0xB9, 0x32, 0xB0, 0xBA, 0x10, 0x32, 0xBC, 0x3A, 0x32, 0xB7, 0x39, 0xB4, 0xB7,
            0xB7, 0x00,
        ],
        &Extensible {
            abc: "bye bye".to_string(),
            def: 774,
            ghi: Some("great extension".to_string()),
        },
    );
}
