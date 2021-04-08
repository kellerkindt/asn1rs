#![recursion_limit = "512"]

mod test_utils;

use test_utils::*;

asn_to_rust!(
    r"ParseWithComponents DEFINITIONS AUTOMATIC TAGS ::=
    BEGIN
    
    SomeEnum ::= ENUMERATED {
        VarA,
        VarB,
        VarC
    }
    
    BaseSeq ::= SEQUENCE {
        abc UTF8String,
        def SomeEnum
    }

    SeqButOnlyVarB ::= BaseSeq(WITH COMPONENTS {
        ...,
        def(VarB)
    })
    
    END"
);

#[test]
pub fn does_it_compile() {
    let _ = SomeEnum::VarB;
    let _ = BaseSeq {
        abc: "some-utf8-string".to_string(),
        def: SomeEnum::VarC,
    };
    let _ = SeqButOnlyVarB(BaseSeq {
        abc: "some-utf8-string-again".to_string(),
        def: SomeEnum::VarB,
    });
}

#[test]
pub fn with_components_must_be_transparent() {
    let mut writer1 = UperWriter::default();
    let mut writer2 = UperWriter::default();

    let base_seq = BaseSeq {
        abc: "some-utf8-string-again".to_string(),
        def: SomeEnum::VarB,
    };

    base_seq.write(&mut writer1).unwrap();
    SeqButOnlyVarB(base_seq).write(&mut writer2).unwrap();

    assert_eq!(writer1.into_bytes_vec(), writer2.into_bytes_vec());
}
