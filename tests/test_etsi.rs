#![recursion_limit = "512"]

mod test_utils;

use test_utils::*;

asn_to_rust!(
    r"DENM-PDU-Descriptions {itu-t (0) identified-organization (4) etsi (0) itsDomain (5) wg1 (1) en (302637) denm (1) version (2)
}

DEFINITIONS AUTOMATIC TAGS ::=

BEGIN

ValidityDuration ::= INTEGER {
    timeOfDetection(0),
    oneSecondAfterDetection(1)
} (0..86400)


ManagementContainer ::= SEQUENCE {
	validityDuration ValidityDuration DEFAULT defaultValidity,
	...
}

defaultValidity INTEGER ::= 600

END
"
);

#[test]
fn does_it_compile() {}
