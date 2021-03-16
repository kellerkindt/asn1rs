#[derive(Debug, Default, Clone, Copy, PartialOrd, PartialEq)]
pub struct Range<T>(pub T, pub T, pub bool);

impl<T> Range<T> {
    pub const fn inclusive(min: T, max: T) -> Self {
        Self(min, max, false)
    }

    pub fn with_extensible(self, extensible: bool) -> Self {
        let Range(min, max, _) = self;
        Range(min, max, extensible)
    }

    pub const fn min(&self) -> &T {
        &self.0
    }

    pub const fn max(&self) -> &T {
        &self.1
    }

    pub const fn extensible(&self) -> bool {
        self.2
    }

    pub fn wrap_opt(self) -> Range<Option<T>> {
        let Range(min, max, extensible) = self;
        Range(Some(min), Some(max), extensible)
    }
}

impl<T> Range<Option<T>> {
    pub fn none() -> Self {
        Range(None, None, false)
    }

    pub fn min_max(&self, min_fn: impl Fn() -> T, max_fn: impl Fn() -> T) -> Option<(T, T)>
    where
        T: Copy,
    {
        match (self.0, self.1) {
            (Some(min), Some(max)) => Some((min, max)),
            (Some(min), None) => Some((min, max_fn())),
            (None, Some(max)) => Some((min_fn(), max)),
            (None, None) => None,
        }
    }
}
