use crate::asn::{Asn, Type};
use crate::model::lit_or_ref::{Error, LitOrRef, Resolved, Resolver, Unresolved};
use crate::model::{Definition, LiteralValue, Model, Target, ValueReference};

#[derive(Default)]
pub struct MultiModuleResolver {
    models: Vec<Model<Asn<Unresolved>>>,
}

impl MultiModuleResolver {
    pub fn push(&mut self, model: Model<Asn<Unresolved>>) {
        self.models.push(model);
    }

    pub fn try_resolve_all(&self) -> Result<Vec<Model<Asn<Resolved>>>, Error> {
        self.models
            .iter()
            .map(|model| {
                ResolveScope {
                    model,
                    scope: &self.models,
                }
                .try_resolve()
            })
            .collect::<_>()
    }
}

pub struct ResolveScope<'a> {
    model: &'a Model<Asn<Unresolved>>,
    scope: &'a [Model<Asn<Unresolved>>],
}

impl<'a> From<&'a Model<Asn<Unresolved>>> for ResolveScope<'a> {
    fn from(model: &'a Model<Asn<Unresolved>>) -> Self {
        Self {
            model,
            scope: core::slice::from_ref(model),
        }
    }
}

impl<'a> ResolveScope<'a> {
    pub(crate) fn try_resolve(&self) -> Result<Model<Asn<Resolved>>, Error> {
        let mut result = Model::<Asn<Resolved>> {
            name: self.model.name.clone(),
            oid: self.model.oid.clone(),
            imports: self.model.imports.clone(),
            definitions: Vec::with_capacity(self.model.definitions.len()),
            value_references: Vec::with_capacity(self.model.value_references.len()),
        };

        // copy over all value references
        for vr in &self.model.value_references {
            result.value_references.push(ValueReference {
                name: vr.name.clone(),
                role: vr.role.try_resolve(self)?,
                value: vr.value.clone(),
            })
        }

        for Definition(name, asn) in &self.model.definitions {
            result
                .definitions
                .push(Definition(name.clone(), asn.try_resolve(self)?))
        }

        Ok(result)
    }

    fn model_with_imported_item(&self, item: &str) -> Option<&'a Model<Asn<Unresolved>>> {
        self.model
            .imports
            .iter()
            .find(|i| i.what.iter().any(|what| what.eq(item)))
            .and_then(|import| {
                self.scope.iter().find(|m| {
                    (m.oid.is_some() && m.oid.eq(&import.from_oid)) || m.name.eq(&import.from)
                })
            })
    }

    fn value_reference(
        &self,
        name: &str,
    ) -> Option<&'a ValueReference<<Asn<Unresolved> as Target>::ValueReferenceType>> {
        self.model
            .value_references
            .iter()
            .find(|vr| vr.name.eq(name))
            .or_else(|| {
                self.model_with_imported_item(name).and_then(|model| {
                    ResolveScope {
                        model,
                        scope: self.scope,
                    }
                    .value_reference(name)
                })
            })
    }

    fn definition(&self, name: &str) -> Option<&'a Definition<Asn<Unresolved>>> {
        self.model
            .definitions
            .iter()
            .find(|def| def.name().eq(name))
            .or_else(|| {
                self.model_with_imported_item(name).and_then(|model| {
                    ResolveScope {
                        model,
                        scope: self.scope,
                    }
                    .definition(name)
                })
            })
    }
}

impl Resolver<usize> for ResolveScope<'_> {
    fn resolve(&self, lor: &LitOrRef<usize>) -> Result<usize, Error> {
        match lor {
            LitOrRef::Lit(lit) => Ok(*lit),
            LitOrRef::Ref(name) => {
                match self.value_reference(name).map(|vr| vr.value.to_integer()) {
                    Some(Some(value)) => Ok(value as usize),
                    Some(None) => Err(Error::FailedToParseLiteral(format!("name: {}", name))),
                    None => Err(Error::FailedToResolveReference(name.clone())),
                }
            }
        }
    }
}

impl Resolver<i64> for ResolveScope<'_> {
    fn resolve(&self, lor: &LitOrRef<i64>) -> Result<i64, Error> {
        match lor {
            LitOrRef::Lit(lit) => Ok(*lit),
            LitOrRef::Ref(name) => match self.value_reference(name).map(|vr| vr.value.to_integer())
            {
                Some(Some(value)) => Ok(value),
                Some(None) => Err(Error::FailedToParseLiteral(format!("name: {}", name))),
                None => Err(Error::FailedToResolveReference(name.clone())),
            },
        }
    }
}

impl Resolver<LiteralValue> for ResolveScope<'_> {
    fn resolve(&self, lor: &LitOrRef<LiteralValue>) -> Result<LiteralValue, Error> {
        match lor {
            LitOrRef::Lit(lit) => Ok(lit.clone()),
            LitOrRef::Ref(name) => self
                .value_reference(name)
                .map(|vr| vr.value.clone())
                .ok_or_else(|| Error::FailedToResolveReference(name.clone())),
        }
    }
}

impl Resolver<Type<Unresolved>> for ResolveScope<'_> {
    fn resolve(&self, lor: &LitOrRef<Type<Unresolved>>) -> Result<Type<Unresolved>, Error> {
        match lor {
            LitOrRef::Lit(lit) => Ok(lit.clone()),
            LitOrRef::Ref(name) => self
                .definition(name)
                .map(|def| def.1.r#type.clone())
                .ok_or_else(|| Error::FailedToResolveType(name.clone())),
        }
    }
}
