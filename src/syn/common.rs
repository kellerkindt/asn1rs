use crate::model::Tag;

pub trait Constraint {
    const TAG: Option<Tag> = None;
}
