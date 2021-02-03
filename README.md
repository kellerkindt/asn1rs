# asn1rs - ASN.1 Compiler for Rust

This crate generates Rust Code and optionally compatible Protobuf and SQL schema files from ASN.1 definitions.
Integration with [serde](https://crates.io/crates/serde) is supported.

The crate can be used as standalone CLI binary or used as library through its API
(for example inside your ```build.rs``` script).


[![Build Status](https://github.com/kellerkindt/asn1rs/workflows/Rust/badge.svg)](https://github.com/kellerkindt/asn1rs/actions?query=workflow%3ARust)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](https://github.com/kellerkindt/asn1rs)
[![Crates.io](https://img.shields.io/crates/v/asn1rs.svg)](https://crates.io/crates/asn1rs)
[![Documentation](https://docs.rs/asn1rs/badge.svg)](https://docs.rs/asn1rs)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](https://github.com/kellerkindt/asn1rs/issues/new)



### Supported Features


| Feature             | Parses  | UPER    | Protobuf    | PSQL        | Async PSQL | UPER Legacy\*     |
| --------------------|:--------|:--------|:------------|:------------|:-----------|------------------:|
| `SEQUENCE`          | ✔️ yes  | ✔️ yes  | ✔️ yes      | ✔️ yes      | ✔️ yes     | ✔️ yes             | 
| ...extensible       | ✔️ yes  | ✔️ yes  | 🆗 ignored  | 🆗 ignored  | 🆗 ignored | 🔶 not serialized  | 
| `SEQUENCE OF`       | ✔️ yes  | ✔️ yes  | ✔️ yes      | ✔️ yes      | ✔️ yes     | ✔️ yes             | 
| ...`SIZE(A..B)`     | ✔️ yes  | ✔️ yes  | 🆗 ignored  | 🆗 ignored  | 🆗 ignored | ⚠️ ignored️         | 
| ...`SIZE(A..B,...)` | ✔️ yes  | ✔️ yes  | 🆗 ignored  | 🆗 ignored  | 🆗 ignored | ⚠️ ignored️         |
| `SET`               | ✔️ yes  | ✔️ yes  | ✔️ yes      | ✔️ yes      | ✔️ yes     | ⚠️ ignored️         | 
| ...extensible       | ✔️ yes  | ✔️ yes  | 🆗 ignored  | 🆗 ignored  | 🆗 ignored | 🔶 not serialized  | 
| `SET OF`            | ✔️ yes  | ✔️ yes  | ✔️ yes      | ✔️ yes      | ✔️ yes     | ⚠️ ignored️         | 
| ...`SIZE(A..B)`     | ✔️ yes  | ✔️ yes  | 🆗 ignored  | 🆗 ignored  | 🆗 ignored | ⚠️ ignored️         | 
| ...`SIZE(A..B,...)` | ✔️ yes  | ✔️ yes  | 🆗 ignored  | 🆗 ignored  | 🆗 ignored | ⚠️ ignored️         | 
| `ENUMERATED`        | ✔️ yes  | ✔️ yes  | ✔️ yes      | ✔️ yes      | ✔️ yes     | ✔️ yes             |           
| ...extensible       | ✔️ yes  | ✔️ yes  | 🆗 ignored  | 🆗 ignored  | 🆗 ignored | 🔶 not serialized  |           
| `CHOICE`            | ✔️ yes  | ✔️ yes  | ✔️ yes      | ✔️ yes      | ✔️ yes     | ✔️ yes             |           
| ...extensible       | ✔️ yes  | ✔️ yes  | 🆗 ignored  | 🆗 ignored  | 🆗 ignored | 🔶 not serialized  | 
| `BIT STRING`        | ✔️ yes  | ✔️ yes  | ✔️ yes¹   | ✔️ yes¹   | ✔️ yes¹  | ✔️ yes             | 
| ...`SIZE(A..B)`     | ✔️ yes  | ✔️ yes  | 🆗 ignored  | 🆗 ignored  | 🆗 ignored | ⚠️ ignored         | 
| ...`SIZE(A..B,...)` | ✔️ yes  | ✔️ yes  | 🆗 ignored  | 🆗 ignored  | 🆗 ignored | ⚠️ ignored         | 
| `OCTET STRING`      | ✔️ yes  | ✔️ yes  | ✔️ yes      | ✔️ yes      | ✔️ yes     | ✔️ yes             | 
| ...`SIZE(A..B)`     | ✔️ yes  | ✔️ yes  | 🆗 ignored  | 🆗 ignored  | 🆗 ignored | ⚠️ ignored         | 
| ...`SIZE(A..B,...)` | ✔️ yes  | ✔️ yes  | 🆗 ignored  | 🆗 ignored  | 🆗 ignored | ⚠️ ignored         |   
| `UTF8String`        | ✔️ yes  | ✔️ yes  | ✔️ yes      | ✔️ yes      | ✔️ yes     | ✔️ yes             | 
| ...`SIZE(A..B)`     | ✔️ yes  | ✔️ yes  | 🆗 ignored  | 🆗 ignored  | 🆗 ignored | ⚠️ ignored         | 
| ...`SIZE(A..B,...)` | ✔️ yes  | ✔️ yes  | 🆗 ignored  | 🆗 ignored  | 🆗 ignored | ⚠️ ignored         | 
| `IA5String`         | ✔️ yes  | ✔️ yes  | ✔️ yes¹   | ✔️ yes¹   | ✔️ yes¹  | ❌ ub              | 
| ...`SIZE(A..B)`     | ✔️ yes  | ✔️ yes  | 🆗 ignored  | 🆗 ignored  | 🆗 ignored | ❌ ub              | 
| ...`SIZE(A..B,...)` | ✔️ yes  | ✔️ yes  | 🆗 ignored  | 🆗 ignored  | 🆗 ignored | ❌ ub              |   
| `INTEGER`           | ✔️ yes  | ✔️ yes  | ✔️ yes      | ✔️ yes      | ✔️ yes     | ✔️ yes             |
| ...`A..B`           | ✔️ yes  | ✔️ yes  | ✔️ yes²   | ✔️ yes²   | ✔️ yes²  | ✔️ yes             |
| ...`A..B,...`       | ✔️ yes  | ✔️ yes  | ✔️ yes²   | ✔️ yes²   | ✔️ yes²  | ⚠️ ignored         |
| `BOOLEAN`           | ✔️ yes  | ✔️ yes  | ✔️ yes      | ✔️ yes      | ✔️ yes     | ✔️ yes             |
| `OPTIONAL`          | ✔️ yes  | ✔️ yes  | ✔️ yes      | ✔️ yes      | ✔️ yes     | ✔️ yes             |
| `IMPORTS..FROM..;`  | ✔️ yes  |         |             |             |            |                    |
| `ObjectIdentifiers` | ✔️ yes  |         |             |             |            |                    |


 - ✔️ yes: according to specification
 - ✔️ yes¹: different representation
 - ✔️ yes²: as close as possible to the original specification (sometimes yes, sometimes yes¹)
 - 🔶 not serialized: values are not serialized or deserialized in this case, might break compatibility
 - ⚠️ ignored️: constraint is ignored, this most likely breaks compatibility
 - 🆗 ignored: constraint is ignored but it does not break compatibility
 - ❌ ub: undefined behavior - whatever seems reasonable to prevent compiler errors and somehow transmit the value
 - 🟥 error: fails to compile / translate

\*The legacy UPER Reader/Writer is deprecated and will be removed in version 0.3.0

**TLDR**
- The new (v0.2.0) UPER Reader/Writer supports all listed features
- Protobuf, sync&async PSQL ignore most constraints
- The legacy UPER Reader/Writer does not support all features (pre v0.2.0)

#### Supported standards
 -  [ETSI TS 102 894-2 / ITS-Container](https://www.etsi.org/deliver/etsi_ts/102800_102899/10289402/01.02.01_60/ts_10289402v010201p.pdf): \
    ```itu-t(0) identified-organization(4) etsi(0) itsDomain(5) wg1(1) ts(102894) cdd(2) version(1)```
 -  [ETSI EN 302 637-2 / ITS-CAM](https://www.etsi.org/deliver/etsi_en/302600_302699/30263702/01.03.01_30/en_30263702v010301v.pdf): \
    ```itu-t(0) identified-organization(4) etsi(0) itsDomain(5) wg1(1) en(302637) cam(2) version(1)```

### CLI usage

It is always helpful to check ```asn1rs --help``` in advance.
The basic usage can be seen blow:

```
asn1rs -t rust directory/for/rust/files some.asn1 messages.asn1
```

```
asn1rs -t proto directory/for/protobuf/files some.asn1 messages.asn1
```

```
asn1rs -t sql directory/for/sql/schema/files some.asn1 messages.asn1
```

### Example: build.rs

The following example generates Rust, Protobuf and SQL files for all ```.asn1```-files in the ```asn/``` directory of a workspace.
While the generated Rust code is written to the ```src/``` directory, the Protobuf files are written to ```proto/``` and the SQL files are written to ```sql/ ```.
Additionally, in this example each generated Rust-Type also receives ```Serialize``` and ```Deserialize``` derive directives (```#[derive(Serialize, Deserialize)]```) for [serde](https://crates.io/crates/serde) integration.

Sample ```build.rs``` file:

```rust
use asn1rs::converter::Converter;
use asn1rs::gen::rust::RustCodeGenerator;
use asn1rs::gen::sql::SqlDefGenerator;

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
    // feature flags decide whether additional code for protobuf and (async) psql is generated
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

    // OPTIONAL: writing the .sql schema files to ../protocol/sql
    if let Err(e) = converter.to_sql_with(
        "../protocol/sql/",
        SqlDefGenerator::default() // optional parameter, alternatively see Converter::to_sql(&self, &str)
            .optimize_tables_for_write_performance() // optional
            .wrap_primary_key_on_overflow(), // optional
    ) {
        panic!("Conversion to sql failed: {:?}", e);
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


### Example: ASN.1-Definition converted to Rust, Protobuf and SQL

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

// only with the feature "async-psql": Insert and query functions for async PostgreSQL
impl Header {
    pub async fn apsql_retrieve_many(context: &apsql::Context<'_>, ids: &[i32]) -> Result<Vec<Self>, apsql::Error> { /*..*/ }
    pub async fn apsql_retrieve(context: &apsql::Context<'_>, id: i32) -> Result<Self, apsql::Error> { /*..*/ }
    pub async fn apsql_load(context: &apsql::Context<'_>, row: &apsql::Row) -> Result<Self, apsql::Error> { /*..*/ }
    pub async fn apsql_insert(&self, context: &apsql::Context<'_>) -> Result<i32, apsql::PsqlError> { /*..*/ }
}

// only with the feature "psql": Insert and query functions for non-async PostgreSQL
impl PsqlRepresentable for Header { /*..*/ }
impl PsqlInsertable for Header { /*..*/ }
impl PsqlQueryable for Header { /*..*/ }
```

The generated protobuf file (optional):

```proto
syntax = 'proto3';
package my.messages;

message Header {
    uint32 timestamp = 1;
}
```

The generated SQL file (optional):

```sql
DROP TABLE IF EXISTS Header CASCADE;

CREATE TABLE Header (
    id SERIAL PRIMARY KEY,
    timestamp INTEGER NOT NULL
);
```

#### Example: Usage of async postgres
NOTE: This requires the `async-psql` feature.

Using async postgres allows the message - or the batched messages - to take advantage of [`pipelining`].
This can provide a significant speedup for deep message types (personal experience this is around 26%) compared to the synchronous/blocking postgres implementation.

```rust
use asn1rs::io::async_psql::*;
use tokio_postgres::NoTls;

#[tokio::main]
async fn main() {
    let transactional = true;
    let (mut client, connection) = tokio_postgres::connect(
        "host=localhost user=postgres application_name=psql_async_demo",
        NoTls,
    )
        .await
        .expect("Failed to connect");

    tokio::spawn(connection);
  

    let context = if transactional {
        let transaction = client
            .transaction()
            .await
            .expect("Failed to open a new transaction");
        Cache::default().into_transaction_context(transaction)
    } else {
        Cache::default().into_client_context(client)
    };

    // using sample message from above
    let message = Header {
        timestamp: 1234,
    };
   
    // This issues all necessary insert statements on the given Context and
    // because it does not require exclusive access to the context, you can
    // issue multiple inserts and await them concurrently with for example
    // tokio::try_join, futures::try_join_all or the like. 
    let id = message.apsql_insert(&context).await.expect("Insert failed");
    
    // This disassembles the context, allowing the Transaction to be committed
    // or rolled back. This operation also optimizes the read access to
    // prepared statements of the Cache. If you do not want to do that, then call
    // Context::split_unoptimized instead.
    // You can also call `Cache::optimize()` manually to optimize the read access
    // to the cached prepared statements.
    // See the doc for more information about the usage of cached prepared statements
    let (mut cache, transaction) = context.split();
   
    // this is (logically) a nop on a non-transactional context
    transaction.commit().await.expect("failed to commit");

    let context = if transactional {
        let transaction = client
            .transaction()
            .await
            .expect("Failed to open a new transaction");
        Cache::default().into_transaction_context(transaction)
    } else {
        Cache::default().into_client_context(client)
    };

    let message_from_db = Header::apsql_retrieve(&context, id).await.expect("Failed to load");
    assert_eq!(message, message_from_db);
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


#### TODO
Things to do at some point in time (PRs are welcome)

  - generate a proper rust module hierarchy from the modules' object-identifier
  - remove legacy rust+uper code generator (v0.3.0)
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
