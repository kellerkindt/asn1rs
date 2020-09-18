pub mod octet_aligned;

// in the long term, this Error type should be moved - maybe to crate::io::err::Error ?
pub use super::per::Error;

/// According to ITU-TX.690 | ISO/IEC 8825-1:2015
pub trait DistinguishedRead {}

/// According to ITU-TX.690 | ISO/IEC 8825-1:2015
pub trait DistinguishedWrite {}
