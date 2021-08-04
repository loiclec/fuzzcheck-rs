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

pub mod builder;
mod code_coverage_sensor;
mod data_structures;
mod fuzzer;
mod pool;
mod signals_handler;
mod world;

#[doc(inline)]
pub use builder::FuzzerBuilder;
use fuzzcheck_traits::*;

#[cfg(feature = "mutators")]
pub use fuzzcheck_mutators;

#[cfg(any(feature = "serde_json_alternative_serializer", feature = "serde_json_serializer"))]
pub use fuzzcheck_serializer;

/**
 * A unit of code coverage.
 * The upper 32 bits are the index of the code coverage counter and the
 * lower 32 bits contain its hit count.
 */
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Feature(u64);

impl Feature {
    #[no_coverage]
    fn new(index: usize, counter: u64) -> Feature {
        let index = index as u64;
        let counter = Self::score_from_counter(counter) as u64;

        Feature(index << 8 | counter)
    }

    #[no_coverage]
    fn erasing_payload(self) -> Self {
        Feature(self.0 & 0xFFFF_FFFF_FFFF_FF00)
    }

    /// “Hash” a u64 into a number between 0 and 64.
    ///
    /// So that similar numbers have the same hash, and very high
    /// numbers have a greater hash.
    ///
    /// We do this because we don't want to overwhelm the fuzzers.
    /// Imagine we have a test case that reached a code block 35_987 times.
    /// We don't want to consider a test case that reaches the same code block
    /// 35_965 times to be interesting. So instead, we group similar
    /// hit counts together.
    #[no_coverage]
    fn score_from_counter(counter: u64) -> u8 {
        if counter <= 3 {
            counter as u8
        } else if counter != core::u64::MAX {
            (64 - counter.leading_zeros() + 1) as u8
        } else {
            64
        }
    }
}

/**
 * A struct that stores the value, cache, and mutation step of an input.
 * It is used for convenience.
 */
struct FuzzedInput<T: Clone, Mut: Mutator<T>> {
    pub value: T,
    pub cache: Mut::Cache,
    pub mutation_step: Mut::MutationStep,
}

impl<T: Clone, Mut: Mutator<T>> FuzzedInput<T, Mut> {
    #[no_coverage]
    pub fn new(value: T, cache: Mut::Cache, mutation_step: Mut::MutationStep) -> Self {
        Self {
            value,
            cache,
            mutation_step,
        }
    }

    #[no_coverage]
    pub fn new_source(&self, m: &Mut) -> Self {
        let (cache, mutation_step) = m.validate_value(&self.value).unwrap();
        Self::new(self.value.clone(), cache, mutation_step)
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
