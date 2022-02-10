use std::{
    any::{Any, TypeId},
    collections::HashMap,
};

use crate::Mutator;

/// Uniquely identifies a [`SubValueProvider`](crate::SubValueProvider)
///
/// The identifier is composed of two fields: `idx` and `generation`. At any
/// point in time, only one subvalue provider should have a given `idx`.
///
/// If two subvalue providers have the same `idx` but different `generation`,
/// then only the one with the larger generation is valid.
#[derive(Clone, Copy)]
pub struct SubValueProviderId {
    pub idx: usize,
    pub generation: Generation,
}

/// See: [`SubValueProviderId`](crate::SubValueProviderId)
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Generation(pub usize);

/// An object-safe trait which can give values of arbitrary types.
///
/// See the documentation of the [`Mutator`](crate::Mutator) trait for more information
/// about its purpose.
pub trait SubValueProvider {
    /// A globally unique identifier for the subvalue provider
    fn identifier(&self) -> SubValueProviderId;
    fn get_subvalue(&self, typeid: TypeId, max_cplx: f64, index: &mut usize) -> Option<&dyn Any>;
}

/// A [`SubValueProvider`](crate::SubValueProvider) that always return `None`
pub struct EmptySubValueProvider;
impl SubValueProvider for EmptySubValueProvider {
    #[no_coverage]
    fn identifier(&self) -> SubValueProviderId {
        SubValueProviderId {
            idx: 0,
            generation: Generation(0),
        }
    }
    #[no_coverage]
    fn get_subvalue(&self, _typeid: TypeId, _max_cplx: f64, _index: &mut usize) -> Option<&dyn Any> {
        None
    }
}

pub struct LensPathAndComplexity<LP> {
    pub lens_path: LP,
    pub complexity: f64,
}

/// A type which implements [`SubValueProvider`]
/// from a fuzzed test case and its mutator.
///
/// Its [`self.get_subvalue(..)`](SubValueProvider::get_subvalue) returns
/// parts of the test case, found by the [`mutator.all_paths(..)`](Mutator::all_paths)
/// [`mutator.lens(..)`](Mutator::lens) methods.
pub struct CrossoverSubValueProvider<'a, M, T>
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    identifier: SubValueProviderId,
    mutator: &'a M,
    value: &'a T,
    cache: &'a M::Cache,
    all_paths: &'a HashMap<TypeId, Vec<LensPathAndComplexity<M::LensPath>>>,
}
impl<'a, M, Value> CrossoverSubValueProvider<'a, M, Value>
where
    Value: Clone + 'static,
    M: Mutator<Value>,
{
    #[no_coverage]
    pub fn from(
        mutator: &'a M,
        value: &'a Value,
        cache: &'a M::Cache,
        all_paths: &'a HashMap<TypeId, Vec<LensPathAndComplexity<M::LensPath>>>,
        identifier: SubValueProviderId,
    ) -> Self {
        Self {
            identifier,
            mutator,
            value,
            cache,
            all_paths,
        }
    }
}
impl<'a, M, Value> SubValueProvider for CrossoverSubValueProvider<'a, M, Value>
where
    Value: Clone + 'static,
    M: Mutator<Value>,
{
    #[no_coverage]
    fn identifier(&self) -> SubValueProviderId {
        self.identifier
    }

    #[no_coverage]
    fn get_subvalue(&self, typeid: TypeId, max_cplx: f64, index: &mut usize) -> Option<&dyn Any> {
        let all_paths = self.all_paths.get(&typeid)?;
        assert!(!all_paths.is_empty());
        loop {
            if self.value.type_id() == typeid && *index == all_paths.len() {
                *index += 1;
                return Some(self.value);
            } else {
                let LensPathAndComplexity { lens_path, complexity } = all_paths.get(*index)?;
                if *complexity < max_cplx {
                    let subvalue = self.mutator.lens(self.value, self.cache, lens_path);
                    *index += 1;
                    return Some(subvalue);
                } else {
                    *index += 1;
                }
            }
        }
    }
}
