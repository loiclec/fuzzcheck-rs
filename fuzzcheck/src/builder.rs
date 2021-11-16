/*!
Builders used to set up a fuzz test.

This module contains 5 types to build a fuzz test: `FuzzerBuilder[1–5]`.

The idea is to help you specify each part of the fuzzer progressively:
1. the function to fuzz
2. the [mutator](Mutator) to generate arguments to the test function (called “inputs” or “test cases”)
3. the [serializer](Serializer) to save test cases to the file system
4. the [sensor](Sensor) to provide feedback after running the test function, and the [pool](Pool) to interpret the feedback from the sensor
5. [other settings](Arguments) for the fuzzer, such as the maximum allowed complexity for the test cases, where to save the corpora or artifacts on the file system, etc.

In most cases, you don't need to manually specify all these components. If the argument type of the function has a [default mutator](DefaultMutator) and is serializable
with serde, then you can write:
```no_run
# fn test_function(x: &bool) {}
let _ = fuzzcheck::fuzz_test(test_function) // FuzzerBuilder1
    .default_options() // FuzzerBuilder5!  we use the default values for stages 2 to 5
    .launch();

```
This is equivalent to:
```no_run
# use fuzzcheck::DefaultMutator;
# fn test_function(x: &bool) {}
#
# fn fuzz() {
let _ = fuzzcheck::fuzz_test(test_function)
    .default_mutator()      // the default is `<T as DefaultMutator>::default_mutator()`
    .serde_serializer()   // the default is `SerdeSerializer::new()`
    .default_sensor_and_pool() // the default is `default_sensor_and_pool().finish()`
    .arguments_from_cargo_fuzzcheck()
    .launch();
# }
```
If you'd like to use a custom mutator, serializer, sensor and pool, or argumemts, you can write:
```no_run
# use fuzzcheck::DefaultMutator;
# use fuzzcheck::builder::default_sensor_and_pool;
# use fuzzcheck::Arguments;
# fn test_function(x: &bool) {}
#
# fn fuzz() {
# let my_mutator = bool::default_mutator();
# let my_serializer = fuzzcheck::SerdeSerializer::default();
# let (sensor, pool) = default_sensor_and_pool().finish();
# let arguments: Arguments = todo!();
let _ = fuzzcheck::fuzz_test(test_function)
    .mutator(my_mutator)         // the default is `<T as DefaultMutator>::default_mutator()`
    .serializer(my_serializer)   // the default is `SerdeSerializer::new()`
    .sensor_and_pool(sensor, pool)
    .arguments(arguments)
    .launch();
# }
```

To build a custom sensor and pool, you may want to look at the [`Sensor`], [`Pool`], and [`CompatibleWithSensor`] traits.
You can also look at the types provided in the [`sensors_and_pools`](crate::sensors_and_pools) module. But the easiest way to customize them
is to use the [`CodeCoverageSensorAndPoolBuilder`], although it only offers a couple limited options.
*/

use crate::code_coverage_sensor::CodeCoverageSensor;
use crate::fuzzer::{Fuzzer, FuzzingResult};
use crate::sensors_and_pools::AndPool;
use crate::sensors_and_pools::MaximiseCounterValuePool;
use crate::sensors_and_pools::MostNDiversePool;
use crate::sensors_and_pools::SimplestToActivateCounterPool;
use crate::sensors_and_pools::{NumberOfActivatedCounters, OptimiseAggregateStatPool, SumOfCounterValues};
use crate::traits::{CompatibleWithSensor, Mutator, Pool, Sensor, Serializer};
use crate::{split_string_by_whitespace, DefaultMutator, SerdeSerializer};

use fuzzcheck_common::arg::{options_parser, ArgumentsError, COMMAND_FUZZ, COMMAND_MINIFY_INPUT, INPUT_FILE_FLAG};
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

