# 0.2.0-alpha3 (Oct 14, 2020)

### Fixes
- Missing CI checks on non-default features

### Added
- Support for `SET`s and `SET OF`s\*
- Support for extensible `SET`s
- Support for `SIZE` constraints for `SET OF`s
- `TagResolver` to properly resolve Tags of ASN.1 types 
- `syn::common::Constraint` which has `const TAG: Tag` and implementation for all generated constraint types 

\* For `SET OF` only BASIC-PER encoding is supported currently, see [#20](https://github.com/kellerkindt/asn1rs/issues/20)

### Changes
- The ASN.1 `OPTIONAL` type is now represented as `optional` instead of `option` in `#[asn(..)]`
- The protobuf serializer is now optional and can be enabled with the `protobuf` feature flag

# 0.2.0-alpha2 (Sep 03, 2020)

This release includes a lot of refactoring and new features.
With these changes, it is now possible to use the following two ASN.1 standards:

 - 🎉 ```itu-t(0) identified-organization(4) etsi(0) itsDomain(5) wg1(1) ts(102894) cdd(2) version(1)``` (ITS-Container)
 - 🎉 ```itu-t(0) identified-organization(4) etsi(0) itsDomain(5) wg1(1) en(302637) cam(2) version(1)``` (CAM-PDU-Descriptions)


### Fixes
- Implement the canonical order for tags (ITU-T X.680 | ISO/IEC 8824-1, 8.6)

### Added
- Support for `INTEGER` constants
- Support for extensible `SEQUENCE`s
- Support for extensible `INTEGER`s
- Support for `BIT STRING`, as well as the `SIZE` constraint, constants, and the extensible flag
- Support for `IA5String`, as well as the `SIZE` constraint, and the extensible flag
- Support for `SIZE` constraints for `OCTET STRING`s
- Support for `SIZE` constraints for `UTF8String`s
- Support for `SIZE` constraints for `SEQUENCE OF`s
- ASN.1 Support Overview to README

### Changes
- Parse/Accept ObjectIdentifier in `FROM` directives and module definitions
- The whole module `crate::io::uper` is now **deprecated**
- Reimplemented all low level uPER functions - this time strictly according to specification and using names mentioned there, see ```crate::io::per```
- Better prepare for alternative encoding rules (especially aligned PER, although this is no specific goal)
- Help the compiler in figuring out where const evaluations are possible (see `const_*!` macros)
- Lots of `#[inline]` hinting 


# 0.2.0-alpha1 (May 13, 2020)

### Fixes
- lots of smaller and niche parsing errors

### Added
- support for ASN-extensible `CHOICE` and `ENUMERATED` types  
- `Reader`, `Writer` traits to (de)serialize based on the visitor pattern, asn attribute annotation, see [showcase] and [proc_macro_attribute]. This will allow further ASN encodings to be implemented without further code generation (to be clear, this not on the roadmap for now, but PRs are welcome).

### Changes
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