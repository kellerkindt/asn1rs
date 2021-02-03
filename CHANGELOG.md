# Version 0.2.0 (2021-02-03)

This release includes a lot of refactoring and new features.
With these changes, it is now possible to use the following two ASN.1 standards:

- 🎉 ```itu-t(0) identified-organization(4) etsi(0) itsDomain(5) wg1(1) ts(102894) cdd(2) version(1)``` (ITS-Container)
- 🎉 ```itu-t(0) identified-organization(4) etsi(0) itsDomain(5) wg1(1) en(302637) cam(2) version(1)``` (CAM-PDU-Descriptions)

The serialization and deserialization process was completely revamped to replace the code generation that uses string concatenation and instead utilize (smaller) proc-macros and types for it.
The previous - now called legacy codegen - is still available, but deprecated and hidden behind the `legacy-uper-codegen` and `legacy-protobuf-codegen` feature.
It will be **removed in 0.3.0**.

Feel free to visit [the tests](tests) to learn about the new usage. You might want to start with the [showcase].

### Fixes
- lots of smaller and niche parsing errors
- Implement the canonical order for tags (ITU-T X.680 | ISO/IEC 8824-1, 8.6)
- Missing CI checks on non-default features

### Added
- support for ASN-extensible `CHOICE` and `ENUMERATED` types
- `Reader`, `Writer` traits to (de)serialize based on the visitor pattern, asn attribute annotation, see [showcase] and [proc_macro_attribute]. This will allow further ASN encodings to be implemented without further code generation (to be clear, this not on the roadmap for now, but PRs are welcome).
- Support for `INTEGER` constants
- Support for extensible `SEQUENCE`s
- Support for extensible `INTEGER`s
- Support for `BIT STRING`, as well as the `SIZE` constraint, constants, and the extensible flag
- Support for `IA5String`, as well as the `SIZE` constraint, and the extensible flag
- Support for `SIZE` constraints for `OCTET STRING`s
- Support for `SIZE` constraints for `UTF8String`s
- Support for `SIZE` constraints for `SEQUENCE OF`s 
- Support for `SET`s and `SET OF`s\*
- Support for extensible `SET`s
- Support for `SIZE` constraints for `SET OF`s
- `TagResolver` to properly resolve Tags of ASN.1 types 
- `syn::common::Constraint` which has `const TAG: Tag` and implementation for all generated constraint types
- CI checks for specific feature combinations



\* For `SET OF` only BASIC-PER encoding is supported currently, see [#20](https://github.com/kellerkindt/asn1rs/issues/20)

### Changes
- Added ASN.1 Support Overview to README
- Deprecated `UperSerializer` which generates a lot of complex code for (uper-)serialization. Instead general purpose and less complex code that is based on the visitor pattern will be generated. See [showcase] and commits linked to [#11]. This also allows to write ASN serializable structures without writing ASN itself (see [proc_macro_attribute]):

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
- Parse/Accept ObjectIdentifier in `FROM` directives and module definitions
- The module `crate::io::uper` is now **deprecated**
- Reimplemented all low level uPER functions - this time strictly according to specification and using names mentioned there, see ```crate::io::per```
- Better prepare for alternative encoding rules (especially aligned PER, although this is no specific goal)
- Help the compiler in figuring out where const evaluations are possible (see `const_*!` macros)
- Lots of `#[inline]` hinting
- The ASN.1 `OPTIONAL` type is now represented as `optional` instead of `option` in `#[asn(..)]`
- The protobuf serializer is now optional and can be enabled with the `protobuf` feature flag
- Deprecated `Protobuf` trait which is replaced by `ProtobufReader` and `ProtobufWriter` that use the common `Readable` and `Writable` traits

[showcase]: tests/showcase.rs
[proc_macro_attribute]: tests/basic_proc_macro_attribute.rs
[#11]: https://github.com/kellerkindt/asn1rs/issues/11