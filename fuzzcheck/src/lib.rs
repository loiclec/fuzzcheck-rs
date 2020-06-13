//! Fuzzcheck is a coverage-guided, evolutionary fuzzing engine for Rust
//! functions.

#![feature(drain_filter)]
#![feature(never_type)]
#![feature(thread_spawn_unchecked)]
#![feature(ptr_offset_from)]
#![feature(vec_remove_item)]
#![feature(is_sorted)]
#![feature(link_llvm_intrinsics)]

mod nix_subset;

mod code_coverage_sensor;
mod data_structures;

mod fuzzer;
mod world;

mod pool;
mod signals_handler;

use fuzzcheck_arg_parser::{
    options_parser, CommandLineArguments, COMMAND_FUZZ, COMMAND_MINIFY_CORPUS, COMMAND_MINIFY_INPUT, CORPUS_SIZE_FLAG,
    DEFAULT_ARGUMENTS, INPUT_FILE_FLAG, IN_CORPUS_FLAG,
};

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
pub fn launch<T, F, M, S>(test: F, mutator: M, serializer: S) -> Result<(), std::io::Error>
where
    T: ?Sized,
    M::Value: Borrow<T>,
    F: Fn(&T) -> bool,
    M: Mutator,
    S: Serializer<Value = M::Value>,
    fuzzer::Fuzzer<T, F, M, S>: 'static,
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

    let args = match CommandLineArguments::from_parser(&parser, &env_args[1..], DEFAULT_ARGUMENTS) {
        Ok(r) => r,
        Err(e) => {
            println!("{}\n\n{}", e, help);
            std::process::exit(1);
        }
    };

    fuzzer::launch(test, mutator, serializer, args)
}

/**
 A [Mutator] is an object capable of mutating a value for the purpose of
 fuzz-testing.

 For example, a mutator could change the value
 `v1 = [1, 4, 2, 1]` to `v1' = [1, 5, 2, 1]`.
 The idea is that if v1 is an “interesting” value to test, then v1' also
 has a high chance of being “interesting” to test.

 ## Complexity

 A mutator is also responsible for keeping track of the
 [complexity](crate::Mutator::complexity) of a value. The complexity is,
 roughly speaking, how large the value is.

 For example, the complexity of a vector is the complexity of its length,
 plus  the sum of the complexities of its elements. So `vec![]` would have a
 complexity of `0.0` and `vec![76]` would have a complexity of `9.0`: `1.0`
 for  its short length and `8.0` for the 8-bit integer “76”. But there is no
 fixed rule for how to compute the complexity of a value, and it is up to you
 to judge how “large” something is.

  ## Cache

 In order to mutate values efficiently, the mutator is able to make use of a
 per-value *cache*. The Cache contains information associated with the value
 that will make it faster to compute its complexity or apply a mutation to
 it. For a vector, its cache is its total complexity, along with a vector of
 the cache of each of its element.

  ## MutationStep

 The same values will be passed to the mutator many times, so that it is
 mutated in many different ways. There are different strategies to choose
 what mutation to apply to a value. The first one is to create a list of
 mutation operations, and choose one to apply randomly from this list.

 However, one may want to have better control over which mutation operation
 is used. For example, if the value to be mutated is of type `Option<T>`,
 then you may want to first mutate it to `None`, and then always mutate it
 to another `Some(t)`. This is where `MutationStep` comes in. The mutation
 step is a type you define to allow you to keep track of which mutation
 operation has already been tried. This allows you to deterministically
 apply mutations to a value such that better mutations are tried first, and
 duplicate mutations are avoided.

 ## Unmutate

 Finally, it is important to note that values and caches are mutated
 *in-place*. The fuzzer does not clone them before handing them to the
 mutator. Therefore, the mutator also needs to know how to reverse each
 mutation it performed. To do so, each mutation needs to return a token
 describing how to reverse it. The [unmutate](crate::Mutator::unmutate)
 method will later be called with that token to get the original value
 and cache back.

 For example, if the value is `[[1, 3], [5], [9, 8]]`, the mutator may
 mutate it to `[[1, 3], [5], [9, 1, 8]]` and return the token:
 `Element(2, Remove(1))`, which means that in order to reverse the
 mutation, the element at index 2 has to be unmutated by removing
 its element at index 1. In pseudocode:

 ```ignore
 value = [[1, 3], [5], [9, 8]];
 cache: c1 (ommitted from example)
 step: s1 (ommitted from example)

 let unmutate_token = self.mutate(&mut value, &mut cache, &mut step, max_cplx);

 // value = [[1, 3], [5], [9, 1, 8]]
 // token = Element(2, Remove(1))
 // cache = c2
 // step = s2

 test(&value);

 self.unmutate(&mut value, &mut cache, unmutate_token);

 // value = [[1, 3], [5], [9, 8]]
 // cache = c1 (back to original cache)
 // step = s2 (step has not been reversed)
 ```

**/
pub trait Mutator: Sized {
    type Value: Clone;
    type Cache: Clone;
    type MutationStep;
    type UnmutateToken;

