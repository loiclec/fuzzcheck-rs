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

pub struct CrossoverSubValueProvider<T, M>
where
    T: 'static + Clone,
    M: Mutator<T>,
{
    identifier: SubValueProviderId,
    immutable_data: Box<(T, M::Cache)>,
    subvalues: HashMap<TypeId, Vec<(*const dyn Any, f64)>>,
}
impl<T, M> CrossoverSubValueProvider<T, M>
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    pub fn new(identifier: SubValueProviderId, value: &T, cache: &M::Cache, mutator: &M) -> Self {
        let boxed_data = Box::new((value.clone(), cache.clone()));

        let mut subvalues: HashMap<TypeId, Vec<(*const dyn Any, f64)>> = HashMap::new();
        mutator.visit_subvalues(
            &boxed_data.0,
            &boxed_data.1,
            #[no_coverage]
            &mut |subvalue, complexity| {
                subvalues
                    .entry(subvalue.type_id())
                    .or_default()
                    .push((subvalue as *const _, complexity));
            },
        );

        Self {
            identifier,
            immutable_data: boxed_data,
            subvalues,
        }
    }
}
impl<T, M> SubValueProvider for CrossoverSubValueProvider<T, M>
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    fn identifier(&self) -> SubValueProviderId {
        self.identifier
    }

    fn get_subvalue(&self, typeid: TypeId, max_cplx: f64, index: &mut usize) -> Option<&dyn Any> {
        let subvalues = self.subvalues.get(&typeid)?;
        assert!(!subvalues.is_empty());
        loop {
            if TypeId::of::<T>() == typeid && *index == subvalues.len() {
                *index += 1;
                return Some(&self.immutable_data.0);
            } else {
                let (subvalue, complexity) = subvalues.get(*index)?;
                if *complexity < max_cplx {
                    let subvalue = unsafe { subvalue.as_ref() }.unwrap();
                    *index += 1;
                    return Some(subvalue);
                } else {
                    *index += 1;
                }
            }
        }
    }
}
