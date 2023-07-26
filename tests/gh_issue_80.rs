#![recursion_limit = "512"]

mod test_utils;

use test_utils::*;

asn_to_rust!(
    r"TestTypes DEFINITIONS AUTOMATIC TAGS ::= BEGIN                                                                                                                                                                                                                                                                                                                                                                                                                                                                     
      MySequence ::= SEQUENCE {                                                                                                                                                                                                                                    
        val1 INTEGER,                                                                                                                                                                                                                                              
        val2 INTEGER,                                                                                                                                                                                                                                              
        ...                                                                                                                                                                                                                                                        
      }                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                
    END"
);

#[test]
fn test() {
    let my_sequence = MySequence {
        val1: 1020,
        val2: 1080,
    };

    let mut writer = UperWriter::default();
    writer.write(&my_sequence).unwrap();
    let mut reader = writer.as_reader();
    let deser = reader.read::<MySequence>().unwrap();
    assert_eq!(my_sequence, deser);
}
