use crate::model::{Asn, BitString, Definition, Integer, Model, Range, Size, Type, ValueReference};
use std::fmt::{Debug, Display, Formatter};

pub trait ResolveState: Clone {
    type SizeType: Display + Debug + Clone + PartialOrd + PartialEq;
    type RangeType: Display + Debug + Clone + PartialOrd + PartialEq;
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct Resolved;
impl ResolveState for Resolved {
    type SizeType = usize;
    type RangeType = i64;
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct Unresolved;
impl ResolveState for Unresolved {
    type SizeType = LitOrRef<usize>;
    type RangeType = LitOrRef<i64>;
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub enum LitOrRef<T> {
    Lit(T),
    Ref(String),
}

impl<T> Default for LitOrRef<T>
where
    T: Default,
{
    fn default() -> Self {
        LitOrRef::Lit(T::default())
    }
}

impl<T> Display for LitOrRef<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            LitOrRef::Lit(v) => Display::fmt(v, f),
            LitOrRef::Ref(v) => Display::fmt(v, f),
        }
    }
}

#[derive(Debug, PartialOrd, PartialEq)]
pub enum Error {
    FailedToResolveReference(String),
    FailedToParseLiteral(String),
}

impl std::error::Error for Error {}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::FailedToResolveReference(name) => {
                write!(f, "Failed to resolve reference with name: {}", name)
            }
            Error::FailedToParseLiteral(literal) => {
                write!(f, "Failed to parse literal: {}", literal)
            }
        }
    }
}

pub trait Resolver<T> {
    fn resolve(&self, lor: &LitOrRef<T>) -> Result<T, Error>;
}

pub trait TryResolve<T, R: Sized> {
    fn try_resolve(&self, resolver: &impl Resolver<T>) -> Result<R, Error>;
}

impl Model<Asn<Unresolved>> {
    pub fn try_resolve(&self) -> Result<Model<Asn<Resolved>>, Error> {
        let mut model = Model::<Asn<Resolved>> {
            name: self.name.clone(),
            oid: self.oid.clone(),
            imports: self.imports.clone(),
            definitions: Vec::with_capacity(self.definitions.len()),
            value_references: Vec::with_capacity(self.value_references.len()),
        };

        // copy over all value references
        for vr in &self.value_references {
            model.value_references.push(ValueReference {
                name: vr.name.clone(),
                role: Asn {
                    tag: vr.role.tag,
                    r#type: vr.role.r#type.try_resolve(self)?,
                },
                value: vr.value.clone(),
            })
        }

        for Definition(name, asn) in &self.definitions {
            model.definitions.push(Definition(
                name.clone(),
                Asn {
                    tag: asn.tag,
                    r#type: asn.r#type.try_resolve(self)?,
                },
            ))
        }

        Ok(model)
    }
}

impl Type<Unresolved> {
    pub fn try_resolve<R: Resolver<i64> + Resolver<usize>>(
        &self,
        resolver: &R,
    ) -> Result<Type<Resolved>, Error> {
        Ok(match self {
            Type::Boolean => Type::Boolean,
            Type::Integer(integer) => Type::Integer(integer.try_resolve(resolver)?),
            Type::String(size, charset) => Type::String(size.try_resolve(resolver)?, *charset),
            Type::OctetString(size) => Type::OctetString(size.try_resolve(resolver)?),
            Type::BitString(string) => Type::BitString(string.try_resolve(resolver)?),
            Type::Optional(inner) => Type::Optional(Box::new(inner.try_resolve(resolver)?)),
            Type::Sequence(seq) => Type::Sequence(seq.clone()),
            Type::SequenceOf(inner, size) => Type::SequenceOf(
                Box::new(inner.try_resolve(resolver)?),
                size.try_resolve(resolver)?,
            ),
            Type::Set(set) => Type::Set(set.clone()),
            Type::SetOf(inner, size) => Type::SetOf(
                Box::new(inner.try_resolve(resolver)?),
                size.try_resolve(resolver)?,
            ),
            Type::Enumerated(e) => Type::Enumerated(e.clone()),
            Type::Choice(c) => Type::Choice(c.clone()),
            Type::TypeReference(name, tag) => Type::TypeReference(name.clone(), *tag),
        })
    }
}

