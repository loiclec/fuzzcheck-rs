//! Fuzzcheck is a coverage-guided, evolutionary fuzzing engine for Rust
//! functions.

#![feature(drain_filter)]
#![feature(never_type)]
#![feature(is_sorted)]
#![feature(thread_local)]
#![feature(maybe_uninit_slice)]
#![feature(test)]
#![feature(no_coverage)]
#![feature(type_alias_impl_trait)]
#![feature(generic_associated_types)]
#![feature(variant_count)]
#![feature(arc_new_cyclic)]
#![feature(trivial_bounds)]
#![allow(clippy::nonstandard_macro_braces)]
#![allow(clippy::too_many_arguments)]

pub extern crate fastrand;

mod and_sensor_and_pool;
pub mod builder;
mod code_coverage_sensor;
mod coverage_sensor_and_pool;
mod data_structures;
mod fenwick_tree;
mod fuzzer;
mod input_minify_pool;
mod maximize_pool;
pub mod mutators;
mod noop_sensor;
mod sensor_and_pool;
pub mod serializers;
mod signals_handler;
mod traits;
mod unique_coverage_pool;
mod unit_pool;
mod world;

#[doc(inline)]
pub use mutators::DefaultMutator;
use sensor_and_pool::TestCase;
#[doc(inline)]
pub use traits::Mutator;
#[doc(inline)]
pub use traits::MutatorWrapper;
#[doc(inline)]
pub use traits::Serializer;

#[doc(inline)]
pub use builder::FuzzerBuilder;

#[doc(inline)]
pub use serializers::ByteSerializer;
#[doc(inline)]
pub use serializers::StringSerializer;

#[cfg(feature = "serde_json_alternative_serializer")]
#[doc(inline)]
pub use serializers::JsonSerializer;

#[cfg(feature = "serde_json_serializer")]
#[doc(inline)]
pub use serializers::SerdeSerializer;

/**
 * A struct that stores the value, cache, and mutation step of an input.
 * It is used for convenience.
 */
pub struct FuzzedInput<T: Clone, Mut: Mutator<T>> {
    pub value: T,
    pub cache: Mut::Cache,
    pub mutation_step: Mut::MutationStep,
    pub generation: usize,
}
impl<T: Clone, Mut: Mutator<T>> Clone for FuzzedInput<T, Mut> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            cache: self.cache.clone(),
            mutation_step: self.mutation_step.clone(),
            generation: self.generation.clone(),
        }
    }
}
impl<T: Clone, Mut: Mutator<T>> TestCase for FuzzedInput<T, Mut> {
    #[no_coverage]
    fn generation(&self) -> usize {
        self.generation
    }
}
impl<T: Clone, Mut: Mutator<T>> FuzzedInput<T, Mut> {
    #[no_coverage]
    pub fn new(value: T, cache: Mut::Cache, mutation_step: Mut::MutationStep, generation: usize) -> Self {
        Self {
            value,
            cache,
            mutation_step,
            generation,
        }
    }

    #[no_coverage]
    pub fn new_source(&self, m: &Mut) -> Self {
        let (cache, mutation_step) = m.validate_value(&self.value).unwrap();
        Self::new(self.value.clone(), cache, mutation_step, self.generation + 1)
    }

    #[no_coverage]
    pub fn complexity(&self, m: &Mut) -> f64 {
        m.complexity(&self.value, &self.cache)
    }

    #[no_coverage]
    pub fn mutate(&mut self, m: &mut Mut, max_cplx: f64) -> Option<(Mut::UnmutateToken, f64)> {
        m.ordered_mutate(&mut self.value, &mut self.cache, &mut self.mutation_step, max_cplx)
    }

    #[no_coverage]
    pub fn unmutate(&mut self, m: &Mut, t: Mut::UnmutateToken) {
        m.unmutate(&mut self.value, &mut self.cache, t);
    }
}