/// A fuzz-test builder that knows the function to fuzz-test. It is created by calling [`fuzz_test(..)`](fuzz_test).
///
/// Use [`self.mutator(..)`](FuzzerBuilder1::mutator) to specify the [mutator](Mutator)
/// and obtain a [`FuzzerBuilder2`]. If the function argument’s type implements [`DefaultMutator`],
/// you can also use [`self.default_mutator()`](FuzzerBuilder1::default_mutator).
///
/// Alternatively, use [`self.default_options()`](FuzzerBuilder1::default_options)
/// to use the default mutator, serializer, sensor, pool, and arguments, and obtain a [`FuzzerBuilder5`].
/// This method is only available if the argument of the test function implements [`DefaultMutator`]
/// and is serializable with serde.
pub struct FuzzerBuilder1<T, F>
where
    T: ?Sized,
    F: Fn(&T) -> bool + 'static,
{
    test_function: F,
    _phantom: PhantomData<*const T>,
}

/// A fuzz-test builder that knows the function to fuzz-test and the mutator.
///
/// Use [`self.serializer(..)`](FuzzerBuilder2::serializer) to specify the [serializer](Serializer) and obtain a [`FuzzerBuilder3`].
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

/// A fuzz-test builder that knows the function to fuzz-test, the mutator, and the serializer.
///
/// Use [`self.sensor_and_pool(..)`](FuzzerBuilder3::sensor_and_pool) to specify the [sensor](Sensor) and [pool](Pool) and obtain a [FuzzerBuilder4].
///
/// Alternatively, use [`self.default_sensor_and_pool(..)`](FuzzerBuilder3::default_sensor_and_pool) to use fuzzcheck’s
/// default sensor, which monitors code coverage.
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

/// A fuzz-test builder that knows the function to fuzz-test, the mutator, the serializer, the sensor, and the pool.
///
/// Use [`self.arguments(..)`] to specify the [arguments](Arguments) and obtain a [`FuzzerBuilder5`].
///
/// If you are using the `cargo-fuzzcheck` command line tool (and you should), use
/// [`self.arguments_from_cargo_fuzzcheck()`](FuzzerBuilder4::arguments_from_cargo_fuzzcheck)
/// to use the arguments specified by this tool, which is easier.
pub struct FuzzerBuilder4<F, M, V, Sens, P>
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

/// A fuzz-test builder that knows every necessary detail to start fuzzing.
///
/// Use [`self.launch()`](FuzzerBuilder5::launch) to start fuzzing.
///
/// You can also override some arguments using:
/// * [`self.command(..)`](FuzzerBuilder5::command)
/// * [`self.in_corpus(..)`](FuzzerBuilder5::in_corpus)
/// * [`self.out_corpus(..)`](FuzzerBuilder5::out_corpus)
/// * [`self.artifacts_folder(..)`](FuzzerBuilder5::artifacts_folder)
/// * [`self.maximum_complexity(..)`](FuzzerBuilder5::maximum_complexity)
/// * [`self.stop_after_iterations(..)`](FuzzerBuilder5::stop_after_iterations)
/// * [`self.stop_after_duration(..)`](FuzzerBuilder5::stop_after_duration)
/// * [`self.stop_after_first_test_failure(..)`](FuzzerBuilder5::stop_after_first_test_failure)
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
    arguments: Arguments,
    _phantom: PhantomData<*const V>,
}

