use crate::fuzzer::{self, Fuzzer};
use crate::traits::{Mutator, Serializer};
use fuzzcheck_common::arg::Arguments;
use fuzzcheck_common::arg::{
    options_parser, COMMAND_FUZZ, COMMAND_MINIFY_CORPUS, COMMAND_MINIFY_INPUT, CORPUS_SIZE_FLAG, INPUT_FILE_FLAG,
    IN_CORPUS_FLAG,
};
use std::borrow::Borrow;
use std::marker::PhantomData;
use std::result::Result;

/** A function that can be fuzz-tested.

Strictly speaking, fuzzcheck can only test functions of type `Fn(&T) -> bool`.
Using this trait, we can convert other types of functions to `Fn(&T) -> bool`
automatically. For example, a function `fn foo(x: &u8) -> Result<T, E>` can be
wrapped in a closure that returns `true` iff `foo(x)` is `Ok(..)`.
*/
pub trait FuzzTestFunction<T: ?Sized, ImplId> {
    type NormalizedFunction: for<'a> Fn(&'a T) -> bool;
    fn test_function(self) -> Self::NormalizedFunction;
}

/// Marker type for a function of type `Fn(&T) -> bool`
pub enum ReturnBool {}
/// Marker type for a function of type `Fn(&T)`
pub enum ReturnVoid {}
/// Marker type for a function of type `Fn(&T) -> Result<V, E>`
pub enum ReturnResult {}

impl<T: ?Sized, F> FuzzTestFunction<T, ReturnBool> for F
where
    F: Fn(&T) -> bool,
{
    type NormalizedFunction = Self;

    fn test_function(self) -> Self::NormalizedFunction {
        self
    }
}
impl<T: ?Sized, F> FuzzTestFunction<T, ReturnVoid> for F
where
    F: Fn(&T),
{
    type NormalizedFunction = impl Fn(&T) -> bool;

    fn test_function(self) -> Self::NormalizedFunction {
        move |x| {
            self(x);
            true
        }
    }
}

impl<T: ?Sized, F, S, E> FuzzTestFunction<T, ReturnResult> for F
where
    F: Fn(&T) -> Result<E, S>,
{
    type NormalizedFunction = impl Fn(&T) -> bool;

    fn test_function(self) -> Self::NormalizedFunction {
        move |x| self(x).is_ok()
    }
}

/**
    Use this builder type to construct a fuzz test and launch it.

    A fuzz-test is constructed by passing these four arguments, in order:

    1. the function to fuzz-test
    2. the mutator that produces the test cases
    3. the serializer to use when saving the interesting test cases to the file system
    4. other fuzzing arguments, which may be produced by `cargo-fuzzcheck`, or specified manually

    For example, you may write:
    ```
    #![feature(no_coverage)]
    use fuzzcheck::{FuzzerBuilder, DefaultMutator, SerdeSerializer};

    fn my_function(xs: &Option<u16>) -> bool {
        // ..
        # false
    }
    fn fuzz_test() {
        FuzzerBuilder::test(my_function)
            .mutator(<Option<u16>>::default_mutator())
            .serializer(SerdeSerializer::default())
            .arguments_from_cargo_fuzzcheck()
            .launch();
    }
    ```

    Each step is performed on a different type. You start with a
    `FuzzerBuilder`, which asks for a test function. Once that test function
    is given, you get a `FuzzerBuilder1`, which asks for a mutator.
    `FuzzerBuilder2` asks for a serializer. `FuzzerBuilder3` asks for fuzzing
    arguments. And finally, `FuzzerBuilder4` has all the information needed to
    launch the fuzz test.
*/
pub enum FuzzerBuilder {}

pub struct FuzzerBuilder1<T, F>
where
    T: ?Sized,
    F: Fn(&T) -> bool,
{
    test_function: F,
    _phantom: PhantomData<T>,
}

pub struct FuzzerBuilder2<T, F, M, V>
where
    T: ?Sized,
    F: Fn(&T) -> bool,
    V: Clone + Borrow<T>,
    M: Mutator<V>,
{
    test_function: F,
    mutator: M,
    _phantom: PhantomData<(*const T, V)>,
}

pub struct FuzzerBuilder3<T, F, M, V, S>
where
    T: ?Sized,
    F: Fn(&T) -> bool,
    V: Clone + Borrow<T>,
    M: Mutator<V>,
    S: Serializer<Value = V>,
{
    test_function: F,
    mutator: M,
    serializer: S,
    _phantom: PhantomData<(*const T, V)>,
}

pub struct FuzzerBuilder4<T, F, M, V, S>
where
    T: ?Sized,
    F: Fn(&T) -> bool,
    V: Clone + Borrow<T>,
    M: Mutator<V>,
    S: Serializer<Value = V>,
{
    test_function: F,
    mutator: M,
    serializer: S,
    arguments: Arguments,
    _phantom: PhantomData<(*const T, V)>,
}

