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
#![feature(doc_cfg)]
#![feature(drain_filter)]
#![feature(never_type)]
#![feature(no_coverage)]
#![feature(type_alias_impl_trait)]
#![feature(generic_associated_types)]
#![feature(variant_count)]
#![feature(arc_new_cyclic)]
#![feature(trivial_bounds)]
#![feature(stmt_expr_attributes)]
#![allow(clippy::nonstandard_macro_braces)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::partialeq_ne_impl)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::collapsible_else_if)]
#![allow(clippy::len_without_is_empty)]
#![allow(clippy::nonminimal_bool)]
#![allow(clippy::comparison_chain)]
#![allow(clippy::needless_lifetimes)]
#![allow(clippy::manual_map)]
#![allow(clippy::bool_comparison)]

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

pub use fuzzcheck_common::arg::Arguments;

pub(crate) use split_string::split_string_by_whitespace;

#[doc(inline)]
pub use crate::fuzzer::FuzzingResult;
#[doc(inline)]
pub use crate::traits::CompatibleWithObservations;
#[doc(inline)]
pub use crate::traits::CorpusDelta;
#[doc(inline)]
pub use crate::traits::Pool;
#[doc(inline)]
pub use crate::traits::PoolExt;
#[doc(inline)]
pub use crate::traits::SaveToStatsFolder;
#[doc(inline)]
pub use crate::traits::Sensor;
#[doc(inline)]
pub use crate::traits::SensorAndPool;
#[doc(inline)]
pub use crate::traits::SensorExt;
#[doc(inline)]
pub use crate::traits::Stats;

#[doc(inline)]
pub use crate::fuzzer::PoolStorageIndex;

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

#[cfg(feature = "serde_json_serializer")]
#[doc(inline)]
pub use serializers::SerdeSerializer;

/// Implement a mutator for the type and make it the type’s `DefaultMutator`.
///
/// The mutator will be called `<Name>Mutator`. It can be constructed in two ways:
/// 1. Through the `DefaultMutator` trait, for example:
/// ```
/// # #![feature(no_coverage)]
/// use fuzzcheck::DefaultMutator;
///
/// #[derive(Clone, DefaultMutator)]
/// struct X<A> {
///     field: A,
/// }
/// let mutator = <X<u8> as DefaultMutator>::default_mutator();
/// // but it can also be inferred by the rust compiler:
/// let mutator = X::<u8>::default_mutator();
/// ```
/// 2. By using `<Name>Mutator::new(..)` with the submutators for every field given as argument, for example:
/// ```
/// # #![feature(no_coverage)]
/// use fuzzcheck::DefaultMutator;
///
/// #[derive(Clone, DefaultMutator)]
/// enum Either<A, B> {
///     Left(A),
///     Right(B)
/// }
/// let mutator = EitherMutator::new(u8::default_mutator(), bool::default_mutator());
/// // mutator impl Mutator<Either<u8, bool>>
/// ```
pub use fuzzcheck_mutators_derive::DefaultMutator;

/**
    Make a mutator for a custom type, optionally making it the type’s default mutator.

    The syntax is as follows:
    ```
    # #![feature(no_coverage)]
    # #![feature(trivial_bounds)]
    use fuzzcheck_mutators_derive::make_mutator;

    use fuzzcheck::mutators::integer_within_range::U8WithinRangeMutator;

    // somewhere, this type is defined
    #[derive(Clone)]
    pub struct S<T> {
        x: u8,
        y: T
    }
    // create a mutator for this type:
    make_mutator! {
        name: SMutator // the name of the mutator
        recursive: false, // the type is not recursive
        default: false, // if `true`, impl DefaultMutator<Mutator = SMutator> for S
        type:  // repeat the declaration of S
            pub struct S<T> {
            // left hand side: the type of the mutator for the field
            // right hand side (optional): the default value of that mutator
            #[field_mutator(U8WithinRangeMutator = { U8WithinRangeMutator::new(0 ..= 10) })]
            x: u8,
            y: T
        }
    }
    ```
    For enums:
    ```
    # #![feature(no_coverage)]
    use fuzzcheck::make_mutator;

    use fuzzcheck::mutators::integer::U8Mutator;

    // somewhere, this type is defined
    #[derive(Clone)]
    pub enum E<T> {
        One,
        Two(T, u8),
        Three { x: Option<u8> }
    }
    // create a mutator for this type:
    make_mutator! {
        name: EMutator // the name of the mutator
        recursive: false, // the type is not recursive
        default: true, // this is E's default mutator
        type: // repeat the declaration of E
            pub enum E<T> {
                One,
                Two(T, #[field_mutator(U8Mutator)] u8),
                Three { x: Option<u8> }
            }
    }
    ```
    Create a recursive mutator:
    ```
    # #![feature(no_coverage)]
    use fuzzcheck::make_mutator;
    use fuzzcheck::mutators::{option::OptionMutator, boxed::BoxMutator};
    use fuzzcheck::mutators::recursive::RecurToMutator;

    #[derive(Clone)]
    pub struct R<T> {
        x: u8,
        y: Option<Box<R<T>>>,
        z: Vec<T>,
    }
    make_mutator! {
        name: RMutator
        recursive: true,
        default: true,
        type: // repeat the declaration of R
            pub struct R<T> {
                x: u8,
                // for recursive mutators, it is necessary to indicate *where* the recursion is
                // and use a `RecurToMutator` as the recursive field's mutator
                // M0 is the type parameter for the mutator of the `x` field, M2 is the type parameter for the mutator of the `z` field
                #[field_mutator(OptionMutator<Box<R<T>>, BoxMutator<RecurToMutator<RMutator<T, M0, M2>>>> = { OptionMutator::new(BoxMutator::new(self_.into())) })]
                //                                                                                            self_.into() creates the RecurToMutator
                y: Option<Box<R<T>>>,
                z: Vec<T>
            }
    }
    ```
*/
pub use fuzzcheck_mutators_derive::make_mutator;

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
            generation: self.generation,
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
        let cache = m.validate_value(&self.value).unwrap();
        let mutation_step = m.default_mutation_step(&self.value, &cache);
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