/**
    Build a fuzz test for the given function!

    The returned value is a [`FuzzerBuilder1`]. See the [module/crate documentation](crate::builder)
    for a full example of how to build a fuzz test.

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
    /// Use the default mutator, serializer, sensor, pool, and arguments.
    #[no_coverage]
    pub fn default_options(
        self,
    ) -> FuzzerBuilder5<
        F::NormalizedFunction,
        <T::Owned as DefaultMutator>::Mutator,
        T::Owned,
        CodeCoverageSensor,
        impl Pool + CompatibleWithSensor<CodeCoverageSensor>,
    > {
        self.mutator(<T::Owned as DefaultMutator>::default_mutator())
            .serializer(SerdeSerializer::default())
            .default_sensor_and_pool()
            .arguments_from_cargo_fuzzcheck()
    }
}

impl<T, F> FuzzerBuilder1<T, F>
where
    T: ?Sized + ToOwned + 'static,
    T::Owned: Clone + DefaultMutator,
    <T::Owned as DefaultMutator>::Mutator: 'static,
    F: Fn(&T) -> bool,
    F: FuzzTestFunction<T::Owned, T, ReturnBool>,
{
    /// Use the [`DefaultMutator`] trait to specify the mutator that produces input values for the tested function.
    #[no_coverage]
    pub fn default_mutator(
        self,
    ) -> FuzzerBuilder2<F::NormalizedFunction, <T::Owned as DefaultMutator>::Mutator, T::Owned> {
        self.mutator(<T::Owned as DefaultMutator>::default_mutator())
    }
}
impl<T, F> FuzzerBuilder1<T, F>
where
    T: ?Sized,
    F: Fn(&T) -> bool,
{
    /**
        Specify the mutator that produces input values for the tested function.

        For example, if the test function is:
        ```
        fn foo(xs: &[u8]) {
            // ..
        }
        ```
        Then the given mutator should produces values that can be borrowed as `[u8]`.
        We can write:
        ```
        use fuzzcheck::DefaultMutator;
        use fuzzcheck::mutators::vector::VecMutator;
        fn foo(xs: &[u8]) {
            // ..
        }
        fn fuzz_test() {
            fuzzcheck::fuzz_test(foo)
                .mutator(VecMutator::new(u8::default_mutator(), 2 ..= 10))
                // ..
                # ;
        }
        ```
        Alternatively, if you would like to use the argument type’s [default mutator](DefaultMutator), you can use
        [`.default_mutator()`](FuzzerBuilder1::default_mutator), as follows:
        ```
        use fuzzcheck::DefaultMutator;
        fn foo(xs: &[u8]) {
            // ..
        }
        fn fuzz_test() {
            fuzzcheck::fuzz_test(foo)
                .default_mutator()
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

        The serializer must implement the [`Serializer`](crate::Serializer) trait. If you wish
        to use `serde`, you can use [`.serde_serializer()`](FuzzerBuilder2::serde_serializer) as follows:
        ```
        # use fuzzcheck::DefaultMutator;
        # fn foo(x: &bool) {}
        fuzzcheck::fuzz_test(foo)
            .mutator(
                # bool::default_mutator()
                /* .. */
            )
            .serde_serializer()
            # ;
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

