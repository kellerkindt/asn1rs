# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
### Changed
 - Implement UperReader for ScopedBitReader over Bits ([aa6b083], [gh-81], [@jkalez])
### Deprecated
### Removed
### Fixed
### Security

[@jkalez]: https://github.com/jkalez
[gh-81]: https://github.com/kellerkindt/asn1rs/issues/81
[aa6b083]: https://github.com/kellerkindt/asn1rs/commit/aa6b08319be382f310b323343d0c49268e17af84

## [0.3.1] - 2023-07-27
### Fixed
- Fix `asn1rs-model` dependency of `asn1rs-macros` ([3986ce9](https://github.com/kellerkindt/asn1rs/commit/3986ce9))
- Fix `asn1rs-macros` dependency of `asn1rs` ([5d4f4ec](https://github.com/kellerkindt/asn1rs/commit/5d4f4ec))
- Fix `asn1rs-*` version dependency ([9825de9](https://github.com/kellerkindt/asn1rs/commit/9825de9))

## [0.3.0] - 2023-07-27

### Added
- More tests ([aa6e4f1](https://github.com/kellerkindt/asn1rs/commit/aa6e4f1) [4568b1f](https://github.com/kellerkindt/asn1rs/commit/4568b1f) [ee74d2b](https://github.com/kellerkindt/asn1rs/commit/ee74d2b))
- Impl PartialEq for Error ([11a8312](https://github.com/kellerkindt/asn1rs/commit/11a8312))
- Feature `descriptive-deserialize-errors` ([7f3e11a](https://github.com/kellerkindt/asn1rs/commit/7f3e11a))
- Ignore multiline comments ([f6a6e86](https://github.com/kellerkindt/asn1rs/commit/f6a6e86), [gh-78](https://github.com/kellerkindt/asn1rs/issues/78), thanks [@Nicceboy](https://github.com/Nicceboy))
### Changed
- Update `syn` version to 1.0.109 ([96df6b2](https://github.com/kellerkindt/asn1rs/commit/96df6b2), [5b8c49f](https://github.com/kellerkindt/asn1rs/commit/5b8c49f))
- Collect Backtrace on insufficient source / destination buffer ([4a2358e](https://github.com/kellerkindt/asn1rs/commit/4a2358e))
- Update `bytes` to v1.0 ([6765543](https://github.com/kellerkindt/asn1rs/commit/6765543))
- Update `postgres` to v0.19.1 ([63131d0](https://github.com/kellerkindt/asn1rs/commit/63131d0))
- Try to generate the protobuf package based on the OID before using the path ([624a697](https://github.com/kellerkindt/asn1rs/commit/624a697), [1c460aa](https://github.com/kellerkindt/asn1rs/commit/1c460aa))
- Prefix protobuf enum variants with their type name to pevent collisions (c sibling rule) ([93214ac](https://github.com/kellerkindt/asn1rs/commit/93214ac))
- Use `bytes` instead of `bit_vec` in protobuf for ASN.1 `BIT STRING` ([6af109b](https://github.com/kellerkindt/asn1rs/commit/6af109b))
- Rewrite `ProtobufReader` to properly handle out-of-order and missing tags ([681ff2b](https://github.com/kellerkindt/asn1rs/commit/681ff2b))
- Ignore non-.rs files for `proc_macro_coverage_hack` ([c925df6](https://github.com/kellerkindt/asn1rs/commit/c925df6), [1319241](https://github.com/kellerkindt/asn1rs/commit/1319241))
- Limit (P-)SQL Type names to not exceed 63 characters ([d662e56](https://github.com/kellerkindt/asn1rs/commit/d662e56), [c7b7401](https://github.com/kellerkindt/asn1rs/commit/c7b7401), [2d69ca1](https://github.com/kellerkindt/asn1rs/commit/2d69ca1), [gh-75](https://github.com/kellerkindt/asn1rs/issues/75))
- Split `Error` type with boxed inner `ErrorKind` ([7f3e11a](https://github.com/kellerkindt/asn1rs/commit/7f3e11a), [2471539](https://github.com/kellerkindt/asn1rs/commit/2471539))
- Add `#[doc(hidden)]` to internal constraint types generated via macro, disable with feature `generate-internal-docs` ([20767e7](https://github.com/kellerkindt/asn1rs/commit/20767e7))
- Derive `Default` for enums ([b189ef6](https://github.com/kellerkindt/asn1rs/commit/b189ef6))
- Make the `CHANGELOG.md` adhere more closely to the [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) template.
### Deprecated
### Removed
### Fixed
- Fix `Scope::exhausted` for `Scope::ExtensibleSequences` ([6c9f334](https://github.com/kellerkindt/asn1rs/commit/6c9f334))
- Fix call to `BitVec::ensure_vec_large_enough` and add pub accessor ([a4cec0e](https://github.com/kellerkindt/asn1rs/commit/a4cec0e))
- Fix extensible fields in `SEQUENCE` and `SET` not treat as optional fields ([75f2882](https://github.com/kellerkindt/asn1rs/commit/75f2882))
- Fix `read_length_determinant` for fragmented sizes ([0083f3b](https://github.com/kellerkindt/asn1rs/commit/0083f3b))
- Fix `write_length_determinant` the size 16kib ([5061379](https://github.com/kellerkindt/asn1rs/commit/5061379))
- Fix fragmented write_octetstring and add return value to `write_length_determinant` ([30dfd73](https://github.com/kellerkindt/asn1rs/commit/30dfd73))
- Fix usage of `rustdoc::broken_intra_doc_links` ([c4f55dc](https://github.com/kellerkindt/asn1rs/commit/c4f55dc), [a4579cc](https://github.com/kellerkindt/asn1rs/commit/a4579cc))
- Fix name duplication for inline choice types ([e8aa191](https://github.com/kellerkindt/asn1rs/commit/e8aa191), [gh-75](https://github.com/kellerkindt/asn1rs/issues/75))
- Prevent two panics (thanks fuzzer) in `PacketRead` impl for `BitRead` ([e3f5323](https://github.com/kellerkindt/asn1rs/commit/e3f5323))
### Security


## [0.2.2] - 2021-05-03

This release includes a lot of refactoring and new features. With these changes, it is now possible to use the following ASN.1 standard:

- 🎉 ```itu-t(0) identified-organization(4) etsi(0) itsDomain(5) wg1(1) en(302637) denm(1) version(2)``` (DENM)

### Fixed
- Properly parse (extensible) `SIZE` without a range value.
- Fix `ProtobufWriter` on slices
- Fix clippy v1.51 remarks.
- Fix `rust_variant_name` and `rust_struct_or_enum_name` for two consecutively upper case letters.
- Fix ` rust_constant_name` not inserting _ around numbers.
 
### Added
- Lots of regression tests for utility functions. 
- Regression tests for `ProtobufWriter` on slices.
- Basic support for `DEFAULT` (for `INTEGER`, *`String`, `BOOLEAN`, `ENUMERATED` and some tuple/transparent types)
- Parsing (and ignoring) of `WITH COMPONENTS` constraints
- Resolving symbols across multiple module descriptions.
- Support ASN `NULL` type

### Changed
- Generate constants besides structs. This has the advantage that constants are clearly visible.
- Performance improvement while converting the Model to Rust: Do not allocating structs that are thrown away anyway (call `RustType::as_inner_type` instead of `::clone` & `RustType::into_inner_type`).

### Removed
- **Legacy** protobuf and uper codegen

## [0.2.1] - 2021-03-22

This release refactors `Model<Asn>` which is now represented as `Model<Asn<Unresolved>>` and `Model<Asn<Resolved>>`.
This change allows Value-References in SIZE and RANGE constraints (see [gh-50](https://github.com/kellerkindt/asn1rs/issues/50) [gh-49](https://github.com/kellerkindt/asn1rs/issues/49)) without a failable `to_rust()` converter.

### Fixed
- No longer choke on empty `SEQUENCE` definitions (see [gh-44](https://github.com/kellerkindt/asn1rs/issues/44))

### Added
- Parsing and resolving Value-References in SIZE and RANGE constraints (see [gh-50](https://github.com/kellerkindt/asn1rs/issues/50) [gh-49](https://github.com/kellerkindt/asn1rs/issues/49)) 

### Changed
- **BREAKING**: `Model::try_from(Tokenizer)` now returns `Model<Asn<Unresolved>>`. To convert to rust (`Model::<Asn<Resolved>>::to_rust(&self) -> Model<Rust>`) the fallible function `Model::<Asn<Unresolved>>::try_resolve(&self) -> Model<Asn<Resolved>>` must be called first.

```rust
let model_rust = Model::try_from(asn_tokens)
    .expect("Failed to parse tokens")
    .try_resolve()                                  <--------------+--- new
    .expect("Failed to resolve at least one value reference")  <---+
    .to_rust();
```
 
## [0.2.0] - 2021-02-03

This release includes a lot of refactoring and new features.
With these changes, it is now possible to use the following two ASN.1 standards:

- 🎉 ```itu-t(0) identified-organization(4) etsi(0) itsDomain(5) wg1(1) ts(102894) cdd(2) version(1)``` (ITS-Container)
- 🎉 ```itu-t(0) identified-organization(4) etsi(0) itsDomain(5) wg1(1) en(302637) cam(2) version(1)``` (CAM-PDU-Descriptions)

The serialization and deserialization process was completely revamped to replace the code generation that uses string concatenation and instead utilize (smaller) proc-macros and types for it.
The previous - now called legacy codegen - is still available, but deprecated and hidden behind the `legacy-uper-codegen` and `legacy-protobuf-codegen` feature.
It will be **removed in 0.3.0**.

Feel free to visit [the tests](tests) to learn about the new usage. You might want to start with the [showcase].

### Fixed
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

### Changed
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
