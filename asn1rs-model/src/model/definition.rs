#[derive(Debug, Clone, PartialOrd, PartialEq, Eq)]
pub struct Definition<T>(pub String, pub T);

impl<T> Definition<T> {
    #[cfg(test)]
    pub fn new<I: ToString>(name: I, value: T) -> Self {
        Definition(name.to_string(), value)
    }

    pub fn name(&self) -> &str {
        &self.0
    }

    pub fn value(&self) -> &T {
        &self.1
    }
}
