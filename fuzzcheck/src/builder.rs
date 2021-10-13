use crate::code_coverage_sensor::CodeCoverageSensor;
use crate::fuzzer::{Fuzzer, ReasonForStopping};
use crate::sensors_and_pools::and_sensor_and_pool::AndPool;
use crate::sensors_and_pools::maximize_pool::CounterMaximizingPool;
use crate::sensors_and_pools::sum_coverage_pool::{
    AggregateCoveragePool, CountNumberOfDifferentCounters, SumCounterValues,
};
use crate::sensors_and_pools::unique_coverage_pool::UniqueCoveragePool;
use crate::traits::{CompatibleWithSensor, Mutator, Pool, Sensor, Serializer};
use crate::{split_string_by_whitespace, DefaultMutator, SerdeSerializer};

use fuzzcheck_common::arg::{options_parser, COMMAND_FUZZ, COMMAND_MINIFY_INPUT, INPUT_FILE_FLAG};
use fuzzcheck_common::arg::{Arguments, FuzzerCommand};
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
pub trait FuzzTestFunction<T, FT: ?Sized, ImplId> {
    type NormalizedFunction: for<'a> Fn(&'a T) -> bool;
    fn test_function(self) -> Self::NormalizedFunction;
}

/// Marker type for a function of type `Fn(&T) -> bool`
pub enum ReturnBool {}
/// Marker type for a function of type `Fn(&T)`
pub enum ReturnVoid {}
/// Marker type for a function of type `Fn(&T) -> Result<V, E>`
pub enum ReturnResult {}

impl<T, FT: ?Sized, F> FuzzTestFunction<T, FT, ReturnBool> for F
where
    T: Borrow<FT>,
    F: Fn(&FT) -> bool,
{
    type NormalizedFunction = impl Fn(&T) -> bool;
    #[no_coverage]
    fn test_function(self) -> Self::NormalizedFunction {
        #[no_coverage]
        move |x| (self)(x.borrow())
    }
}
impl<T, FT: ?Sized, F> FuzzTestFunction<T, FT, ReturnVoid> for F
where
    T: Borrow<FT>,
    F: Fn(&FT),
{
    type NormalizedFunction = impl Fn(&T) -> bool;
    #[no_coverage]
    fn test_function(self) -> Self::NormalizedFunction {
        #[no_coverage]
        move |x| {
            self(x.borrow());
            true
        }
    }
}

impl<T, FT: ?Sized, F, S, E> FuzzTestFunction<T, FT, ReturnResult> for F
where
    T: Borrow<FT>,
    F: Fn(&FT) -> Result<E, S>,
{
    type NormalizedFunction = impl Fn(&T) -> bool;
    #[no_coverage]
    fn test_function(self) -> Self::NormalizedFunction {
        move |x| self(x.borrow()).is_ok()
    }
}
pub struct FuzzerBuilder1<T, F>
where
    T: ?Sized,
    F: Fn(&T) -> bool + 'static,
{
    test_function: F,
    _phantom: PhantomData<*const T>,
}

pub struct FuzzerBuilder2<F, M, V>
where
    F: Fn(&V) -> bool + 'static,
    V: Clone,
    M: Mutator<V>,
{
    test_function: F,
    mutator: M,
    _phantom: PhantomData<*const V>,
}

pub struct FuzzerBuilder3<F, M, V>
where
    F: Fn(&V) -> bool + 'static,
    V: Clone,
    M: Mutator<V>,
{
    test_function: F,
    mutator: M,
    serializer: Box<dyn Serializer<Value = V>>,
    _phantom: PhantomData<*const V>,
}
pub struct FuzzerBuilder4<F, M, V, Sens>
where
    F: Fn(&V) -> bool + 'static,
    V: Clone,
    M: Mutator<V>,
    Sens: Sensor,
{
    test_function: F,
    mutator: M,
    serializer: Box<dyn Serializer<Value = V>>,
    sensor: Sens,
    _phantom: PhantomData<*const V>,
}
pub struct FuzzerBuilder5<F, M, V, Sens, P>
where
    F: Fn(&V) -> bool + 'static,
    V: Clone,
    M: Mutator<V>,
    Sens: Sensor,
    P: Pool + CompatibleWithSensor<Sens>,
{
    test_function: F,
    mutator: M,
    serializer: Box<dyn Serializer<Value = V>>,
    sensor: Sens,
    pool: P,
    _phantom: PhantomData<*const V>,
}
pub struct FuzzerBuilder6<F, M, V, Sens, P>
where
    F: Fn(&V) -> bool + 'static,
    V: Clone,
    M: Mutator<V>,
    Sens: Sensor,
    P: Pool + CompatibleWithSensor<Sens>,
{
    test_function: F,
    mutator: M,
    serializer: Box<dyn Serializer<Value = V>>,
    sensor: Sens,
    pool: P,
    arguments: Arguments,
    _phantom: PhantomData<*const V>,
}

