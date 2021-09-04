use crate::code_coverage_sensor::CodeCoverageSensor;
use crate::fuzzer::{Fuzzer, ReasonForStopping};
use crate::sensors_and_pools::and_sensor_and_pool::{AndPool, AndSensor};
use crate::sensors_and_pools::artifacts_pool::{ArtifactsPool, TestFailureSensor};
use crate::sensors_and_pools::maximize_pool::CounterMaximizingPool;
use crate::sensors_and_pools::sum_coverage_pool::{AggregateCoveragePool, CountNumberOfDifferentCounters, SumCounterValues};
use crate::sensors_and_pools::unique_coverage_pool::UniqueCoveragePool;
use crate::traits::{CompatibleWithSensor, Mutator, Pool, Sensor, Serializer};
use crate::{DefaultMutator, FuzzedInput, SerdeSerializer};

use fuzzcheck_common::arg::{Arguments, FuzzerCommand};
use fuzzcheck_common::arg::{
    options_parser, COMMAND_FUZZ, COMMAND_MINIFY_CORPUS, COMMAND_MINIFY_INPUT, CORPUS_SIZE_FLAG, INPUT_FILE_FLAG,
    IN_CORPUS_FLAG,
};
use std::borrow::Borrow;
use std::marker::PhantomData;
use std::path::Path;
use std::result::Result;
use std::time::Duration;

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
    #[no_coverage]
    fn test_function(self) -> Self::NormalizedFunction {
        self
    }
}
impl<T: ?Sized, F> FuzzTestFunction<T, ReturnVoid> for F
where
    F: Fn(&T),
{
    type NormalizedFunction = impl Fn(&T) -> bool;
    #[no_coverage]
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
    #[no_coverage]
    fn test_function(self) -> Self::NormalizedFunction {
        move |x| self(x).is_ok()
    }
}
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
pub struct FuzzerBuilder4<T, F, M, V, S, Sens>
where
    T: ?Sized,
    F: Fn(&T) -> bool,
    V: Clone + Borrow<T>,
    M: Mutator<V>,
    S: Serializer<Value = V>,
    Sens: Sensor,
{
    test_function: F,
    mutator: M,
    serializer: S,
    sensor: Sens,
    _phantom: PhantomData<(*const T, V)>,
}
pub struct FuzzerBuilder5<T, F, M, V, S, Sens, P>
where
    T: ?Sized,
    F: Fn(&T) -> bool,
    V: Clone + Borrow<T>,
    M: Mutator<V>,
    S: Serializer<Value = V>,
    Sens: Sensor,
    P: Pool<TestCase = FuzzedInput<V, M>> + CompatibleWithSensor<Sens>
{
    test_function: F,
    mutator: M,
    serializer: S,
    sensor: Sens,
    pool: P,
    _phantom: PhantomData<(*const T, V)>,
}
pub struct FuzzerBuilder6<T, F, M, V, S, Sens, P>
where
    T: ?Sized,
    F: Fn(&T) -> bool,
    V: Clone + Borrow<T>,
    M: Mutator<V>,
    S: Serializer<Value = V>,
    Sens: Sensor,
    P: Pool<TestCase = FuzzedInput<V, M>> + CompatibleWithSensor<Sens>
{
    test_function: F,
    mutator: M,
    serializer: S,
    sensor: Sens,
    pool: P,
    arguments: Arguments,
    _phantom: PhantomData<(*const T, V)>,
}

/**
    Specify the function to fuzz-test.

    There are currently three kinds of functions that can be passed as arguments:

    1. `Fn(&T)` : the fuzzer will only report a failure when the given function crashes
    2. `Fn(&T) -> Bool` : the fuzzer will report a failure when the output is `false`
    3. `Fn(&T) -> Result<_,_>` : the fuzzer will report a failure when the output is `Err(..)`
*/
#[no_coverage]
pub fn fuzz_test<T, F, TestFunctionKind>(test_function: F) -> FuzzerBuilder1<T, F::NormalizedFunction>
where
    T: ?Sized,
    F: FuzzTestFunction<T, TestFunctionKind>,
{
    FuzzerBuilder1 {
        test_function: test_function.test_function(),
        _phantom: PhantomData,
    }
}

