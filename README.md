# asn1rs - ASN.1 Compiler for Rust

This crate generates Rust Code and optionally compatible Protobuf and SQL schema files from ASN.1 definitions.
Basic support for [serde](https://crates.io/crates/serde) integration is provided.
The crate can be used as standalone CLI binary or used as library through its API
(for example inside the ```build.rs``` script).


[![Build Status](https://github.com/kellerkindt/asn1rs/workflows/Rust/badge.svg)](https://github.com/kellerkindt/asn1rs/actions?query=workflow%3ARust)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](https://github.com/kellerkindt/asn1rs)
[![Crates.io](https://img.shields.io/crates/v/asn1rs.svg)](https://crates.io/crates/asn1rs)
[![Documentation](https://docs.rs/asn1rs/badge.svg)](https://docs.rs/asn1rs)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](https://github.com/kellerkindt/asn1rs/issues/new)



#### Support Table

| Feature             | Parses  | UPER    | Protobuf    | PSQL        | Async PSQL | UPER Legacy       |
| --------------------|:--------|:--------|:------------|:------------|:-----------|------------------:|
| `SEQUENCE`          | ✅ yes  | ✅ yes  | ✅ yes      | ✅ yes      | ✅ yes     | ✅ yes             | 
| ...extensible       | ✅ yes  | ✅ yes  | 🆗 ignored  | 🆗 ignored  | 🆗 ignored | 🔶 not serialized  | 
| `SEQUENCE OF`       | ✅ yes  | ✅ yes  | ✅ yes      | ✅ yes      | ✅ yes     | ✅ yes             | 
| ...`SIZE(A..B)`     | ✅ yes  | ✅ yes  | 🆗 ignored  | 🆗 ignored  | 🆗 ignored | ⚠️ ignored️         | 
| ...`SIZE(A..B,...)` | ✅ yes  | ✅ yes  | 🆗 ignored  | 🆗 ignored  | 🆗 ignored | ⚠️ ignored️         | 
| `ENUMERATED`        | ✅ yes  | ✅ yes  | ✅ yes      | ✅ yes      | ✅ yes     | ✅ yes             |           
| ...extensible       | ✅ yes  | ✅ yes  | 🆗 ignored  | 🆗 ignored  | 🆗 ignored | 🔶 not serialized  |           
| `CHOICE`            | ✅ yes  | ✅ yes  | ✅ yes      | ✅ yes      | ✅ yes     | ✅ yes             |           
| ...extensible       | ✅ yes  | ✅ yes  | 🆗 ignored  | 🆗 ignored  | 🆗 ignored | 🔶 not serialized  | 
| `BIT STRING`        | ✅ yes  | ✅ yes  | ✅ yes(1)   | ✅ yes(1)   | ✅ yes(1)  | ✅ yes             | 
| ...`SIZE(A..B)`     | ✅ yes  | ✅ yes  | 🆗 ignored  | 🆗 ignored  | 🆗 ignored | ⚠️ ignored         | 
| ...`SIZE(A..B,...)` | ✅ yes  | ✅ yes  | 🆗 ignored  | 🆗 ignored  | 🆗 ignored | ⚠️ ignored         | 
| `OCTET STRING`      | ✅ yes  | ✅ yes  | ✅ yes      | ✅ yes      | ✅ yes     | ✅ yes             | 
| ...`SIZE(A..B)`     | ✅ yes  | ✅ yes  | 🆗 ignored  | 🆗 ignored  | 🆗 ignored | ⚠️ ignored         | 
| ...`SIZE(A..B,...)` | ✅ yes  | ✅ yes  | 🆗 ignored  | 🆗 ignored  | 🆗 ignored | ⚠️ ignored         |   
| `UTF8String`        | ✅ yes  | ✅ yes  | ✅ yes      | ✅ yes      | ✅ yes     | ✅ yes             | 
| ...`SIZE(A..B)`     | ✅ yes  | ✅ yes  | 🆗 ignored  | 🆗 ignored  | 🆗 ignored | ⚠️ ignored         | 
| ...`SIZE(A..B,...)` | ✅ yes  | ✅ yes  | 🆗 ignored  | 🆗 ignored  | 🆗 ignored | ⚠️ ignored         | 
| `IA5String`         | ✅ yes  | ✅ yes  | ✅ yes(1)   | ✅ yes(1)   | ✅ yes(1)  | ❌ ub              | 
| ...`SIZE(A..B)`     | ✅ yes  | ✅ yes  | 🆗 ignored  | 🆗 ignored  | 🆗 ignored | ❌ ub              | 
| ...`SIZE(A..B,...)` | ✅ yes  | ✅ yes  | 🆗 ignored  | 🆗 ignored  | 🆗 ignored | ❌ ub              |   
| `INTEGER`           | ✅ yes  | ✅ yes  | ✅ yes      | ✅ yes      | ✅ yes     | ✅ yes             |
| ...`A..B`           | ✅ yes  | ✅ yes  | ✅ yes(2)   | ✅ yes(2)   | ✅ yes(2)  | ✅ yes             |
| ...`A..B,...`       | ✅ yes  | ✅ yes  | ✅ yes(2)   | ✅ yes(2)   | ✅ yes(2)  | ⚠️ ignored         |
| `BOOLEAN`           | ✅ yes  | ✅ yes  | ✅ yes      | ✅ yes      | ✅ yes     | ✅ yes             |
| `OPTIONAL`          | ✅ yes  | ✅ yes  | ✅ yes      | ✅ yes      | ✅ yes     | ✅ yes             |
| `IMPORTS..FROM..;`  | ✅ yes  |         |             |             |            |                    |
| `ObjectIdentifiers` | ✅ yes  |         |             |             |            |                    |


 - ✅ yes: according to specification
 - ✅ yes(1): different representation
 - ✅ yes(2): as close as possible to the original specification (sometimes yes, sometimes yes(1))
 - 🔶 not serialized: values are not serialized or deserialized in this case, might break compatibility
 - ⚠️ ignored️: constraint is ignored, this most likely breaks compatibility
 - 🆗 ignored: constraint is ignored but it does not break compatibility
 - ❌ ub: undefined behavior - whatever seems reasonable to prevent compiler errors and somehow transmit the value
 - 🟥 error: fails to compile / translate


##### Supported standards
 - ```itu-t(0) identified-organization(4) etsi(0) itsDomain(5) wg1(1) ts(102894) cdd(2) version(1)``` (ITS-Container)
 - ```itu-t(0) identified-organization(4) etsi(0) itsDomain(5) wg1(1) en(302637) cam(2) version(1)``` (CAM-PDU-Descriptions)

#### CLI usage

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

#### API usage

The following example generates Rust, Protobuf and SQL files for all ```.asn1```-files in the ```asn/``` directory of the project.
While the generated Rust code is written to the ```src/``` directory, the Protobuf files are written to ```proto/``` and the SQL files are written to ```sql/ ```.
Additionally, in this example each generated Rust-Type also receives ```Serialize``` and ```Deserialize``` derive directives (```#[derive(Serialize, Deserialize)]```) for automatic [serde](https://crates.io/crates/serde) integration.

File ```build.rs```:

```rust
extern crate asn1rs;

use std::fs;

use asn1rs::converter::convert_to_proto;
use asn1rs::converter::convert_to_rust;
use asn1rs::converter::convert_to_sql;
use asn1rs::gen::rust::RustCodeGenerator;

pub fn main() {
    for entry in fs::read_dir("asn").unwrap().into_iter() {
        let entry = entry.unwrap();
        let file_name = entry.file_name().into_string().unwrap();
        if file_name.ends_with(".asn1") {
            if let Err(e) = convert_to_rust(
                entry.path().to_str().unwrap(),
                "src/",
                |generator: &mut RustCodeGenerator| {
                    generator.add_global_derive("Serialize");
                    generator.add_global_derive("Deserialize");
                },
            ) {
                panic!("Conversion to rust failed for {}: {:?}", file_name, e);
            }
            if let Err(e) = convert_to_proto(entry.path().to_str().unwrap(), "proto/") {
                panic!("Conversion to proto failed for {}: {:?}", file_name, e);
            }
            if let Err(e) = convert_to_sql(entry.path().to_str().unwrap(), "sql/") {
                panic!("Conversion to sql failed for {}: {:?}", file_name, e);
            }
        }
    }
}
```

#### Inlining ASN.1 with procedural macros

Useful for tests or very small definitions. See ```tests/``` for more examples.
```rust
use asn1rs::io::buffer::BitBuffer;
use asn1rs::macros::asn_to_rust;

asn_to_rust!(
    r"BasicInteger DEFINITIONS AUTOMATIC TAGS ::=
    BEGIN
    
    RangedMax ::= Integer (0..MAX)
    
    NotRanged ::= Integer
    
    END"
);

#[test]
fn test_default_range() {
    assert_eq!(RangedMax::value_min(), NotRanged::value_min());
    assert_eq!(RangedMax::value_max(), NotRanged::value_max());
    let _ = NotRanged(123_u64); // does not compile if the inner type is not u64
}
```


#### Example ASN.1-Definition to Rust, Protobuf and SQL

Input ```input.asn1```

```asn
MyMessages DEFINITIONS AUTOMATIC TAGS ::=
BEGIN

Header ::= SEQUENCE {
    timestamp    INTEGER (0..1209600000)
}

END
```

Output ```my_messages.rs```:

```rust
use asn1rs::prelude::*;

#[asn(sequence)]
#[derive(Default, Debug, Clone, PartialEq, Hash)]
pub struct Header {
    #[asn(integer(0..1209600000))] pub timestamp: u32,
}

impl Header {
    pub fn timestamp_min() -> u32 {
        0
    }

    pub fn timestamp_max() -> u32 {
        1_209_600_000
    }

    // Insert and query functions for Async PostgreSQL
    pub async fn apsql_retrieve_many(context: &apsql::Context<'_>, ids: &[i32]) -> Result<Vec<Self>, apsql::Error> { /*..*/ }
    pub async fn apsql_retrieve(context: &apsql::Context<'_>, id: i32) -> Result<Self, apsql::Error> { /*..*/ }
    pub async fn apsql_load(context: &apsql::Context<'_>, row: &apsql::Row) -> Result<Self, apsql::Error> { /*..*/ }
    pub async fn apsql_insert(&self, context: &apsql::Context<'_>) -> Result<i32, apsql::PsqlError> { /*..*/ }
}

// Serialize and deserialize functions for ASN.1 UPER
impl Uper for Header { /*..*/ }

// Serialize and deserialize functions for protobuf
impl ProtobufEq for Header { /*..*/ }
impl Protobuf for Header { /*..*/ }

// Insert and query functions for PostgreSQL
impl PsqlRepresentable for Header { /*..*/ }
impl PsqlInsertable for Header { /*..*/ }
impl PsqlQueryable for Header { /*..*/ }
```

Output ```my_messages.proto```:

```proto
syntax = 'proto3';
package my.messages;

message Header {
    uint32 timestamp = 1;
}
```

Output ```my_messages.sql```:

```sql
DROP TABLE IF EXISTS Header CASCADE;

CREATE TABLE Header (
    id SERIAL PRIMARY KEY,
    timestamp INTEGER NOT NULL
);
```

#### Example usage of async postgres
NOTE: This requires the `async-psql` feature.

Using async postgres allows the message - or the batched messages - to take advantage of [`pipelining`] automatically.
This can provide a speedup (personal experience: at around 26%) compared to the synchronous/blocking postgres implementation.

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

#### Good to know
The module ```asn1rs::io``` exposes (de-)serializers and helpers for direct usage without ASN.1 definitons:
```rust
use asn1rs::prelude::*;
use asn1rs::io::per::unaligned::buffer::BitBuffer;

let mut buffer = BitBuffer::default();
buffer.write_bit(true).unwrap();
buffer.write_utf8_string("My UTF8 Text").unwrap();

send_to_another_host(buffer.into::<Vec<u8>>()):
```
```rust
use asn1rs::io::protobuf::*;

let mut buffer = Vec::default();
buffer.write_varint(1337).unwrap();
buffer.write_string("Still UTF8 Text").unwrap();

send_to_another_host(buffer):
``` 


#### TODO
Things to do at some point in time (PRs are welcome)

  - generate a proper module hierarchy from the modules' object-identifier
  - remove legacy rust+uper code generator (probably in 0.3)
  - support ```#![no_std]```
  - refactor / clean-up (rust) code-generators


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