impl<T: Display + Debug + Clone + Sized> TryResolve<T, Integer<T>> for Integer<LitOrRef<T>> {
    fn try_resolve(&self, resolver: &impl Resolver<T>) -> Result<Integer<T>, Error> {
        Ok(Integer {
            range: Range(
                self.range
                    .0
                    .as_ref()
                    .map(|lor| resolver.resolve(&lor))
                    .transpose()?,
                self.range
                    .1
                    .as_ref()
                    .map(|lor| resolver.resolve(&lor))
                    .transpose()?,
                self.range.2,
            ),
            constants: self.constants.clone(),
        })
    }
}

impl<T: Display + Debug + Clone + Sized> TryResolve<T, Size<T>> for Size<LitOrRef<T>> {
    fn try_resolve(&self, resolver: &impl Resolver<T>) -> Result<Size<T>, Error> {
        Ok(match self {
            Size::Any => Size::Any,
            Size::Fix(len, ext) => Size::Fix(resolver.resolve(len)?, *ext),
            Size::Range(min, max, ext) => {
                Size::Range(resolver.resolve(min)?, resolver.resolve(max)?, *ext)
            }
        })
    }
}

impl<T: Display + Debug + Clone + Sized> TryResolve<T, BitString<T>> for BitString<LitOrRef<T>> {
    fn try_resolve(&self, resolver: &impl Resolver<T>) -> Result<BitString<T>, Error> {
        Ok(BitString {
            size: self.size.try_resolve(resolver)?,
            constants: self.constants.clone(),
        })
    }
}

impl Resolver<usize> for Model<Asn<Unresolved>> {
    fn resolve(&self, lor: &LitOrRef<usize>) -> Result<usize, Error> {
        match lor {
            LitOrRef::Lit(lit) => Ok(*lit),
            LitOrRef::Ref(name) => match self
                .value_references
                .iter()
                .find(|vr| vr.name.eq(name))
                .map(|vr| vr.value.parse())
            {
                Some(Ok(value)) => Ok(value),
                Some(Err(e)) => Err(Error::FailedToParseLiteral(format!(
                    "name: {}, err: {}",
                    name, e
                ))),
                None => Err(Error::FailedToResolveReference(name.clone())),
            },
        }
    }
}

impl Resolver<i64> for Model<Asn<Unresolved>> {
    fn resolve(&self, lor: &LitOrRef<i64>) -> Result<i64, Error> {
        match lor {
            LitOrRef::Lit(lit) => Ok(*lit),
            LitOrRef::Ref(name) => match self
                .value_references
                .iter()
                .find(|vr| vr.name.eq(name))
                .map(|vr| vr.value.parse())
            {
                Some(Ok(value)) => Ok(value),
                Some(Err(e)) => Err(Error::FailedToParseLiteral(format!(
                    "name: {}, err: {}",
                    name, e
                ))),
                None => Err(Error::FailedToResolveReference(name.clone())),
            },
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn test_simple_resolve() {
        let mut unresolved = Model::<Asn<Unresolved>> {
            name: "UnresolvedModel".to_string(),
            definitions: vec![Definition(
                "IntegerWithVR".to_string(),
                Type::<Unresolved>::Integer(Integer {
                    range: Range(
                        Some(LitOrRef::Ref("my_min".to_string())),
                        Some(LitOrRef::Ref("my_max".to_string())),
                        true,
                    ),
                    constants: Vec::default(),
                })
                .untagged(),
            )],
            ..Default::default()
        };

        assert_eq!(
            Error::FailedToResolveReference("my_min".to_string()),
            unresolved.try_resolve().unwrap_err()
        );

        unresolved.value_references.push(ValueReference {
            name: "my_min".to_string(),
            role: Type::Integer(Integer::default()).untagged(),
            value: "123".to_string(),
        });

        assert_eq!(
            Error::FailedToResolveReference("my_max".to_string()),
            unresolved.try_resolve().unwrap_err()
        );

        unresolved.value_references.push(ValueReference {
            name: "my_max".to_string(),
            role: Type::Integer(Integer::default()).untagged(),
            value: "456".to_string(),
        });

        let resolved = unresolved.try_resolve().unwrap();
        assert_eq!(
            &resolved.definitions[..],
            &[Definition(
                "IntegerWithVR".to_string(),
                Type::<Resolved>::Integer(Integer {
                    range: Range(Some(123), Some(456), true),
                    constants: Vec::default(),
                })
                .untagged(),
            )]
        )
    }
}
