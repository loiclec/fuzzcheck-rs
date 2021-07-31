//! Fuzzcheck is a coverage-guided, evolutionary fuzzing engine for Rust
//! functions.

#![feature(drain_filter)]
#![feature(never_type)]
#![feature(is_sorted)]
#![feature(link_llvm_intrinsics)]
#![feature(thread_local)]
#![feature(maybe_uninit_slice)]
#![feature(test)]
#![feature(no_coverage)]

pub extern crate fuzzcheck_traits;

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

* The fourth argument are the command line arguments given to the fuzzer. 

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
#[no_coverage]
pub fn launch<T, FT, F, M, S>(test: F, mutator: M, serializer: S, args: Vec<&str>) -> Result<(), std::io::Error>
where
    FT: ?Sized,
    T: Clone + Borrow<FT>,
    F: Fn(&FT) -> bool,
    M: Mutator<T>,
    S: Serializer<Value = T>,
    fuzzer::Fuzzer<T, FT, F, M, S>: 'static,
{
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

    let args = match FullCommandLineArguments::from_parser(&parser, &args) {
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
