//! ```text
//! crate::io                       Utils, common io-root
//!      ::io::per                  Generic Packed Encoding impls and traits
//!      ::io::per::unaligned       UNALIGNED PER specialization
//!      ::io::per::aligned         ALIGNED PER specialization
//!      ::io::...                  Other ASN.1 representations (e.g xer, ber, ...)
//!
//!      ::io::async_psql           Async PSQL io-utils
//!      ::io::protobuf             Protocol Buffer io-utils
//!      ::io::psql                 Blocking PSQL io-utils
//!
//!      ::io::uper                 Deprecated UNALIGNED PER decoder/encoder
//! ```

pub mod per;
pub mod protobuf;

#[cfg(feature = "psql")]
pub mod psql;

#[cfg(feature = "async-psql")]
pub mod async_psql;

#[cfg(feature = "legacy-uper-codegen")]
#[deprecated(note = "Use per::unaligned::buffer instead")]
pub mod buffer;

#[cfg(feature = "legacy-uper-codegen")]
#[deprecated(note = "Use per::unaligned instead")]
pub mod uper;
