//!
//! crate::io                       Utils, common io-root
//!      ::io::per                  Generic Packed Encoding impls
//!      ::io::per::unaligned       UNALIGNED PER specialization
//!
//!      ::io::async_psql           Async PSQL io-utils
//!      ::io::protobuf             Protocol Buffer io-utils
//!      ::io::psql                 Blocking PSQL io-utils
//!
//!      ::io::uper                 Deprecated UNALIGNED PER decoder/encoder
//!

pub mod buffer;
pub mod per;
pub mod protobuf;
pub mod uper;

#[cfg(feature = "psql")]
pub mod psql;

#[cfg(feature = "async-psql")]
pub mod async_psql;
