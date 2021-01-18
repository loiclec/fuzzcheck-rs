//! Fuzzcheck is a coverage-guided, evolutionary fuzzing engine for Rust
//! functions.

#![feature(drain_filter)]
#![feature(never_type)]
#![feature(is_sorted)]
#![feature(link_llvm_intrinsics)]
#![feature(thread_local)]

mod nix_subset;

mod code_coverage_sensor;
mod data_structures;

mod fuzzer;
mod world;

mod pool;
mod signals_handler;

use fuzzcheck_common::arg::{
    options_parser, FullCommandLineArguments, COMMAND_FUZZ, COMMAND_MINIFY_CORPUS, COMMAND_MINIFY_INPUT,
    CORPUS_SIZE_FLAG, INPUT_FILE_FLAG, IN_CORPUS_FLAG,
};

extern crate fuzzcheck_traits;
use fuzzcheck_traits::*;

use std::borrow::Borrow;

/** Fuzz-test the given test function, following to the command-line arguments
provided by the cargo-fuzzcheck tool.

* The first argument is a function `fn(T) -> bool` to fuzz-test.
**It is only allowed to use the main thread**. If it tries to perform asynchronous
operations, the fuzzing engine will be confused and act in unpredictable ways.

* The second argument is a mutator for values of type `T`.
See the [Mutator] trait for more information. Some basic mutators are provided
by the fuzzcheck_mutators crate.

* The third argument is a serializer for values of type `T`.
See the [Serializer] trait for more information. Some basic serializers are
provided by the fuzzcheck_serializer crate.

This function will either:

* never return
    * the fuzz-test does not find any crash and continuously keeps
running
    * if a test failure or crash is detected
    * if the command line arguments could not be parsed. It will then print the
    help section of cargo-fuzzcheck and exit.

* return `Ok(())` if the maximum number of iterations has been reached

* return an error if some necessary IO operation could not be performed. You
do not need to catch or handle the error.
*/
pub fn launch<T, FT, F, M, S>(test: F, mutator: M, serializer: S) -> Result<(), std::io::Error>
where
    FT: ?Sized,
    T: Clone + Borrow<FT>,
    F: Fn(&FT) -> bool,
    M: Mutator<T>,
    S: Serializer<Value = T>,
    fuzzer::Fuzzer<T, FT, F, M, S>: 'static,
{
    let env_args: Vec<_> = std::env::args().collect();
    let parser = options_parser();
    let mut help = format!(
        r#""
fuzzcheck <SUBCOMMAND> [OPTIONS]

SUBCOMMANDS:
    {fuzz}    Run the fuzz test
    {tmin}    Minify a crashing test input, requires --{input_file}
    {cmin}    Minify a corpus of test inputs, requires --{in_corpus}
"#,
        fuzz = COMMAND_FUZZ,
        tmin = COMMAND_MINIFY_INPUT,
        input_file = INPUT_FILE_FLAG,
        cmin = COMMAND_MINIFY_CORPUS,
        in_corpus = IN_CORPUS_FLAG,
    );
    help += parser.usage("").as_str();
    help += format!(
        r#""
## Examples:

fuzzcheck {fuzz}
    Launch the fuzzer with default options.

fuzzcheck {tmin} --{input_file} "artifacts/crash.json"

    Minify the test input defined in the file "artifacts/crash.json".
    It will put minified inputs in the folder artifacts/crash.minified/
    and name them {{complexity}}-{{hash}}.json. 
    For example, artifacts/crash.minified/4213--8cd7777109b57b8c.json
    is a minified input of complexity 42.13.

fuzzcheck {cmin} --{in_corpus} "fuzz-corpus" --{corpus_size} 25

    Minify the corpus defined by the folder "fuzz-corpus", which should
    contain JSON-encoded test inputs.
    It will remove files from that folder until only the 25 most important
    test inputs remain.
"#,
        fuzz = COMMAND_FUZZ,
        tmin = COMMAND_MINIFY_INPUT,
        input_file = INPUT_FILE_FLAG,
        cmin = COMMAND_MINIFY_CORPUS,
        in_corpus = IN_CORPUS_FLAG,
        corpus_size = CORPUS_SIZE_FLAG
    )
    .as_str();

    let args = match FullCommandLineArguments::from_parser(&parser, &env_args[1..]) {
        Ok(r) => r,
        Err(e) => {
            println!("{}\n\n{}", e, help);
            std::process::exit(1);
        }
    };

    fuzzer::launch(test, mutator, serializer, args)
}

