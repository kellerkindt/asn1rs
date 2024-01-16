//! ```text
//! crate::io                       Utils, common io-root
//!      ::io::per                  Generic Packed Encoding impls and traits
//!      ::io::per::unaligned       UNALIGNED PER specialization
//!      ::io::per::aligned         ALIGNED PER specialization
//!      ::io::...                  Other ASN.1 representations (e.g der, xer, ber, ...)
//! ```

pub mod per;
pub mod protobuf;