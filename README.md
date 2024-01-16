# asn1rs - ASN.1 Compiler for Rust

This crate generates Rust Code and optionally compatible Protobuf and SQL schema files from ASN.1 definitions.
Integration with [serde](https://crates.io/crates/serde) is supported.

The crate can be used as standalone CLI binary or used as library through its API
(for example inside your ```build.rs``` script).


[![Build Status](https://github.com/kellerkindt/asn1rs/workflows/Rust/badge.svg)](https://github.com/kellerkindt/asn1rs/actions?query=workflow%3ARust)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](https://github.com/kellerkindt/asn1rs)
[![Crates.io](https://img.shields.io/crates/v/asn1rs.svg)](https://crates.io/crates/asn1rs)
[![Coverage Status](https://coveralls.io/repos/github/kellerkindt/asn1rs/badge.svg?branch=master)](https://coveralls.io/github/kellerkindt/asn1rs?branch=master)
[![Documentation](https://docs.rs/asn1rs/badge.svg)](https://docs.rs/asn1rs)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](https://github.com/kellerkindt/asn1rs/issues/new)



### Supported Features


| Feature             | Parses  | UPER   | Protobuf   |
| --------------------|:--------|:-------|:-----------|
| ...extensible       | ✔️ yes  | ✔️ yes | 🆗 ignored |
| `SEQUENCE OF`       | ✔️ yes  | ✔️ yes | ✔️ yes     |
| ...`SIZE(A..B)`     | ✔️ yes  | ✔️ yes | 🆗 ignored |
| ...`SIZE(A..B,...)` | ✔️ yes  | ✔️ yes | 🆗 ignored |
| `SET`               | ✔️ yes  | ✔️ yes | ✔️ yes     |
| ...extensible       | ✔️ yes  | ✔️ yes | 🆗 ignored |
| `SET OF`            | ✔️ yes  | ✔️ yes | ✔️ yes     |
| ...`SIZE(A..B)`     | ✔️ yes  | ✔️ yes | 🆗 ignored |
| ...`SIZE(A..B,...)` | ✔️ yes  | ✔️ yes | 🆗 ignored |
| `ENUMERATED`        | ✔️ yes  | ✔️ yes | ✔️ yes     |
| ...extensible       | ✔️ yes  | ✔️ yes | 🆗 ignored |
| `CHOICE`            | ✔️ yes  | ✔️ yes | ✔️ yes     |
| ...extensible       | ✔️ yes  | ✔️ yes | 🆗 ignored |
| `BIT STRING`        | ✔️ yes  | ✔️ yes | ✔️ yes¹    |
| ...`SIZE(A..B)`     | ✔️ yes  | ✔️ yes | 🆗 ignored |
| ...`SIZE(A..B,...)` | ✔️ yes  | ✔️ yes | 🆗 ignored |
| `OCTET STRING`      | ✔️ yes  | ✔️ yes | ✔️ yes     |
| ...`SIZE(A..B)`     | ✔️ yes  | ✔️ yes | 🆗 ignored |
| ...`SIZE(A..B,...)` | ✔️ yes  | ✔️ yes | 🆗 ignored |
| `UTF8String`        | ✔️ yes  | ✔️ yes | ✔️ yes     |
| ...`SIZE(A..B)`     | ✔️ yes  | ✔️ yes | 🆗 ignored |
| ...`SIZE(A..B,...)` | ✔️ yes  | ✔️ yes | 🆗 ignored |
| `IA5String`         | ✔️ yes  | ✔️ yes | ✔️ yes¹    |
| ...`SIZE(A..B)`     | ✔️ yes  | ✔️ yes | 🆗 ignored |
| ...`SIZE(A..B,...)` | ✔️ yes  | ✔️ yes | 🆗 ignored |
| `NumericString`     | ✔️ yes  | ✔️ yes | ✔️ yes¹    |
| ...`SIZE(A..B)`     | ✔️ yes  | ✔️ yes | 🆗 ignored |
| ...`SIZE(A..B,...)` | ✔️ yes  | ✔️ yes | 🆗 ignored |
| `PrintableString`   | ✔️ yes  | ✔️ yes | ✔️ yes¹    |
| ...`SIZE(A..B)`     | ✔️ yes  | ✔️ yes | 🆗 ignored |
| ...`SIZE(A..B,...)` | ✔️ yes  | ✔️ yes | 🆗 ignored |
| `VisibleString`     | ✔️ yes  | ✔️ yes | ✔️ yes¹    |
| ...`SIZE(A..B)`     | ✔️ yes  | ✔️ yes | 🆗 ignored |
| ...`SIZE(A..B,...)` | ✔️ yes  | ✔️ yes | 🆗 ignored |
| `INTEGER`           | ✔️ yes  | ✔️ yes | ✔️ yes     |
| ...`A..B`           | ✔️ yes  | ✔️ yes | ✔️ yes²    |
| ...`A..B,...`       | ✔️ yes  | ✔️ yes | ✔️ yes²    |
| `BOOLEAN`           | ✔️ yes  | ✔️ yes | ✔️ yes     |
| `OPTIONAL`          | ✔️ yes  | ✔️ yes | ✔️ yes     |
| `DEFAULT ...`       | ✔️ yes  |        |            |
| ...`INTEGER`        | ✔️ yes  | ✔️ yes | ✔️ yes¹    |
| ...`*String`        | ✔️ yes  | ✔️ yes | ✔️ yes¹    |
| ...`BOOLEAN`        | ✔️ yes  | ✔️ yes | ✔️ yes¹    |
| ...`ENUMERATED`     | ✔️ yes  | ✔️ yes | ✔️ yes¹    |
| `NULL`              | ✔️ yes  | ✔️ yes | ✔️ yes¹    |
| `IMPORTS..FROM..;`  | ✔️ yes  |        |            |
| `ObjectIdentifiers` | ✔️ yes  |        |            |
| Value References    | ✔️ yes  |        |            |
| ... in Range        | ✔️ yes  |        |            |
| ... in Size         | ✔️ yes  |        |            |
| ... in Default      | ✔️ yes  |        |            |
| `WITH COMPONENTS`   | ✔️ yes  |        |            |

 - ✔️ yes: according to specification
 - ✔️ yes¹: different representation
 - ✔️ yes²: as close as possible to the original specification (sometimes yes, sometimes yes¹)
 - 🔶 not serialized: values are not serialized or deserialized in this case, might break compatibility
 - ⚠️ ignored️: constraint is ignored, this most likely breaks compatibility
 - 🆗 ignored: constraint is ignored but it does not break compatibility
 - ❌ ub: undefined behavior - whatever seems reasonable to prevent compiler errors and somehow transmit the value
 - 🟥 error: fails to compile / translate


#### Supported standards
 -  [📜️ ETSI TS 102 894-2 (PDF)](https://www.etsi.org/deliver/etsi_ts/102800_102899/10289402/01.02.01_60/ts_10289402v010201p.pdf)
    / [🧰 ITS-Container (GIT)](https://forge.etsi.org/rep/ITS/asn1/cdd_ts102894_2/blob/151b191121d05c3b808f5dec14387339730db14f/ITS-Container.asn): \
    ```itu-t(0) identified-organization(4) etsi(0) itsDomain(5) wg1(1) ts(102894) cdd(2) version(2)```
 -  [📜️ ETSI EN 302 637-2 (PDF)](https://www.etsi.org/deliver/etsi_en/302600_302699/30263702/01.03.01_30/en_30263702v010301v.pdf)
    / [🧰 CAM-PDU-Description (GIT)](https://forge.etsi.org/rep/ITS/asn1/cam_en302637_2/blob/7ae4195d48dd468754a50f1a3bb0c2ce976ae15a/CAM-PDU-Descriptions.asn): \
    ```itu-t(0) identified-organization(4) etsi(0) itsDomain(5) wg1(1) en(302637) cam(2) version(2)```
 -  [📜️ ETSI EN 302 637-3 (PDF)](https://www.etsi.org/deliver/etsi_en/302600_302699/30263703/01.02.01_30/en_30263703v010201v.pdf)
    / [🧰 DENM-PDU-Descriptions (GIT)](https://forge.etsi.org/rep/ITS/asn1/denm_en302637_3/blob/29ec748fd9a0e44b91e1896867fa34453781e334/DENM-PDU-Descriptions.asn): \
    ```itu-t(0) identified-organization(4) etsi(0) itsDomain(5) wg1(1) en(302637) denm(1) version(2)```

### CLI usage

It is always helpful to check ```asn1rs --help``` in advance.
The basic usage can be seen blow:

```
asn1rs -t rust directory/for/rust/files some.asn1 messages.asn1
```

```
asn1rs -t proto directory/for/protobuf/files some.asn1 messages.asn1
```

### Example: build.rs

The following example generates Rust and Protobuf files for all ```.asn1```-files in the ```asn/``` directory of a workspace.
While the generated Rust code is written to the ```src/``` directory, the Protobuf files are written to ```proto/```.
Additionally, in this example each generated Rust-Type also receives ```Serialize``` and ```Deserialize``` derive directives (```#[derive(Serialize, Deserialize)]```) for [serde](https://crates.io/crates/serde) integration.

Sample ```build.rs``` file:

```rust
use asn1rs::converter::Converter;
use asn1rs::gen::rust::RustCodeGenerator;

pub fn main() {
    let mut converter = Converter::default();

    // collecting all relevant .asn1 files
    std::fs::read_dir("../protocol/asn")
        .into_iter()
        .flat_map(|read_dir| {
            read_dir
                .into_iter()
                .flat_map(|dir_entry| dir_entry.into_iter())
                .flat_map(|entry| {
                    entry
                        .path()
                        .as_os_str()
                        .to_os_string()
                        .into_string()
                        .into_iter()
                })
                .filter(|entry| entry.ends_with(".asn1"))
        })
        .for_each(|path| {
            println!("cargo:rerun-if-changed={}", path);
            if let Err(e) = converter.load_file(&path) {
                panic!("Loading of .asn1 file failed {}: {:?}", path, e);
            }
        });

    // writing the .rs files into src with serde_derive support
    // feature flags decide whether additional code for protobuf is generated
    if let Err(e) = converter.to_rust("src/", |generator: &mut RustCodeGenerator| {
        generator.add_global_derive("Serialize"); // Adds serde_derive support: #[derive(Serialize)]
        generator.add_global_derive("Deserialize"); // Adds serde_derive support: #[derive(Deserialize)]
    }) {
        panic!("Conversion to rust failed: {:?}", e);
    }

    // OPTIONAL: writing the .proto representation to ../protocol/proto
    if let Err(e) = converter.to_protobuf("../protocol/proto/") {
        panic!("Conversion to proto failed: {:?}", e);
    }
}

```

### Example: Inlining ASN.1 with procedural macros

Minimal example by inlining the ASN.1 definition. For more examples see [tests/](tests).
```rust
use asn1rs::prelude::*;

asn_to_rust!(
    r"BasicInteger DEFINITIONS AUTOMATIC TAGS ::=
    BEGIN
    
    RangedMax ::= Integer (0..MAX)
    
    NotRanged ::= Integer
    
    END"
);

#[test]
fn test_write_read() {
    // inner INTEGER identified as u64
    let value = NotRanged(123_u64);

    let mut writer = UperWriter::default();
    writer.write(&value).expect("Failed to serialize");

    let mut reader = writer.into_reader();
    let value2 = reader.read::<NotRanged>().expect("Failed to deserialize");
    
    assert_eq!(value, value2);
}

#[test]
fn test_constraint_eq() {
    // these types should normally not be accessed, but in this exampled they show
    // the way the ASN.1 constraints are encoded with the Rust type system.
    use asn1rs::syn::numbers::Constraint;
    assert_eq!(
        ___asn1rs_RangedMaxField0Constraint::MIN,
        ___asn1rs_NotRangedField0Constraint::MIN,
    );
    assert_eq!(
        ___asn1rs_RangedMaxField0Constraint::MAX,
        ___asn1rs_NotRangedField0Constraint::MAX,
    );
}
```


### Example: ASN.1-Definition converted to Rust and Protobuf

Minimal example showcasing what is being generated from an ASN.1 definition:

```asn
MyMessages DEFINITIONS AUTOMATIC TAGS ::=
BEGIN

Header ::= SEQUENCE {
    timestamp    INTEGER (0..1209600000)
}

END
```

The generated Rust file:

```rust
use asn1rs::prelude::*;

#[asn(sequence)]
#[derive(Default, Debug, Clone, PartialEq, Hash)]
pub struct Header {
    #[asn(integer(0..1209600000))] pub timestamp: u32,
}
```

The generated protobuf file (optional):

```proto
syntax = 'proto3';
package my.messages;

message Header {
    uint32 timestamp = 1;
}
```

#### Example: Raw uPER usage
The module ```asn1rs::io``` exposes (de-)serializers and helpers for direct usage without ASN.1 definition:
```rust
use asn1rs::prelude::*;
use asn1rs::io::per::unaligned::buffer::BitBuffer;

let mut buffer = BitBuffer::default();
buffer.write_bit(true).unwrap();
buffer.write_utf8_string("My UTF8 Text").unwrap();

send_to_another_host(buffer.into::<Vec<u8>>()):
```

#### Example: Raw Protobuf usage
The module ```asn1rs::io::protobuf``` exposes (de-)serializers for protobuf usage:
```rust
use asn1rs::io::protobuf::*;

let mut buffer = Vec::default();
buffer.write_varint(1337).unwrap();
buffer.write_string("Still UTF8 Text").unwrap();

send_to_another_host(buffer):
``` 

### Extending Rust Codegen

```rust
use asn1rs::model::model::gen::rust::RustCodeGenerator;
use asn1rs::model::model::gen::rust::GeneratorSupplement;

impl GeneratorSupplement<Rust> for MyRustCodeGeneratorExtension {
    // .. implement the trait
}

fn main() {
    let model: Model<Rust> = ... ;
    let (_file_name, file_content) = RustCodeGenerator::from(model)
        .to_string_with_generators(&[&MyRustCodeGeneratorExtension])
        .into_iter()
        .next()
        .unwrap();
}
```

#### Finding deserialization error origins

For a more detailed report on deserialization errors, enable the `descriptive-deserialize-errors` feature.
With this feature flag more details will be memorized while deserializing your data (see `ScopeDescription`) - thus causing a performance penalty -
but it will list intermediate results with the error origin and the current location in the type hierarchy when displaying the error ( `println!("{e}")`);

#### TODO
Things to do at some point in time (PRs are welcome)

  - generate a proper rust module hierarchy from the modules' object-identifier
  - support ```#![no_std]```
  - refactor / clean-up (rust) code-generators (most will be removed in v0.3.0)
  - support more encoding formats of ASN.1 (help is welcome!)


#### License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
</sub>


##### Origin
<sub>
This crate was initially developed during a research project at IT-Designers GmbH (http://www.it-designers.de).
</sub>

[`pipelining`]: https://docs.rs/tokio-postgres/0.5.2/tokio_postgres/#pipelining