    /// Compute the cache for the given value
    fn cache_from_value(&self, value: &Self::Value) -> Self::Cache;
    /// Compute the initial mutation step for the given value
    fn mutation_step_from_value(&self, value: &Self::Value) -> Self::MutationStep;

    /// The maximum complexity of an input of this type
    fn max_complexity(&self) -> f64;
    /// The minimum complexity of an input of this type
    fn min_complexity(&self) -> f64;
    /// The complexity of the current input
    fn complexity(&self, value: &Self::Value, cache: &Self::Cache) -> f64;

    /// Create an arbitrary value
    fn arbitrary(&mut self, seed: usize, max_cplx: f64) -> (Self::Value, Self::Cache);

    fn mutate(
        &mut self,
        value: &mut Self::Value,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        max_cplx: f64,
    ) -> Self::UnmutateToken;

    fn unmutate(&self, value: &mut Self::Value, cache: &mut Self::Cache, t: Self::UnmutateToken);
}

/**
 * A Serializer is used to encode and decode values into bytes.
 *
 * One possible implementation would be to use `serde` to implement
 * both required functions. But we also want to be able to fuzz-test
 * types that are not serializable with `serde`, which is why this
 * Serializer trait exists.
*/
pub trait Serializer {
    type Value;
    fn extension(&self) -> &str;
    fn from_data(&self, data: &[u8]) -> Option<Self::Value>;
    fn to_data(&self, value: &Self::Value) -> Vec<u8>;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct InstrFeatureWithoutTag(u64);

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
#[derive(Clone)]
struct FuzzedInput<Mut: Mutator> {
    pub value: Mut::Value,
    pub cache: Mut::Cache,
    pub mutation_step: Mut::MutationStep,
}

impl<Mut: Mutator> FuzzedInput<Mut> {
    pub fn new(value: Mut::Value, cache: Mut::Cache, mutation_step: Mut::MutationStep) -> Self {
        Self {
            value,
            cache,
            mutation_step,
        }
    }
    pub fn default(m: &mut Mut) -> Self {
        let (value, cache) = m.arbitrary(0, 1.0);
        let mutation_step = m.mutation_step_from_value(&value);
        Self::new(value, cache, mutation_step)
    }

    pub fn new_source(&self, m: &Mut) -> Self {
        Self::new(
            self.value.clone(),
            self.cache.clone(),
            m.mutation_step_from_value(&self.value),
        )
    }

    pub fn complexity(&self, m: &Mut) -> f64 {
        m.complexity(&self.value, &self.cache)
    }

    pub fn mutate(&mut self, m: &mut Mut, max_cplx: f64) -> Mut::UnmutateToken {
        m.mutate(&mut self.value, &mut self.cache, &mut self.mutation_step, max_cplx)
    }

    pub fn unmutate(&mut self, m: &Mut, t: Mut::UnmutateToken) {
        m.unmutate(&mut self.value, &mut self.cache, t);
    }
}
