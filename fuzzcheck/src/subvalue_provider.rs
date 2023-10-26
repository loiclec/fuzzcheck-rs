/*!
Subvalue providers are used by mutators. They help generate interesting test
cases by mixing parts of differents values together.

See the documentation of the [`Mutator`](crate::Mutator) trait for more
informatiom.
*/

use std::any::{Any, TypeId};
use std::collections::HashMap;

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
    /// Get a subvalue of the given type and under a certain maximum complexity.
    ///
    /// Returns `Some((subvalue, cplx))` or `None` if no subvalue matches the
    /// type id or maximum complexity.
    fn get_random_subvalue(&self, typeid: TypeId, max_cplx: f64) -> Option<(&dyn Any, f64)>;
    /// Get a subvalue of the given type and under a certain maximum complexity.
    ///
    /// Each `index` points to a different subvalue. The function will automatically
    /// increment `index` to the next valid value.
    ///
    /// Returns `Some((subvalue, cplx))` or `None` if no more unique subvalues
    /// match thhe type id or maximum complexity.
    fn get_subvalue(&self, typeid: TypeId, max_cplx: f64, index: &mut usize) -> Option<(&dyn Any, f64)>;
}

/// A [`SubValueProvider`](crate::SubValueProvider) that always return `None`
pub struct EmptySubValueProvider;
impl SubValueProvider for EmptySubValueProvider {
    #[coverage(off)]
    fn identifier(&self) -> SubValueProviderId {
        SubValueProviderId {
            idx: 0,
            generation: Generation(0),
        }
    }

    #[coverage(off)]
    fn get_random_subvalue(&self, _typeid: TypeId, _max_cplx: f64) -> Option<(&dyn Any, f64)> {
        None
    }

    #[coverage(off)]
    fn get_subvalue(&self, _typeid: TypeId, _max_cplx: f64, _index: &mut usize) -> Option<(&dyn Any, f64)> {
        None
    }
}

/// A [`SubValueProvider`](crate::SubValueProvider) created from the subvalues
/// of a particular test case.
pub struct CrossoverSubValueProvider<T, M>
where
    T: 'static + Clone,
    M: Mutator<T>,
{
    identifier: SubValueProviderId,
    immutable_data: Box<(T, M::Cache)>,
    whole_complexity: f64,
    subvalues: HashMap<TypeId, Vec<(*const dyn Any, f64)>>,
    rng: fastrand::Rng,
}
impl<T, M> CrossoverSubValueProvider<T, M>
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    #[coverage(off)]
    pub fn new(identifier: SubValueProviderId, value: &T, cache: &M::Cache, mutator: &M) -> Self {
        let boxed_data = Box::new((value.clone(), cache.clone()));

        let mut subvalues: HashMap<TypeId, Vec<(*const dyn Any, f64)>> = HashMap::new();

        let mut act_on_subvalue = #[coverage(off)]
        |subvalue: &dyn Any, complexity| {
            subvalues
                .entry(subvalue.type_id())
                .or_default()
                .push((subvalue as *const _, complexity));
        };

        mutator.visit_subvalues(&boxed_data.0, &boxed_data.1, &mut act_on_subvalue);
        for (_typeid, subvalues) in subvalues.iter_mut() {
            subvalues.sort_by(
                #[coverage(off)]
                |x, y| (x.1, x.0).partial_cmp(&(y.1, y.0)).unwrap_or(std::cmp::Ordering::Equal),
            );
            // Why do we dedup the subvalues? Because an `AlternationMutator` may visit the same subvalue multiple times.
            //
            // Because of the guarantees offered by the `Mutator` trait, two equal subvalues will have the same
            // complexity, no matter which mutator evaluates it. And because equal subvalues have the same type,
            // their *const dyn Any pointers will be equal (same pointer address and same vtable).
            //
            // Since we have sorted the vector, equal values will be next to each other. So it is sufficient to call
            // `dedup` to get rid of duplicates.
            //
            // It is not a big problem if `dedup` fails to get rid of all duplicates, for whatever reason. The mutations
            // will just be a bit less efficient.
            subvalues.dedup();
        }
        let whole_complexity = mutator.complexity(value, cache);
        Self {
            identifier,
            immutable_data: boxed_data,
            whole_complexity,
            subvalues,
            rng: fastrand::Rng::new(),
        }
    }
}
impl<T, M> SubValueProvider for CrossoverSubValueProvider<T, M>
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    #[coverage(off)]
    fn identifier(&self) -> SubValueProviderId {
        self.identifier
    }

    #[coverage(off)]
    fn get_random_subvalue(&self, typeid: TypeId, max_cplx: f64) -> Option<(&dyn Any, f64)> {
        let subvalues = self.subvalues.get(&typeid)?;
        assert!(!subvalues.is_empty());
        let end_index_for_complexity = subvalues
            .iter()
            .position(
                #[coverage(off)]
                |x| x.1 >= max_cplx,
            )
            .unwrap_or(subvalues.len());
        // it's none if all return false, that is, if for all x.1 < max_cplx, so we can choose any index in the vector

        // but if the first element already has a cplx > max_cplx, then we can't choose any
        if end_index_for_complexity == 0 {
            return None;
        }

        let idx = self.rng.usize(..end_index_for_complexity);
        let (subvalue, complexity) = &subvalues[idx];
        let subvalue = unsafe { subvalue.as_ref() }.unwrap();
        Some((subvalue, *complexity))
    }

    #[coverage(off)]
    fn get_subvalue(&self, typeid: TypeId, max_cplx: f64, index: &mut usize) -> Option<(&dyn Any, f64)> {
        let subvalues = self.subvalues.get(&typeid)?;
        assert!(!subvalues.is_empty());

        if TypeId::of::<T>() == typeid && *index == subvalues.len() {
            *index += 1;
            Some((&self.immutable_data.0, self.whole_complexity))
        } else {
            let (subvalue, complexity) = subvalues.get(*index)?;
            if *complexity < max_cplx {
                let subvalue = unsafe { subvalue.as_ref() }.unwrap();
                *index += 1;
                Some((subvalue, *complexity))
            } else {
                // the values are sorted by complexity!
                // therefore every next *complexity will be bigger than max_cplx, and we can give up
                None
            }
        }
    }
}