/**
 * A unit of code coverage.
 *
 * A `Feature` describes a certain characteristic of the program’s code
 * coverage. For example, it can mean “this control flow edge was reached” or
 * “this instruction was called with these operands”.
 *
 * It is implemented as a wrapper of a `u64` for performance reason. But it
 * actually contains a lot of information.
 *
 * - The first two bits designate the kind of the `Feature`, which can be either
 * `edge`, `indirect`, or `instruction`.
 * - Then, the next 54 bits are the `id` of the feature. They are supposed to
 * uniquely identify a point in the source code.
 * - Finally, the last 8 bits are for the `payload` of the feature. They are
 * the information associated with the feature, such as the number of times
 * the control flow edge was reached or a hash of the operands to the
 * instruction.
 * - Note that for `indirect` features, `id` and `payload` are merged.
 *
 * Each feature has a certain [score](Feature::score) that is currently only
 * determined by its `tag`.
 */
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Feature(u64);

#[cfg(trace_compares)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct InstrFeatureWithoutTag(u64);

#[cfg(trace_compares)]
impl Feature {
    fn from_instr(f: InstrFeatureWithoutTag) -> Self {
        Self((f.0 as u64) | (Feature::instr_tag() << Feature::tag_offset()) as u64)
    }
}

impl Feature {
    /// The bit offset for the id of the feature
    fn id_offset() -> u64 {
        8
    }
    /// The bit offset for the tag of the feature
    fn tag_offset() -> u64 {
        62
    }

    // fn edge_tag() -> u64 {
    //     0b00
    // }
    fn indir_tag() -> u64 {
        0b01
    }

    #[cfg(trace_compares)]
    fn instr_tag() -> u64 {
        0b10
    }
    /// Create a “control flow edge” feature identified by the given `pc_guard`
    /// whose payload is the intensity of the given `counter`.
    fn edge(pc_guard: usize, counter: u16) -> Feature {
        let mut feature: u64 = 0;
        // feature |= 0b00 << Feature::tag_offset();
        // take 32 last bits, I don't want to worry about programs with more than 4 billion instrumented edges anyway
        feature |= ((pc_guard & 0xFFFF_FFFF) as u64) << Feature::id_offset();
        feature |= u64::from(Feature::score_from_counter(counter)); // will only ever be 8 bits long

        Feature(feature)
    }

    // TODO: indir disabled for now
    // fn indir(caller: usize, callee: usize) -> Feature {
    //     let (caller, callee) = (caller as u64, callee as u64);
    //     let mut feature: u64 = 0;
    //     feature |= Feature::indir_tag() << Feature::tag_offset();
    //     feature |= (caller ^ callee) & 0x3FFF_FFFF_FFFF_FFFF;

    //     Feature(feature)
    // }

    fn erasing_payload(self) -> Self {
        if (self.0 >> Self::tag_offset()) == Self::indir_tag() {
            // if it is indirect, there is no payload to erase
            self
        } else {
            // else, zero out the payload bits
            Feature(self.0 & 0xFFFF_FFFF_FFFF_FF00)
        }
    }

    /// “Hash” a u16 into a number between 0 and 16.
    ///
    /// So that similar numbers have the same hash, and very different
    /// numbers have a greater hash.
    fn score_from_counter(counter: u16) -> u8 {
        if counter <= 3 {
            counter as u8
        } else if counter != core::u16::MAX {
            (16 - counter.leading_zeros() + 1) as u8
        } else {
            16
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
    pub fn new(value: T, cache: Mut::Cache, mutation_step: Mut::MutationStep) -> Self {
        Self {
            value,
            cache,
            mutation_step,
        }
    }
    pub fn default(m: &mut Mut) -> Option<Self> {
        if let Some((value, cache)) = m.ordered_arbitrary(&mut <_>::default(), 1.0) {
            let mutation_step = m.initial_step_from_value(&value);
            Some(Self::new(value, cache, mutation_step))
        } else {
            None
        }
    }

    pub fn new_source(&self, m: &Mut) -> Self {
        Self::new(
            self.value.clone(),
            self.cache.clone(),
            m.initial_step_from_value(&self.value),
        )
    }

    pub fn complexity(&self, m: &Mut) -> f64 {
        m.complexity(&self.value, &self.cache)
    }

    pub fn mutate(&mut self, m: &mut Mut, max_cplx: f64) -> Option<Mut::UnmutateToken> {
        m.ordered_mutate(&mut self.value, &mut self.cache, &mut self.mutation_step, max_cplx)
    }

    pub fn unmutate(&mut self, m: &Mut, t: Mut::UnmutateToken) {
        m.unmutate(&mut self.value, &mut self.cache, t);
    }
}

impl<T: Clone, M: Mutator<T>> Clone for FuzzedInput<T, M> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            cache: self.cache.clone(),
            mutation_step: self.mutation_step.clone(),
        }
    }
}
