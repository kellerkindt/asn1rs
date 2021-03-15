#[derive(Debug, Clone, Copy, PartialOrd, PartialEq)]
pub enum Size {
    Any,
    Fix(usize, bool),
    Range(usize, usize, bool),
}

impl Size {
    pub fn min(&self) -> Option<usize> {
        match self {
            Size::Any => None,
            Size::Fix(min, _) => Some(*min),
            Size::Range(min, _, _) => Some(*min),
        }
    }

    pub fn max(&self) -> Option<usize> {
        match self {
            Size::Any => None,
            Size::Fix(max, _) => Some(*max),
            Size::Range(_, max, _) => Some(*max),
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
        if Size::Any != *self {
            Some(format!(
                "{}..{}{}",
                self.min().unwrap_or_default(),
                self.max().unwrap_or_else(|| i64::max_value() as usize),
                if self.extensible() { ",..." } else { "" }
            ))
        } else {
            None
        }
    }
}
