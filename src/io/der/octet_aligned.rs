use crate::io::buf::OctetBuffer;
use crate::io::der::Error;
use crate::io::der::{DistinguishedRead, DistinguishedWrite};

impl DistinguishedRead for OctetBuffer {}

impl DistinguishedWrite for OctetBuffer {}
