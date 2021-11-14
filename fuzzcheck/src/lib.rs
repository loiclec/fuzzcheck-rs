//! Fuzzcheck is an evolutionary fuzzing engine for Rust functions.
//!
//! It is recommended to use it with the command line tool `cargo-fuzzcheck`, which
//! makes it easy to compile your crate with code coverage instrumentation and
//! to manage fuzz targets.
//!
//! The best way to get started is to follow [the guide at fuzzcheck.neocities.org](https://fuzzcheck.neocities.org).
//!
//! The crate documentation contains information on how to set up and launch a fuzz-test ([here](crate::builder)) but
//! also documents the core traits ([`Pool`], [`Sensor`], [`Mutator`], etc.) that are useful to understand how it works
//! and to extend it.

#![feature(drain_filter)]
#![feature(never_type)]
#![feature(no_coverage)]
#![feature(type_alias_impl_trait)]
#![feature(generic_associated_types)]
#![feature(variant_count)]
#![feature(arc_new_cyclic)]
#![allow(clippy::nonstandard_macro_braces)]
#![allow(clippy::too_many_arguments)]

#[doc(hidden)]
pub extern crate fastrand;

mod bitset;
pub mod builder;
mod code_coverage_sensor;
mod data_structures;
mod fenwick_tree;
mod fuzzer;

pub mod mutators;
pub mod sensors_and_pools;
pub mod serializers;
mod signals_handler;
mod split_string;
mod traits;
mod world;

pub(crate) use split_string::split_string_by_whitespace;

#[doc(inline)]
pub use crate::traits::CompatibleWithSensor;
#[doc(inline)]
pub use crate::traits::CorpusDelta;
#[doc(inline)]
pub use crate::traits::Pool;
#[doc(inline)]
pub use crate::traits::Sensor;

#[doc(inline)]
pub use crate::fuzzer::PoolStorageIndex;

#[doc(inline)]
pub use builder::default_sensor_and_pool;
#[doc(inline)]
pub use fuzzer::ReasonForStopping;
#[doc(inline)]
pub use mutators::DefaultMutator;
#[doc(inline)]
pub use traits::Mutator;
#[doc(inline)]
pub use traits::MutatorWrapper;
#[doc(inline)]
pub use traits::Serializer;
#[doc(inline)]
pub use traits::{CSVField, ToCSV};

#[doc(inline)]
pub use builder::fuzz_test;

#[doc(inline)]
pub use serializers::ByteSerializer;
#[doc(inline)]
pub use serializers::StringSerializer;

#[doc(inline)]
pub use serializers::SerdeSerializer;

/**
 * A struct that stores the value, cache, and mutation step of an input.
 * It is used for convenience.
 */
pub(crate) struct FuzzedInput<T: Clone, Mut: Mutator<T>> {
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