#[cfg(feature="serde_json_serializer")]
impl<T, F> FuzzerBuilder1<T, F>
where
    T: ?Sized + ToOwned + 'static,
    T::Owned: Clone + serde::Serialize + for<'e> serde::Deserialize<'e> + DefaultMutator,
    <T::Owned as DefaultMutator>::Mutator : 'static,
    F: Fn(&T) -> bool + 'static,
{
    pub fn default_options(self) -> FuzzerBuilder6<T, F, <T::Owned as DefaultMutator>::Mutator, T::Owned, SerdeSerializer<T::Owned>, AndSensor<CodeCoverageSensor, TestFailureSensor>, impl Pool<TestCase = FuzzedInput<T::Owned, <T::Owned as DefaultMutator>::Mutator>> + CompatibleWithSensor<AndSensor<CodeCoverageSensor, TestFailureSensor>>> {
        self.mutator(<T::Owned as DefaultMutator>::default_mutator())
            .serializer(SerdeSerializer::default())
            .default_sensor()
            .default_pool()
            .arguments_from_cargo_fuzzcheck()
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
    #[no_coverage]
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
    #[no_coverage]
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
    #[no_coverage]
    pub fn default_sensor(self) -> FuzzerBuilder4<T, F, M, V, S, AndSensor<CodeCoverageSensor, TestFailureSensor>> {
        let codecov = CodeCoverageSensor::new(|_| true, |f| f.is_relative());
        let test_failure = TestFailureSensor::default();
        let sensor = AndSensor {
            s1: codecov,
            s2: test_failure,
        };
        FuzzerBuilder4 {
            test_function: self.test_function,
            mutator: self.mutator,
            serializer: self.serializer,
            sensor,
            _phantom: PhantomData,
        }
    }
    pub fn sensor<Sens: Sensor>(self, sensor: Sens) -> FuzzerBuilder4<T, F, M, V, S, Sens> {
         FuzzerBuilder4 {
            test_function: self.test_function,
            mutator: self.mutator,
            serializer: self.serializer,
            sensor,
            _phantom: PhantomData,
        }
    }
}

impl<T, F, M, V, S> FuzzerBuilder4<T, F, M, V, S, AndSensor<CodeCoverageSensor, TestFailureSensor>>
where
    T: ?Sized,
    F: Fn(&T) -> bool,
    V: Clone + Borrow<T>,
    M: Mutator<V>,
    S: Serializer<Value = V>,
{
    #[no_coverage]
    pub fn default_pool(self) -> FuzzerBuilder5<T, F, M, V, S, AndSensor<CodeCoverageSensor, TestFailureSensor>, impl Pool<TestCase = FuzzedInput<V,M>> + CompatibleWithSensor<AndSensor<CodeCoverageSensor, TestFailureSensor>>> {
        let count_instrumented = self.sensor.s1.count_instrumented;
        let nbr_features = count_instrumented * 64; // TODO: change that once the size of feature groups is decided

        let pool = UniqueCoveragePool::new("uniq_cov", nbr_features); // TODO: reduce nbr of possible values from score_from_counter
        let pool2 = CounterMaximizingPool::new("max_hits", count_instrumented);
        let pool3 = ArtifactsPool::new("artifacts");
        let pool4 = AggregateCoveragePool::<_, SumCounterValues>::new("sum_counters");
        let pool5 = AggregateCoveragePool::<_, CountNumberOfDifferentCounters>::new("count_differents_counters");
        let pool = AndPool {
            p1: pool2,
            p2: pool,
            percent_choose_first: 10,
            rng: fastrand::Rng::new(),
        };
        let pool = AndPool {
            p1: pool,
            p2: pool4,
            percent_choose_first: 99,
            rng: fastrand::Rng::new(),
        };
        let pool = AndPool {
            p1: pool,
            p2: pool5,
            percent_choose_first: 99,
            rng: fastrand::Rng::new(),
        };
        let pool = AndPool {
            p1: pool,
            p2: pool3,
            percent_choose_first: 99,
            rng: fastrand::Rng::new(),
        };

        FuzzerBuilder5 {
            test_function: self.test_function,
            mutator: self.mutator,
            serializer: self.serializer,
            sensor: self.sensor,
            pool,
            _phantom: PhantomData,
        }
    }
}

impl<T, F, M, V, S, Sens> FuzzerBuilder4<T, F, M, V, S, Sens>
where
    T: ?Sized,
    F: Fn(&T) -> bool,
    V: Clone + Borrow<T>,
    M: Mutator<V>,
    S: Serializer<Value = V>,
    Sens: Sensor,
{
    #[no_coverage]
    pub fn pool<P>(self, pool: P) -> FuzzerBuilder5<T, F, M, V, S, Sens, P> where P: Pool<TestCase = FuzzedInput<V, M>> + CompatibleWithSensor<Sens> {
        FuzzerBuilder5 {
            test_function: self.test_function,
            mutator: self.mutator,
            serializer: self.serializer,
            sensor: self.sensor,
            pool,
            _phantom: PhantomData,
        }
    }
}

impl<T, F, M, V, S, Sens, P> FuzzerBuilder5<T, F, M, V, S, Sens, P>
where
    T: ?Sized,
    F: Fn(&T) -> bool,
    V: Clone + Borrow<T>,
    M: Mutator<V>,
    S: Serializer<Value = V>,
    Sens: Sensor,
    P: Pool<TestCase = FuzzedInput<V, M>> + CompatibleWithSensor<Sens>
{
    #[no_coverage]
    pub fn arguments(self, arguments: Arguments) -> FuzzerBuilder6<T, F, M, V, S, Sens, P> {
        FuzzerBuilder6 {
            test_function: self.test_function,
            mutator: self.mutator,
            serializer: self.serializer,
            sensor: self.sensor,
            pool: self.pool,
            arguments,
            _phantom: self._phantom,
        }
    }
    #[no_coverage]
    pub fn arguments_from_cargo_fuzzcheck(self) -> FuzzerBuilder6<T, F, M, V, S, Sens, P> {
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
        FuzzerBuilder6 {
            test_function: self.test_function,
            mutator: self.mutator,
            serializer: self.serializer,
            sensor: self.sensor,
            pool: self.pool,
            arguments,
            _phantom: PhantomData,
        }
    }
}

impl<T, F, M, V, S, Sens, P> FuzzerBuilder6<T, F, M, V, S, Sens, P>
where
    T: ?Sized,
    F: Fn(&T) -> bool,
    V: Clone + Borrow<T>,
    M: Mutator<V>,
    S: Serializer<Value = V>,
    Sens: Sensor,
    P: Pool<TestCase = FuzzedInput<V, M>> + CompatibleWithSensor<Sens>,
    Fuzzer<V, T, F, M, S, Sens, P>: 'static,
{
    pub fn command(self, command: FuzzerCommand) -> Self {
        let mut x = self;
        x.arguments.command = command;
        x
    }
    pub fn in_corpus(self, path: Option<&Path>) -> Self {
        let mut x = self;
        x.arguments.corpus_in = path.map(Path::to_path_buf);
        x
    }
    pub fn out_corpus(self, path: Option<&Path>) -> Self {
        let mut x = self;
        x.arguments.corpus_out = path.map(Path::to_path_buf);
        x
    }
    pub fn artifacts_folder(self, path: Option<&Path>) -> Self {
        let mut x = self;
        x.arguments.artifacts_folder = path.map(Path::to_path_buf);
        x
    }
    pub fn maximum_complexity(self, max_input_cplx: f64) -> Self {
        let mut x = self;
        x.arguments.max_input_cplx = max_input_cplx;        
        x
    }
    pub fn stop_after_iterations(self, number_of_iterations: usize) -> Self {
        let mut x = self;
        x.arguments.maximum_iterations = number_of_iterations;
        x
    }
    pub fn stop_after_duration(self, duration: Duration) -> Self {
        let mut x = self;
        x.arguments.maximum_duration = duration;
        x
    }
    pub fn stop_after_first_test_failure(self, stop_after_first_test_failure: bool) -> Self {
        let mut x = self;
        x.arguments.stop_after_first_failure = stop_after_first_test_failure;
        x
    }
     /// Launch the fuzz test!
    #[no_coverage]
    pub fn launch(self) -> Result<(), ReasonForStopping<V>> {
        #[cfg(fuzzing)]
        self.launch_even_if_cfg_fuzzing_is_not_set()?;
        Ok(())
    }
    
    /// do not use
    #[no_coverage]
    pub fn launch_even_if_cfg_fuzzing_is_not_set(self) -> Result<(), ReasonForStopping<V>> {
        let FuzzerBuilder6 {
            test_function,
            mutator,
            serializer,
            pool, 
            sensor,
            arguments,
            _phantom,
        } = self;

        crate::fuzzer::launch(
            test_function,
            mutator,
            serializer,
            sensor,
            pool,
            arguments,
        )
    }
}
