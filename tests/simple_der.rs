pub use asn1rs::prelude::*;

asn_to_rust!(
    r"World-Schema DEFINITIONS ::=
BEGIN
  Rocket ::= SEQUENCE
  {
     range     INTEGER, -- huge (see a special directive above)
     name      UTF8String (SIZE(1..16)),
     message   UTF8String OPTIONAL ,
     fuel      ENUMERATED {solid, liquid, gas},
     speed     CHOICE
     {
        mph    [0] INTEGER,
        kmph   [1] INTEGER
     }  OPTIONAL,
     payload   SEQUENCE OF UTF8String
  }
END"
);

#[test]
fn simple_der() {
    let der_content = b"0$\x02\x05\x07\xec=\xaf\x94\x0c\x06Falcon\n\x01\x00\xa0\x04\x02\x02FP0\n\x0c\x03Car\x0c\x03GPS".to_vec();
    let mut reader = DerReader::from_bits(der_content);
    let result = reader.read::<Rocket>().unwrap();
    println!("Decoded:");
    println!("{:#?}", result);

    assert_eq!(result.range, 34028236692u64);
    assert_eq!(result.name, "Falcon");
    assert_eq!(result.message, None);
    assert_eq!(result.fuel, RocketFuel::Solid);
    assert_eq!(result.speed, Some(RocketSpeed::Mph(18000)));
    assert_eq!(result.payload, vec!["Car", "GPS"]);
}