impl FuzzerBuilder {
    /**
        Specify the function to fuzz-test.

        There are currently three kinds of functions that can be passed as arguments:

        1. `Fn(&T)` : the fuzzer will only report a failure when the given function crashes
        2. `Fn(&T) -> Bool` : the fuzzer will report a failure when the output is `false`
        3. `Fn(&T) -> Result<_,_>` : the fuzzer will report a failure when the output is `Err(..)`
    */
    pub fn test<T, F, TestFunctionKind>(test_function: F) -> FuzzerBuilder1<T, F::NormalizedFunction>
    where
        T: ?Sized,
        F: FuzzTestFunction<T, TestFunctionKind>,
    {
        FuzzerBuilder1 {
            test_function: test_function.test_function(),
            _phantom: PhantomData,
        }
    }
}

impl<T, F> FuzzerBuilder1<T, F>
where
    T: ?Sized,
    F: Fn(&T) -> bool,
{
    /**
        Specify the mutator that produces input values for the tested function.

        The easiest way to create a mutator is to use the `fuzzcheck_mutators` crate,
        which is automatically included in fuzzcheck when compiled with the “mutators”
        feature.

        For example, if the test function is:
        ```
        fn foo(xs: &[u8]) {
            // ..
        }
        ```
        Then the given mutator should produces values that can be borrowed as `[u8]`.
        We can write:
        ```
        use fuzzcheck::{FuzzerBuilder, DefaultMutator};
        # fn foo(xs: &[u8]) {
        #     // ..
        # }

        fn fuzz_test() {
            FuzzerBuilder::test(foo)
                .mutator(Vec::<u8>::default_mutator())
                // ..
                # ;
        }
        ```
    */
    pub fn mutator<M, V>(self, mutator: M) -> FuzzerBuilder2<T, F, M, V>
    where
        V: Clone + Borrow<T>,
        M: Mutator<V>,
    {
        FuzzerBuilder2 {
            test_function: self.test_function,
            mutator,
            _phantom: PhantomData,
        }
    }
}

impl<T, F, M, V> FuzzerBuilder2<T, F, M, V>
where
    T: ?Sized,
    F: Fn(&T) -> bool,
    V: Clone + Borrow<T>,
    M: Mutator<V>,
{
    /**
        Specify the serializer to use when saving the interesting test cases to the file system.

        The serializer must conform the [Serializer](fuzzcheck_traits::Serializer) trait. If you wish
        to use `serde`, you can compile fuzzcheck with the `serde_json_serializer` feature, which exposes
        `fuzzcheck::fuzzcheck_serializer::SerdeSerializer`. You can then write:
        ```ignore
        FuzzerBuilder::test(foo)
            .mutator(/* .. */)
            .serializer(SerdeSerializer::default())
        ```
    */
    pub fn serializer<S>(self, serializer: S) -> FuzzerBuilder3<T, F, M, V, S>
    where
        S: Serializer<Value = V>,
    {
        FuzzerBuilder3 {
            test_function: self.test_function,
            mutator: self.mutator,
            serializer,
            _phantom: PhantomData,
        }
    }
}
impl<T, F, M, V, S> FuzzerBuilder3<T, F, M, V, S>
where
    T: ?Sized,
    F: Fn(&T) -> bool,
    V: Clone + Borrow<T>,
    M: Mutator<V>,
    S: Serializer<Value = V>,
{
    /**
        Use the arguments provided to cargo-fuzzcheck when launching this test.
    */
    pub fn arguments_from_cargo_fuzzcheck(self) -> FuzzerBuilder4<T, F, M, V, S> {
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

        let arguments = std::env::var("FUZZCHECK_ARGS").unwrap();
        let arguments = arguments.split_ascii_whitespace().collect::<Vec<_>>();
        let arguments = match Arguments::from_parser(&parser, &arguments) {
            Ok(r) => r,
            Err(e) => {
                println!("{}\n\n{}", e, help);
                std::process::exit(1);
            }
        };

        FuzzerBuilder4 {
            test_function: self.test_function,
            mutator: self.mutator,
            serializer: self.serializer,
            arguments,
            _phantom: PhantomData,
        }
    }
}

impl<T, F, M, V, S> FuzzerBuilder4<T, F, M, V, S>
where
    T: ?Sized,
    F: Fn(&T) -> bool,
    V: Clone + Borrow<T>,
    M: Mutator<V>,
    S: Serializer<Value = V>,
    Fuzzer<V, T, F, M, S>: 'static,
{
    /**
        Launch the fuzz test!

        This method will either:

        * never return
            * if the fuzz-test does not find any crash and continuously keeps
        running
            * if a test failure or crash is detected
            * if the command line arguments could not be parsed. It will then print the
            help section of cargo-fuzzcheck and exit.

        * return `Ok(())` if the maximum number of iterations has been reached

        * return an error if some necessary IO operation could not be performed. You
        generally do not need to catch or handle the error.
    */
    pub fn launch(self) {
        let FuzzerBuilder4 {
            test_function,
            mutator,
            serializer,
            arguments,
            _phantom,
        } = self;

        fuzzer::launch(test_function, mutator, serializer, arguments).unwrap();
    }
}
