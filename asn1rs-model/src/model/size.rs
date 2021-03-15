use std::fmt::{Debug, Display};

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub enum Size<T: Display + Debug + Clone = usize> {
    Any,
    Fix(T, bool),
    Range(T, T, bool),
}

impl<T: Display + Debug + Clone> Size<T> {
    pub fn min(&self) -> Option<&T> {
        match self {
            Size::Any => None,
            Size::Fix(min, _) => Some(min),
            Size::Range(min, _, _) => Some(min),
        }
    }

    pub fn max(&self) -> Option<&T> {
        match self {
            Size::Any => None,
            Size::Fix(max, _) => Some(max),
            Size::Range(_, max, _) => Some(max),
        }
    }

    pub fn extensible(&self) -> bool {
        match self {
            Size::Any => false,
            Size::Fix(_, extensible) => *extensible,
            Size::Range(_, _, extensible) => *extensible,
        }
    }

    pub fn to_constraint_string(&self) -> Option<String> {
        match self {
            Size::Any => None,
            Size::Fix(min, extensible) => Some(format!(
                "size({}{})",
                min,
                if *extensible { ",..." } else { "" }
            )),
            Size::Range(min, max, extensible) => Some(format!(
                "size({}..{}{})",
                min,
                max,
                if *extensible { ",..." } else { "" }
            )),
        }
    }
}