impl<F, M, V> FuzzerBuilder2<F, M, V>
where
    F: Fn(&V) -> bool,
    V: Clone + serde::Serialize + for<'e> serde::Deserialize<'e> + 'static,
    M: Mutator<V>,
{
    /// Specify [`SerdeSerializer`] as the serializer to use when saving the interesting test cases to the file system.
    #[no_coverage]
    pub fn serde_serializer(self) -> FuzzerBuilder3<F, M, V> {
        FuzzerBuilder3 {
            test_function: self.test_function,
            mutator: self.mutator,
            serializer: Box::new(SerdeSerializer::<V>::default()),
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
    pub fn default_sensor_and_pool(
        self,
    ) -> FuzzerBuilder4<F, M, V, CodeCoverageSensor, impl CompatibleWithSensor<CodeCoverageSensor>> {
        let (sensor, pool) = default_sensor_and_pool().finish();
        FuzzerBuilder4 {
            test_function: self.test_function,
            mutator: self.mutator,
            serializer: self.serializer,
            sensor,
            pool,
            _phantom: PhantomData,
        }
    }
    #[no_coverage]
    pub fn sensor_and_pool<Sens: Sensor, P: CompatibleWithSensor<Sens>>(
        self,
        sensor: Sens,
        pool: P,
    ) -> FuzzerBuilder4<F, M, V, Sens, P> {
        FuzzerBuilder4 {
            test_function: self.test_function,
            mutator: self.mutator,
            serializer: self.serializer,
            sensor,
            pool,
            _phantom: PhantomData,
        }
    }
}

impl<F, M, V, Sens, P> FuzzerBuilder4<F, M, V, Sens, P>
where
    F: Fn(&V) -> bool,
    V: Clone,
    M: Mutator<V>,
    Sens: Sensor,
    P: Pool + CompatibleWithSensor<Sens>,
{
    #[no_coverage]
    pub fn arguments(self, arguments: Arguments) -> FuzzerBuilder5<F, M, V, Sens, P> {
        FuzzerBuilder5 {
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
    pub fn arguments_from_cargo_fuzzcheck(self) -> FuzzerBuilder5<F, M, V, Sens, P> {
        let parser = options_parser();
        let mut help = format!(
            r#""
fuzzcheck <SUBCOMMAND> [OPTIONS]

SUBCOMMANDS:
    {fuzz}    Run the fuzz test
    {minify}    Minify a crashing test input, requires --{input_file}
"#,
            fuzz = COMMAND_FUZZ,
            minify = COMMAND_MINIFY_INPUT,
            input_file = INPUT_FILE_FLAG,
        );
        help += parser.usage("").as_str();
        help += format!(
            r#""
## Examples:

fuzzcheck {fuzz}
    Launch the fuzzer with default options.

fuzzcheck {minify} --{input_file} "artifacts/crash.json"

    Minify the test input defined in the file "artifacts/crash.json".
    It will put minified inputs in the folder artifacts/crash.minified/
    and name them {{complexity}}-{{hash}}.json. 
    For example, artifacts/crash.minified/4213--8cd7777109b57b8c.json
    is a minified input of complexity 42.13.
"#,
            fuzz = COMMAND_FUZZ,
            minify = COMMAND_MINIFY_INPUT,
            input_file = INPUT_FILE_FLAG,
        )
        .as_str();

        let arguments = std::env::var("FUZZCHECK_ARGS").unwrap();
        let arguments = split_string_by_whitespace(&arguments);
        let matches = parser.parse(arguments).map_err(|e| ArgumentsError::from(e));
        let arguments = match matches.and_then(|matches| Arguments::from_matches(&matches, false)) {
            Ok(r) => r,
            Err(e) => {
                println!("{}\n\n{}", e, help);
                std::process::exit(1);
            }
        };
        FuzzerBuilder5 {
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

impl<F, M, V, Sens, P> FuzzerBuilder5<F, M, V, Sens, P>
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
    pub fn launch(self) -> FuzzingResult<V> {
        #[cfg(fuzzing)]
        return self.launch_even_if_cfg_fuzzing_is_not_set();
        #[cfg(not(fuzzing))]
        return FuzzingResult {
            found_test_failure: true,
            reason_for_stopping: crate::fuzzer::ReasonForStopping::LaunchedFuzzcheckWithoutCfgFuzzing,
        };
    }

    /// do not use
    #[no_coverage]
    pub fn launch_even_if_cfg_fuzzing_is_not_set(self) -> FuzzingResult<V> {
        let FuzzerBuilder5 {
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

/// An alias for the basic pool chosen to handle the [`CodeCoverageSensor`]'s observations
pub type BasicPool = SimplestToActivateCounterPool;
/// An alias for the type of the pool which tries to find a fixed-length set of test cases which, together, activate the most counters.
pub type DiversePool = AndPool<MostNDiversePool, OptimiseAggregateStatPool<NumberOfActivatedCounters>>;
/// An alias for the type of the pool which tries to find test cases repeatedly hitting the same counters.
pub type MaxHitsPool = AndPool<MaximiseCounterValuePool, OptimiseAggregateStatPool<SumOfCounterValues>>;

/// An alias for the combination of the [`BasicPool`] and the [`DiversePool`]
pub type BasicAndDiversePool = AndPool<SimplestToActivateCounterPool, DiversePool>;
/// An alias for the combination of the [`BasicPool`] and the [`MaxHitsPool`]
pub type BasicAndMaxHitsPool = AndPool<SimplestToActivateCounterPool, MaxHitsPool>;
/// An alias for the combination of the [`BasicPool`], the [`DiversePool`], and the [`MaxHitsPool`]
pub type BasicAndDiverseAndMaxHitsPool = AndPool<SimplestToActivateCounterPool, AndPool<DiversePool, MaxHitsPool>>;

#[no_coverage]
pub fn max_cov_hits_sensor_and_pool() -> CodeCoverageSensorAndPoolBuilder<MaxHitsPool> {
    let sensor = CodeCoverageSensor::observing_only_files_from_current_dir();
    let nbr_counters = sensor.count_instrumented;
    CodeCoverageSensorAndPoolBuilder {
        sensor,
        pool: AndPool::new(
            MaximiseCounterValuePool::new("max_each_cov_hits", nbr_counters),
            OptimiseAggregateStatPool::<SumOfCounterValues>::new("max_total_cov_hits"),
            192, // choose max_each_cov_hits ~75% of the time
        ),
    }
}

/// Create the initial [sensor and pool builder](CodeCoverageSensorAndPoolBuilder)
///
/// Use [`.find_most_diverse_set_of_test_cases()`](CodeCoverageSensorAndPoolBuilder::<BasicPool>::find_most_diverse_set_of_test_cases)
/// or [`.find_test_cases_repeatedly_hitting_coverage_counters()`](CodeCoverageSensorAndPoolBuilder::<BasicPool>::find_test_cases_repeatedly_hitting_coverage_counters)
/// on the result to augment the pool. Or use [`.finish()`](CodeCoverageSensorAndPoolBuilder::finish) to obtain the concrete sensor and pool.
#[no_coverage]
pub fn basic_sensor_and_pool() -> CodeCoverageSensorAndPoolBuilder<BasicPool> {
    let sensor = CodeCoverageSensor::observing_only_files_from_current_dir();
    let nbr_counters = sensor.count_instrumented;
    CodeCoverageSensorAndPoolBuilder {
        sensor,
        pool: SimplestToActivateCounterPool::new("simplest_cov", nbr_counters),
    }
}

/// Create the [sensor and pool builder](CodeCoverageSensorAndPoolBuilder) that is used by default by fuzzcheck
///
/// Currently, the result cannot be augmented any further. Thus, the only action you can take on the result is to
/// use [`.finish()`](CodeCoverageSensorAndPoolBuilder::finish) to obtain the concrete sensor and pool.
#[no_coverage]
pub fn default_sensor_and_pool() -> CodeCoverageSensorAndPoolBuilder<BasicAndDiverseAndMaxHitsPool> {
    basic_sensor_and_pool()
        .find_most_diverse_set_of_test_cases(20)
        .find_test_cases_repeatedly_hitting_coverage_counters()
}
/// A builder to create a [sensor](Sensor) and [pool](Pool) that can be given as argument to
/// [`FuzzerBuilder3::sensor_and_pool`].
///
/// # Usage
/// ```no_run
/// use fuzzcheck::builder::basic_sensor_and_pool;
///
/// let (sensor, pool) = basic_sensor_and_pool()
///     .find_most_diverse_set_of_test_cases(10) // optional
///     .find_test_cases_repeatedly_hitting_coverage_counters() // optional
///     .finish(); // mandatory
/// ```
pub struct CodeCoverageSensorAndPoolBuilder<P>
where
    P: CompatibleWithSensor<CodeCoverageSensor>,
{
    sensor: CodeCoverageSensor,
    pool: P,
}

impl<P> CodeCoverageSensorAndPoolBuilder<P>
where
    P: CompatibleWithSensor<CodeCoverageSensor>,
{
    /// Obtain the sensor and pool from the builder
    #[no_coverage]
    pub fn finish(self) -> (CodeCoverageSensor, impl CompatibleWithSensor<CodeCoverageSensor>) {
        (self.sensor, self.pool)
    }
}

impl CodeCoverageSensorAndPoolBuilder<BasicPool> {
    /// Augment the current pool such that it also tries to find a fixed-length set of test cases which, together,
    /// trigger the most code coverage.
    ///
    /// ### Argument
    /// `size` : the size of the set of test cases to find
    #[no_coverage]
    pub fn find_most_diverse_set_of_test_cases(
        self,
        size: usize,
    ) -> CodeCoverageSensorAndPoolBuilder<BasicAndDiversePool> {
        let nbr_counters = self.sensor.count_instrumented;
        CodeCoverageSensorAndPoolBuilder {
            sensor: self.sensor,
            pool: AndPool::new(
                self.pool,
                AndPool::new(
                    MostNDiversePool::new(&format!("diverse_cov_{}", size), size, nbr_counters),
                    OptimiseAggregateStatPool::<NumberOfActivatedCounters>::new("diverse_cov_1"),
                    192, // choose most n diverse ~75% of the time
                ),
                192, // 75% if the time
            ),
        }
    }
    /// Augment the current pool such that it also tries to find test cases repeatedly hitting the same regions of code.
    #[no_coverage]
    pub fn find_test_cases_repeatedly_hitting_coverage_counters(
        self,
    ) -> CodeCoverageSensorAndPoolBuilder<BasicAndMaxHitsPool> {
        let nbr_counters = self.sensor.count_instrumented;
        CodeCoverageSensorAndPoolBuilder {
            sensor: self.sensor,
            pool: AndPool::new(
                self.pool,
                AndPool::new(
                    MaximiseCounterValuePool::new("max_each_cov_hits", nbr_counters),
                    OptimiseAggregateStatPool::<SumOfCounterValues>::new("max_total_cov_hits"),
                    192, // choose max_each_cov_hits ~75% of the time
                ),
                217, // ~85% of the time
            ),
        }
    }
}
impl CodeCoverageSensorAndPoolBuilder<BasicAndDiversePool> {
    /// Augment the current pool such that it also tries to find test cases repeatedly hitting the same regions of code.
    #[no_coverage]
    pub fn find_test_cases_repeatedly_hitting_coverage_counters(
        self,
    ) -> CodeCoverageSensorAndPoolBuilder<BasicAndDiverseAndMaxHitsPool> {
        let nbr_counters = self.sensor.count_instrumented;
        CodeCoverageSensorAndPoolBuilder {
            sensor: self.sensor,
            pool: AndPool::new(
                self.pool.p1, // smallest to activate counter pool
                AndPool::new(
                    self.pool.p2, // diverse pool
                    AndPool::new(
                        // in the end, the max_hits pool is chosen ~10% of the time
                        MaximiseCounterValuePool::new("max_each_cov_hits", nbr_counters),
                        OptimiseAggregateStatPool::<SumOfCounterValues>::new("max_total_cov_hits"),
                        192, // choose max_each_cov_hits ~75% of the time
                    ),
                    183, // choose diverse pool 71% of the time => in the end, chosen (100% - 65%) * 75% = ~25% of the time
                ),
                166, // 65% of the time
            ),
        }
    }
}
impl CodeCoverageSensorAndPoolBuilder<BasicAndMaxHitsPool> {
    /// Augment the current pool such that it also tries to find a fixed-length set of test cases which, together,
    /// trigger the most code coverage.
    ///
    /// ### Argument
    /// `size` : the size of the set of test cases to find
    #[no_coverage]
    pub fn find_most_diverse_set_of_test_cases(
        self,
        size: usize,
    ) -> CodeCoverageSensorAndPoolBuilder<BasicAndDiverseAndMaxHitsPool> {
        let nbr_counters = self.sensor.count_instrumented;
        CodeCoverageSensorAndPoolBuilder {
            sensor: self.sensor,
            pool: AndPool::new(
                self.pool.p1, // smallest to activate counter pool
                AndPool::new(
                    AndPool::new(
                        MostNDiversePool::new(&format!("diverse_cov_{}", size), size, nbr_counters),
                        OptimiseAggregateStatPool::<NumberOfActivatedCounters>::new("diverse_cov1"),
                        192, // choose most n diverse ~75% of the time
                    ),
                    self.pool.p2, // diverse pool
                    183, // choose diverse pool 71% of the time => in the end, chosen (100% - 65%) * 75% = ~25% of the time
                ),
                166, // 65% of the time
            ),
        }
    }
}