/**
    Specify the function to fuzz-test.

    There are currently three kinds of functions that can be passed as arguments:

    1. `Fn(&T)` : the fuzzer will only report a failure when the given function crashes
    2. `Fn(&T) -> Bool` : the fuzzer will report a failure when the output is `false`
    3. `Fn(&T) -> Result<_,_>` : the fuzzer will report a failure when the output is `Err(..)`
*/
#[no_coverage]
pub fn fuzz_test<T, F, TestFunctionKind>(test_function: F) -> FuzzerBuilder1<T::Owned, F::NormalizedFunction>
where
    T: ?Sized + ToOwned + 'static,
    T::Owned: Clone,
    F: FuzzTestFunction<T::Owned, T, TestFunctionKind>,
{
    FuzzerBuilder1 {
        test_function: test_function.test_function(),
        _phantom: PhantomData,
    }
}

impl<T, F> FuzzerBuilder1<T, F>
where
    T: ?Sized + ToOwned + 'static,
    T::Owned: Clone + serde::Serialize + for<'e> serde::Deserialize<'e> + DefaultMutator,
    <T::Owned as DefaultMutator>::Mutator: 'static,
    F: Fn(&T) -> bool,
    F: FuzzTestFunction<T::Owned, T, ReturnBool>,
{
    #[no_coverage]
    pub fn default_options(
        self,
    ) -> FuzzerBuilder6<
        F::NormalizedFunction,
        <T::Owned as DefaultMutator>::Mutator,
        T::Owned,
        CodeCoverageSensor,
        impl Pool + CompatibleWithSensor<CodeCoverageSensor>,
    > {
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
        ```ignore
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
    pub fn mutator<M, V>(self, mutator: M) -> FuzzerBuilder2<F::NormalizedFunction, M, V>
    where
        V: Clone + Borrow<T>,
        F: FuzzTestFunction<V, T, ReturnBool>,
        M: Mutator<V>,
    {
        FuzzerBuilder2 {
            test_function: self.test_function.test_function(),
            mutator,
            _phantom: PhantomData,
        }
    }
}

impl<F, M, V> FuzzerBuilder2<F, M, V>
where
    F: Fn(&V) -> bool,
    V: Clone,
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
    pub fn serializer<S>(self, serializer: S) -> FuzzerBuilder3<F, M, V>
    where
        S: Serializer<Value = V> + 'static,
    {
        FuzzerBuilder3 {
            test_function: self.test_function,
            mutator: self.mutator,
            serializer: Box::new(serializer),
            _phantom: PhantomData,
        }
    }
}
impl<F, M, V> FuzzerBuilder3<F, M, V>
where
    F: Fn(&V) -> bool,
    V: Clone,
    M: Mutator<V>,
{
    #[no_coverage]
    pub fn default_sensor(self) -> FuzzerBuilder4<F, M, V, CodeCoverageSensor> {
        let sensor = CodeCoverageSensor::observing_only_files_from_current_dir();
        let coverage_map = sensor.coverage_map();
        let file = Path::new("coverage_map.json");
        let contents = serde_json::to_vec_pretty(&coverage_map).unwrap();
        std::fs::write(file, contents).unwrap();

        FuzzerBuilder4 {
            test_function: self.test_function,
            mutator: self.mutator,
            serializer: self.serializer,
            sensor,
            _phantom: PhantomData,
        }
    }
    #[no_coverage]
    pub fn sensor<Sens: Sensor>(self, sensor: Sens) -> FuzzerBuilder4<F, M, V, Sens> {
        FuzzerBuilder4 {
            test_function: self.test_function,
            mutator: self.mutator,
            serializer: self.serializer,
            sensor,
            _phantom: PhantomData,
        }
    }
}

#[no_coverage]
pub fn default_sensor_and_pool() -> (CodeCoverageSensor, impl CompatibleWithSensor<CodeCoverageSensor>) {
    let sensor = CodeCoverageSensor::observing_only_files_from_current_dir();
    let coverage_map = sensor.coverage_map();
    let file = Path::new("coverage_map.json");
    let contents = serde_json::to_vec_pretty(&coverage_map).unwrap();
    std::fs::write(file, contents).unwrap();
    let pool = defaul_pool_for_code_coverage_sensor(&sensor);
    (sensor, pool)
}

#[no_coverage]
fn defaul_pool_for_code_coverage_sensor(sensor: &CodeCoverageSensor) -> impl CompatibleWithSensor<CodeCoverageSensor> {
    let count_instrumented = sensor.count_instrumented;
    let pool = UniqueCoveragePool::new("uniq_cov", count_instrumented);
    let pool2 = CounterMaximizingPool::new("high_cov_hits", count_instrumented);

    let pool4 = AggregateCoveragePool::<SumCounterValues>::new("highest_aggregate_cov_hits");
    let pool5 = AggregateCoveragePool::<CountNumberOfDifferentCounters>::new("most_diverse_cov");

    let pool = AndPool::new(pool2, pool, 1);
    let pool = AndPool::new(pool, pool4, 254);
    let pool = AndPool::new(pool, pool5, 254);
    pool
}

impl<F, M, V> FuzzerBuilder4<F, M, V, CodeCoverageSensor>
where
    F: Fn(&V) -> bool,
    V: Clone,
    M: Mutator<V>,
{
    #[no_coverage]
    pub fn default_pool(
        self,
    ) -> FuzzerBuilder5<F, M, V, CodeCoverageSensor, impl Pool + CompatibleWithSensor<CodeCoverageSensor>> {
        let pool = defaul_pool_for_code_coverage_sensor(&self.sensor);
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

impl<F, M, V, Sens> FuzzerBuilder4<F, M, V, Sens>
where
    F: Fn(&V) -> bool,
    V: Clone,
    M: Mutator<V>,
    Sens: Sensor,
{
    #[no_coverage]
    pub fn pool<P>(self, pool: P) -> FuzzerBuilder5<F, M, V, Sens, P>
    where
        P: Pool + CompatibleWithSensor<Sens>,
    {
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

impl<F, M, V, Sens, P> FuzzerBuilder5<F, M, V, Sens, P>
where
    F: Fn(&V) -> bool,
    V: Clone,
    M: Mutator<V>,
    Sens: Sensor,
    P: Pool + CompatibleWithSensor<Sens>,
{
    #[no_coverage]
    pub fn arguments(self, arguments: Arguments) -> FuzzerBuilder6<F, M, V, Sens, P> {
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
    pub fn arguments_from_cargo_fuzzcheck(self) -> FuzzerBuilder6<F, M, V, Sens, P> {
        let parser = options_parser();
        let mut help = format!(
            r#""
fuzzcheck <SUBCOMMAND> [OPTIONS]

SUBCOMMANDS:
    {fuzz}    Run the fuzz test
    {tmin}    Minify a crashing test input, requires --{input_file}
"#,
            fuzz = COMMAND_FUZZ,
            tmin = COMMAND_MINIFY_INPUT,
            input_file = INPUT_FILE_FLAG,
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
"#,
            fuzz = COMMAND_FUZZ,
            tmin = COMMAND_MINIFY_INPUT,
            input_file = INPUT_FILE_FLAG,
        )
        .as_str();

        let arguments = std::env::var("FUZZCHECK_ARGS").unwrap();
        let arguments = split_string_by_whitespace(&arguments);
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

impl<F, M, V, Sens, P> FuzzerBuilder6<F, M, V, Sens, P>
where
    F: Fn(&V) -> bool + 'static,
    V: Clone,
    M: Mutator<V>,
    Sens: Sensor,
    P: Pool + CompatibleWithSensor<Sens>,
    Fuzzer<V, M, Sens, P>: 'static,
{
    #[no_coverage]
    pub fn command(self, command: FuzzerCommand) -> Self {
        let mut x = self;
        x.arguments.command = command;
        x
    }
    #[no_coverage]
    pub fn in_corpus(self, path: Option<&Path>) -> Self {
        let mut x = self;
        x.arguments.corpus_in = path.map(Path::to_path_buf);
        x
    }
    #[no_coverage]
    pub fn out_corpus(self, path: Option<&Path>) -> Self {
        let mut x = self;
        x.arguments.corpus_out = path.map(Path::to_path_buf);
        x
    }
    #[no_coverage]
    pub fn artifacts_folder(self, path: Option<&Path>) -> Self {
        let mut x = self;
        x.arguments.artifacts_folder = path.map(Path::to_path_buf);
        x
    }
    #[no_coverage]
    pub fn maximum_complexity(self, max_input_cplx: f64) -> Self {
        let mut x = self;
        x.arguments.max_input_cplx = max_input_cplx;
        x
    }
    #[no_coverage]
    pub fn stop_after_iterations(self, number_of_iterations: usize) -> Self {
        let mut x = self;
        x.arguments.maximum_iterations = number_of_iterations;
        x
    }
    #[no_coverage]
    pub fn stop_after_duration(self, duration: Duration) -> Self {
        let mut x = self;
        x.arguments.maximum_duration = duration;
        x
    }
    #[no_coverage]
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

        crate::fuzzer::launch(Box::new(test_function), mutator, serializer, sensor, pool, arguments)
    }
}
