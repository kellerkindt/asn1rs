# 0.2.0-alpha1 (May 13, 2020)

### Fixes
- lots of smaller and niche parsing errors

### Added
- support for ASN-extensible `CHOICE` and `ENUMERATED` types  
- `Reader`, `Writer` traits to (de)serialize based on the visitor pattern, asn attribute annotation, see [showcase] and [proc_macro_attribute]. This will allow further ASN encodings to be implemented without further code generation (to be clear, this not on the roadmap for now, but PRs are welcome).

## Changes
- deprecated `UperSerializer` which generates a lot of complex code for (uper-)serialization. Instead general purpose and less complex code that is based on the visitor pattern will be generated. See [showcase] and commits linked to [#11]. This also allows to write ASN serializable structures without writing ASN itself (see [proc_macro_attribute]):

```rust
#[asn(sequence)]
#[derive(Debug, PartialOrd, PartialEq)]
pub struct Pizza {
    #[asn(integer(1..4))]
    size: u8,
    #[asn(complex(Topping))]
    topping: Topping,
}

#[test]
fn pizza_test_uper_1() {
    let mut uper = UperWriter::default();
    let pizza = Pizza {
        size: 2,
        topping: Topping::NotPineapple,
    };
    uper.write(&pizza).unwrap();
    // https://asn1.io/asn1playground/
    assert_eq!(&[0x40], uper.byte_content());
    assert_eq!(4, uper.bit_len());
    let mut uper = uper.into_reader();
    assert_eq!(pizza, uper.read::<Pizza>().unwrap());
    assert_eq!(0, uper.bits_remaining());
}

```

[showcase]: tests/showcase.rs
[proc_macro_attribute]: tests/basic_proc_macro_attribute.rs
[#11]: https://github.com/kellerkindt/asn1rs/issues/11