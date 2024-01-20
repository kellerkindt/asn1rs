use asn1rs_model::asn::Tag;

pub trait Constraint {
    const TAG: Tag;
}
