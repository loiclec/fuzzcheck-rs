use std::any::{Any, TypeId};
use std::borrow::Borrow;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::process::exit;
use std::result::Result;

use fuzzcheck_common::arg::{Arguments, FuzzerCommand};
use fuzzcheck_common::{FuzzerEvent, FuzzerStats};
use libc::{SIGABRT, SIGALRM, SIGBUS, SIGFPE, SIGINT, SIGSEGV, SIGTERM, SIGTRAP};

use crate::data_structures::RcSlab;
use crate::sensors_and_pools::{
    AndSensorAndPool, NoopSensor, TestFailure, TestFailurePool, TestFailureSensor, UnitPool, TEST_FAILURE,
};
use crate::signals_handler::set_signal_handlers;
use crate::subvalue_provider::{CrossoverSubValueProvider, Generation, SubValueProviderId};
use crate::traits::{CorpusDelta, Mutator, SaveToStatsFolder, SensorAndPool, Serializer};
use crate::world::World;
use crate::{CSVField, SubValueProvider, ToCSV};

static WRITE_STATS_ERROR: &str = "the stats could not be written to the file system";
static WORLD_NEW_ERROR: &str = "an IO operation failed when setting up the fuzzer";
static SERIALIZER_FROM_DATA_ERROR: &str = "the file could not be decoded into a valid input";
static READ_INPUT_FILE_ERROR: &str = "the input file could not be read";
static SAVE_ARTIFACTS_ERROR: &str = "the artifact could not be saved";
static UPDATE_CORPUS_ERROR: &str = "the corpus could not be updated on the file system";

static mut DID_FIND_ANY_TEST_FAILURE: bool = false;

/// The result of a fuzz test, if it ends.
///
/// It contains two fields:
/// 1. [`found_test_failure`](Self::found_test_failure) is `true` if the fuzzer found any failing test case
/// 2. [`reason_for_stopping`](Self::reason_for_stopping) gives the reason why the fuzzer stopped.
///
/// If the fuzzer stopped because it found a failing test case, then `reason_for_stopping` has the
/// value [`ReasonForStopping::TestFailure(T)`](crate::ReasonForStopping::TestFailure) where `T` is the
/// failing test case.
#[derive(Debug, Clone)]
pub struct FuzzingResult<T> {
    pub found_test_failure: bool,
    pub reason_for_stopping: ReasonForStopping<T>,
}

#[derive(Debug, Clone)]
pub enum ReasonForStopping<T> {
    TestFailure(T),
    ExhaustedAllPossibleMutations,
    MaxIterationsReached,
    MaxDurationReached,
}

/// The index to a test case in the fuzzer’s storage.
#[cfg_attr(feature = "serde_json_serializer", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PoolStorageIndex(usize);

// #[cfg(test)]
impl PoolStorageIndex {
    #[coverage(off)]
    pub fn mock(idx: usize) -> Self {
        Self(idx)
    }
}

enum FuzzerInputIndex<T> {
    None,
    Temporary(T),
    Pool(PoolStorageIndex),
}

struct FuzzedInputAndSubValueProvider<T, M>
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    input: FuzzedInput<T, M>,
    subvalues: CrossoverSubValueProvider<T, M>,
}

/**
 * A struct that stores the value, cache, and mutation step of an input.
 * It is used for convenience.
 */
struct FuzzedInput<T: Clone + 'static, Mut: Mutator<T>> {
    value: T,
    cache: Mut::Cache,
    mutation_step: Mut::MutationStep,
    generation: Generation,
}
impl<T: Clone + 'static, Mut: Mutator<T>> Clone for FuzzedInput<T, Mut> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            cache: self.cache.clone(),
            mutation_step: self.mutation_step.clone(),
            generation: self.generation,
        }
    }
}

impl<T: Clone + 'static, Mut: Mutator<T>> FuzzedInput<T, Mut> {
    #[coverage(off)]
    fn new(value: T, cache: Mut::Cache, mutation_step: Mut::MutationStep, generation: Generation) -> Self {
        Self {
            value,
            cache,
            mutation_step,
            generation,
        }
    }

    #[coverage(off)]
    fn new_source(&self, m: &Mut, generation: Generation) -> Self {
        let cache = m.validate_value(&self.value).unwrap();
        let mutation_step = m.default_mutation_step(&self.value, &cache);
        Self::new(self.value.clone(), cache, mutation_step, generation)
    }

    #[coverage(off)]
    fn complexity(&self, m: &Mut) -> f64 {
        m.complexity(&self.value, &self.cache)
    }

    #[coverage(off)]
    fn mutate(
        &mut self,
        m: &Mut,
        subvalue_provider: &dyn SubValueProvider,
        max_cplx: f64,
    ) -> Option<(Mut::UnmutateToken, f64)> {
        m.ordered_mutate(
            &mut self.value,
            &mut self.cache,
            &mut self.mutation_step,
            subvalue_provider,
            max_cplx,
        )
    }

    #[coverage(off)]
    fn unmutate(&mut self, m: &Mut, t: Mut::UnmutateToken) {
        m.unmutate(&mut self.value, &mut self.cache, t);
    }
}

struct FuzzerState<T: Clone + 'static, M: Mutator<T>> {
    mutator: M,
    sensor_and_pool: Box<dyn SensorAndPool>,
    pool_storage: RcSlab<FuzzedInputAndSubValueProvider<T, M>>,
    /// The step given to the mutator when the fuzzer wants to create a new arbitrary test case
    arbitrary_step: M::ArbitraryStep,
    /// The index of the test case that is being tested
    input_idx: FuzzerInputIndex<FuzzedInput<T, M>>,
    /// Various statistics about the fuzzer run
    fuzzer_stats: FuzzerStats,

    settings: Arguments,
    serializer: Box<dyn Serializer<Value = T>>,
    /// The world handles effects
    world: World,
    rng: fastrand::Rng,

    signal_handler_alt_stack: Option<(*mut u8, std::alloc::Layout)>,
}

impl<T: Clone + 'static, M: Mutator<T>> Drop for FuzzerState<T, M> {
    #[coverage(off)]
    fn drop(&mut self) {
        unsafe {
            crate::signals_handler::reset_signal_handlers();
            if let Some((stack_ptr, stack_layout)) = self.signal_handler_alt_stack {
                std::alloc::dealloc(stack_ptr, stack_layout);
            }
        }
    }
}

impl<T: Clone + 'static, M: Mutator<T>> FuzzerState<T, M> {
    #[coverage(off)]
    fn get_input<'a>(
        fuzzer_input_idx: &'a FuzzerInputIndex<FuzzedInput<T, M>>,
        pool_storage: &'a RcSlab<FuzzedInputAndSubValueProvider<T, M>>,
    ) -> Option<&'a FuzzedInput<T, M>> {
        match fuzzer_input_idx {
            FuzzerInputIndex::None => None,
            FuzzerInputIndex::Temporary(input) => Some(input),
            FuzzerInputIndex::Pool(idx) => Some(&pool_storage[idx.0].input),
        }
    }
}

#[coverage(off)]
fn update_fuzzer_stats(stats: &mut FuzzerStats, world: &mut World) {
    let microseconds = world.elapsed_time_since_last_checkpoint();
    let nbr_runs = stats.total_number_of_runs - stats.number_of_runs_since_last_reset_time;
    let nbr_runs_times_million = nbr_runs * 1_000_000;
    if microseconds != 0 {
        stats.exec_per_s = nbr_runs_times_million / microseconds;
    }
    if microseconds > 1_000_000 {
        world.set_checkpoint_instant();
        stats.number_of_runs_since_last_reset_time = stats.total_number_of_runs;
    }
}

impl<T: Clone + 'static, M: Mutator<T>> SaveToStatsFolder for FuzzerState<T, M>
where
    Self: 'static,
{
    #[coverage(off)]
    fn save_to_stats_folder(&self) -> Vec<(std::path::PathBuf, Vec<u8>)> {
        let mut contents = self.sensor_and_pool.save_to_stats_folder();
        contents.extend(self.world.save_to_stats_folder());
        contents
    }
}

impl<T: Clone + 'static, M: Mutator<T>> FuzzerState<T, M>
where
    Self: 'static,
{
    #[coverage(off)]
    fn write_stats(&mut self) -> Result<(), std::io::Error> {
        self.world.write_stats_content(self.save_to_stats_folder())
    }

    #[coverage(off)]
    fn receive_signal(&mut self, signal: i32) -> ! {
        self.world.report_event(
            FuzzerEvent::CaughtSignal(signal as i32),
            Some((&self.fuzzer_stats, self.sensor_and_pool.stats().as_ref())),
        );

        match signal {
            SIGABRT | SIGBUS | SIGSEGV | SIGFPE | SIGALRM | SIGTRAP => {
                if let Some(input) = Self::get_input(&self.input_idx, &self.pool_storage) {
                    let input = input.new_source(&self.mutator, Generation(0));
                    let cplx = input.complexity(&self.mutator);
                    let content = self.serializer.to_data(&input.value);
                    let _ = self.world.save_artifact(content, cplx, self.serializer.extension());
                    self.write_stats().expect(WRITE_STATS_ERROR);
                    exit(TerminationStatus::Crash as i32);
                } else {
                    self.world.report_event(
                        FuzzerEvent::CrashNoInput,
                        Some((&self.fuzzer_stats, self.sensor_and_pool.stats().as_ref())),
                    );
                    exit(TerminationStatus::Crash as i32);
                }
            }
            SIGINT | SIGTERM => {
                self.write_stats().expect(WRITE_STATS_ERROR);
                self.world.stop()
            }
            _ => exit(TerminationStatus::Unknown as i32),
        }
    }
    #[coverage(off)]
    fn arbitrary_input(&mut self) -> Option<(FuzzedInput<T, M>, f64)> {
        if let Some((v, cplx)) = self
            .mutator
            .ordered_arbitrary(&mut self.arbitrary_step, self.settings.max_input_cplx)
        {
            let cache = self.mutator.validate_value(&v).unwrap();
            let step = self.mutator.default_mutation_step(&v, &cache);
            Some((FuzzedInput::new(v, cache, step, Generation(0)), cplx))
        } else {
            None
        }
    }
    #[coverage(off)]
    unsafe fn set_up_signal_handler(&mut self) {
        let ptr = self as *mut Self;
        let (stack_ptr, stack_size) = set_signal_handlers(
            #[coverage(off)]
            move |sig| (*ptr).receive_signal(sig),
        );
        self.signal_handler_alt_stack = Some((stack_ptr, stack_size));
    }
}

pub struct Fuzzer<T, M>
where
    T: Clone + 'static,
    M: Mutator<T>,
    Self: 'static,
{
    state: FuzzerState<T, M>,
    test: Box<dyn Fn(&T) -> bool>,
}

impl<T, M> Fuzzer<T, M>
where
    T: Clone + 'static,
    M: Mutator<T>,
    Self: 'static,
{
    #[coverage(off)]
    fn new(
        test: Box<dyn Fn(&T) -> bool>,
        mutator: M,
        serializer: Box<dyn Serializer<Value = T>>,
        sensor_and_pool: Box<dyn SensorAndPool>,
        settings: Arguments,
        world: World,
    ) -> Self {
        let arbitrary_step = mutator.default_arbitrary_step();
        Fuzzer {
            state: FuzzerState {
                sensor_and_pool,
                pool_storage: RcSlab::new(),
                mutator,
                arbitrary_step,
                input_idx: FuzzerInputIndex::None,
                fuzzer_stats: FuzzerStats::default(),
                settings,
                serializer,
                world,
                rng: fastrand::Rng::new(),
                signal_handler_alt_stack: None,
            },
            test,
        }
    }

    #[coverage(off)]
    fn test_and_process_input(&mut self, cplx: f64) -> Result<(), ReasonForStopping<T>> {
        let Fuzzer {
            state:
                FuzzerState {
                    mutator,
                    sensor_and_pool,
                    pool_storage,
                    input_idx,
                    fuzzer_stats,
                    serializer,
                    world,
                    settings,
                    ..
                },
            test,
            ..
        } = self;

        // we have verified in the caller function that there is an input
        let input = FuzzerState::<T, M>::get_input(input_idx, pool_storage).unwrap();

        std::panic::set_hook(Box::new(
            #[coverage(off)]
            move |panic_info| {
                let mut hasher = DefaultHasher::new();
                panic_info.location().hash(&mut hasher);
                unsafe {
                    TEST_FAILURE = Some(TestFailure {
                        display: format!("{}", panic_info),
                        id: hasher.finish(),
                    });
                }
            },
        ));
        if settings.detect_infinite_loop {
            let _old_time_left = unsafe { libc::alarm(1) };
            // TODO: I think setitimer should be prefered, but libc
            // doesn't support it on linux, see:
            // https://github.com/rust-lang/libc/issues/1347#event-3879031340

            // let success = unsafe {
            // let t = itimerval {
            //     it_interval: libc::timeval { tv_sec: 0, tv_usec: 0 },
            //     it_value: libc::timeval { tv_sec: 1, tv_usec: 0 },
            // };
            // libc::setitimer(ITIMER_REAL, &t, std::ptr::null_mut())
            // };
            // assert!(success == 0);
        }
        sensor_and_pool.start_recording();
        let result = catch_unwind(AssertUnwindSafe(
            #[coverage(off)]
            || (test)(input.value.borrow()),
        ));

        let _ = std::panic::take_hook();
        let test_failure = match result {
            Ok(false) => unsafe {
                TEST_FAILURE = Some(TestFailure {
                    display: "test function returned false".to_string(),
                    id: 0,
                });
                true
            },
            Err(_) => {
                // the panic handler already changed the value of TEST_FAILURE
                // so we don't need to do anything
                true
            }
            Ok(true) => false,
        };
        if test_failure {
            unsafe {
                DID_FIND_ANY_TEST_FAILURE = true;
            }
        }
        sensor_and_pool.stop_recording();
        if test_failure && self.state.settings.stop_after_first_failure {
            let serialized_input = serializer.to_data(&input.value);
            self.state
                .world
                .save_artifact(serialized_input, cplx, serializer.extension())
                .expect(SAVE_ARTIFACTS_ERROR);
            return Err(ReasonForStopping::TestFailure(input.value.clone()));
        }

        fuzzer_stats.total_number_of_runs += 1;

        let input_id = PoolStorageIndex(pool_storage.next_slot());

        let deltas = sensor_and_pool.process(input_id, cplx);

        if !deltas.is_empty() {
            let add_ref_count = deltas.iter().fold(
                0,
                #[coverage(off)]
                |acc, delta| if delta.add { acc + 1 } else { acc },
            );
            update_fuzzer_stats(fuzzer_stats, world);
            let event = CorpusDelta::fuzzer_event(&deltas);
            let content = if add_ref_count > 0 {
                serializer.to_data(&input.value)
            } else {
                vec![]
            };
            world
                .update_corpus(input_id, content, &deltas, serializer.extension())
                .expect(UPDATE_CORPUS_ERROR);
            world.report_event(event, Some((fuzzer_stats, sensor_and_pool.stats().as_ref())));
            if add_ref_count > 0 {
                let generation = Generation(fuzzer_stats.total_number_of_runs);
                let input = input.new_source(mutator, generation);
                // check that the mutator's handling of the complexity is correct
                let serialised = String::from_utf8(serializer.to_data(&input.value)).unwrap();
                assert!(
                    (input.complexity(mutator) - cplx).abs() < 0.01,
                    "The mutator used by the fuzz test does not evaluate the complexity of the test cases consistently.
                    This is a bug in the implementation of {}
                    =============
                    
                    {serialised}

                    =============
                    ",
                    std::any::type_name::<M>()
                );

                let mut subvalues: HashMap<TypeId, Vec<(*const dyn Any, f64)>> = HashMap::default();

                let mut act_on_subvalue = #[coverage(off)]
                |subvalue: &dyn Any, complexity| {
                    subvalues
                        .entry(subvalue.type_id())
                        .or_default()
                        .push((subvalue as *const _, complexity));
                };

                mutator.visit_subvalues(&input.value, &input.cache, &mut act_on_subvalue);
                let storage_idx_1 = pool_storage.next_slot();
                let subvalues = CrossoverSubValueProvider::new(
                    SubValueProviderId {
                        idx: storage_idx_1,
                        generation,
                    },
                    &input.value,
                    &input.cache,
                    mutator,
                );
                let stored_input = FuzzedInputAndSubValueProvider { input, subvalues };
                let storage_idx_2 = pool_storage.insert(stored_input, add_ref_count);
                assert_eq!(storage_idx_1, storage_idx_2);
            }
            for delta in deltas {
                for r in delta.remove {
                    pool_storage.remove(r.0);
                }
            }
        }

        Ok(())
    }

    #[coverage(off)]
    fn get_input_and_subvalue_provider<'a>(
        pool_storage: &'a mut RcSlab<FuzzedInputAndSubValueProvider<T, M>>,
        sensor_and_pool: &mut dyn SensorAndPool,
        rng: &fastrand::Rng,
        idx: PoolStorageIndex,
    ) -> (&'a mut FuzzedInput<T, M>, &'a (impl SubValueProvider + 'a)) {
        let idx_cross = sensor_and_pool.get_random_index().unwrap();

        if idx == idx_cross || rng.u8(..5) == 0 {
            let FuzzedInputAndSubValueProvider { input, subvalues } = &mut pool_storage[idx.0];
            (input, subvalues)
        } else {
            // crossover of two different test cases
            let (input, FuzzedInputAndSubValueProvider { subvalues, .. }) =
                pool_storage.get_mut_and_ref(idx.0, idx_cross.0).unwrap();
            (&mut input.input, subvalues)
        }
    }

    #[coverage(off)]
    fn process_next_input(&mut self) -> Result<(), ReasonForStopping<T>> {
        let FuzzerState {
            pool_storage,
            sensor_and_pool,
            input_idx,
            mutator,
            settings,
            rng,
            fuzzer_stats,
            world,
            ..
        } = &mut self.state;

        if let Some(idx) = sensor_and_pool.get_random_index() {
            *input_idx = FuzzerInputIndex::Pool(idx);
            let (input, subvalue_provider) =
                Self::get_input_and_subvalue_provider(pool_storage, sensor_and_pool.as_mut(), rng, idx);
            let generation = input.generation;
            if let Some((unmutate_token, complexity)) =
                input.mutate(mutator, subvalue_provider, settings.max_input_cplx)
            {
                //drop(subvalue_provider);
                if complexity < self.state.settings.max_input_cplx {
                    self.test_and_process_input(complexity)?;
                }

                // Retrieving the input may fail because the input may have been deleted
                if let Some(input) = self.state.pool_storage.get_mut(idx.0).map(
                    #[coverage(off)]
                    |x| &mut x.input,
                ) && input.generation == generation {
                    input.unmutate(&self.state.mutator, unmutate_token);
                }

                Ok(())
            } else {
                world.report_event(FuzzerEvent::End, Some((fuzzer_stats, sensor_and_pool.stats().as_ref())));
                Err(ReasonForStopping::ExhaustedAllPossibleMutations)
            }
        } else if let Some((input, cplx)) = self.state.arbitrary_input() {
            self.state.input_idx = FuzzerInputIndex::Temporary(input);

            if cplx < self.state.settings.max_input_cplx {
                self.test_and_process_input(cplx)?;
            }

            Ok(())
        } else {
            self.state.world.report_event(
                FuzzerEvent::End,
                Some((&self.state.fuzzer_stats, self.state.sensor_and_pool.stats().as_ref())),
            );
            Err(ReasonForStopping::ExhaustedAllPossibleMutations)
        }
    }

    #[coverage(off)]
    fn process_initial_inputs(&mut self) -> Result<(), ReasonForStopping<T>> {
        let mut inputs: Vec<FuzzedInput<T, M>> = self
            .state
            .world
            .read_input_corpus()
            .expect(READ_INPUT_FILE_ERROR)
            .into_iter()
            .filter_map(
                #[coverage(off)]
                |value| {
                    let value = self.state.serializer.from_data(&value)?;
                    let cache = self.state.mutator.validate_value(&value)?;
                    let mutation_step = self.state.mutator.default_mutation_step(&value, &cache);
                    Some(FuzzedInput::new(value, cache, mutation_step, Generation(0)))
                },
            )
            .collect();

        for _ in 0..100 {
            if let Some((input, _)) = self.state.arbitrary_input() {
                inputs.push(input);
            } else {
                break;
            }
        }
        inputs.retain(
            #[coverage(off)]
            |i| i.complexity(&self.state.mutator) <= self.state.settings.max_input_cplx,
        );
        // assert!(!inputs.is_empty());

        self.state.world.set_checkpoint_instant();
        for input in inputs {
            let cplx = input.complexity(&self.state.mutator);
            self.state.input_idx = FuzzerInputIndex::Temporary(input);
            self.test_and_process_input(cplx)?;
        }

        Ok(())
    }

    #[coverage(off)]
    fn main_loop(&mut self, minify: bool) -> Result<!, ReasonForStopping<T>> {
        self.state.world.report_event(
            FuzzerEvent::Start,
            Some((&self.state.fuzzer_stats, self.state.sensor_and_pool.stats().as_ref())),
        );
        if !minify {
            self.process_initial_inputs()?;
            self.state.world.report_event(
                FuzzerEvent::DidReadCorpus,
                Some((&self.state.fuzzer_stats, self.state.sensor_and_pool.stats().as_ref())),
            );
        }

        self.state.world.set_checkpoint_instant();
        let mut next_milestone = (self.state.fuzzer_stats.total_number_of_runs + 10) * 2;
        loop {
            let duration_since_beginning = self.state.world.elapsed_time_since_start();
            if duration_since_beginning > self.state.settings.maximum_duration {
                return Err(ReasonForStopping::MaxDurationReached);
            }
            if self.state.fuzzer_stats.total_number_of_runs >= self.state.settings.maximum_iterations {
                return Err(ReasonForStopping::MaxIterationsReached);
            }
            self.process_next_input()?;
            if self.state.fuzzer_stats.total_number_of_runs >= next_milestone {
                update_fuzzer_stats(&mut self.state.fuzzer_stats, &mut self.state.world);
                self.state.world.report_event(
                    FuzzerEvent::Pulse,
                    Some((&self.state.fuzzer_stats, self.state.sensor_and_pool.stats().as_ref())),
                );
                next_milestone = self.state.fuzzer_stats.total_number_of_runs * 2;
            }
        }
    }
}

pub enum TerminationStatus {
    Success = 0,
    Crash = 1,
    TestFailure = 2,
    Unknown = 3,
}

#[coverage(off)]
pub fn launch<T, M>(
    test: Box<dyn Fn(&T) -> bool>,
    mutator: M,
    serializer: Box<dyn Serializer<Value = T>>,
    sensor_and_pool: Box<dyn SensorAndPool>,
    mut args: Arguments,
) -> FuzzingResult<T>
where
    T: Clone + 'static,
    M: Mutator<T>,
    Fuzzer<T, M>: 'static,
{
    let command = &args.command;
    let reason_for_stopping = match command {
        FuzzerCommand::Fuzz => {
            if !args.stop_after_first_failure {
                let test_failure_sensor = TestFailureSensor::default();
                let test_failure_pool = TestFailurePool::new("test_failures");
                let sensor_and_pool = AndSensorAndPool::new(
                    sensor_and_pool,
                    Box::new((test_failure_sensor, test_failure_pool)),
                    10.0,
                    1.0,
                );
                let mut fuzzer = Fuzzer::new(
                    test,
                    mutator,
                    serializer,
                    Box::new(sensor_and_pool),
                    args.clone(),
                    World::new(args.clone()).expect(WORLD_NEW_ERROR),
                );

                let mut stats_headers = vec![CSVField::String("time".to_string())];
                stats_headers.extend(fuzzer.state.fuzzer_stats.csv_headers());
                stats_headers.extend(fuzzer.state.sensor_and_pool.stats().csv_headers());
                fuzzer
                    .state
                    .world
                    .append_stats_file(&stats_headers)
                    .expect(WRITE_STATS_ERROR);
                unsafe { fuzzer.state.set_up_signal_handler() };

                let reason_for_stopping = fuzzer.main_loop(false).unwrap_err();
                fuzzer.state.write_stats().expect(WRITE_STATS_ERROR);

                reason_for_stopping
            } else {
                let mut fuzzer = Fuzzer::new(
                    test,
                    mutator,
                    serializer,
                    sensor_and_pool,
                    args.clone(),
                    World::new(args.clone()).expect(WORLD_NEW_ERROR),
                );
                unsafe { fuzzer.state.set_up_signal_handler() };

                let mut stats_headers = vec![CSVField::String("time".to_string())];
                stats_headers.extend(fuzzer.state.fuzzer_stats.csv_headers());
                stats_headers.extend(fuzzer.state.sensor_and_pool.stats().csv_headers());
                fuzzer
                    .state
                    .world
                    .append_stats_file(&stats_headers)
                    .expect(WRITE_STATS_ERROR);
                let reason_for_stopping = fuzzer.main_loop(false).unwrap_err();
                fuzzer.state.write_stats().expect(WRITE_STATS_ERROR);

                reason_for_stopping
            }
        }
        FuzzerCommand::MinifyInput { input_file } => {
            let world = World::new(args.clone()).expect(WORLD_NEW_ERROR);
            let value = world.read_input_file(input_file).expect(READ_INPUT_FILE_ERROR);
            let value = serializer.from_data(&value).expect(SERIALIZER_FROM_DATA_ERROR);
            if let Some(cache) = mutator.validate_value(&value) {
                let mutation_step = mutator.default_mutation_step(&value, &cache);
                args.max_input_cplx = mutator.complexity(&value, &cache) - 0.01;

                let noop_sensor = NoopSensor;
                let unit_pool = UnitPool::new(PoolStorageIndex(0));
                let sensor_and_pool =
                    // 100:1 might seem like an excessive ratio, but the second pool will never make progress,
                    // therefore its relative weight willl diminish over time
                    // if after 100 iterations, the first pool makes progress, then the ratio will be 1:1
                    // what the exact value should be and how the ratio should evolve is an open question to me
                    AndSensorAndPool::new(sensor_and_pool, Box::new((noop_sensor, unit_pool)), 1.0, 100.0);
                let mut fuzzer = Fuzzer::new(
                    test,
                    mutator,
                    serializer,
                    Box::new(sensor_and_pool),
                    args.clone(),
                    world,
                );

                let mut subvalues: HashMap<TypeId, Vec<(*const dyn Any, f64)>> = HashMap::default();

                let mut act_on_subvalue = #[coverage(off)]
                |subvalue: &dyn Any, complexity| {
                    subvalues
                        .entry(subvalue.type_id())
                        .or_default()
                        .push((subvalue as *const _, complexity));
                };

                fuzzer
                    .state
                    .mutator
                    .visit_subvalues(&value, &cache, &mut act_on_subvalue);
                let storage_idx_1 = fuzzer.state.pool_storage.next_slot();
                let generation = Generation(0);
                let subvalues = CrossoverSubValueProvider::new(
                    SubValueProviderId {
                        idx: storage_idx_1,
                        generation,
                    },
                    &value,
                    &cache,
                    &fuzzer.state.mutator,
                );
                let stored_input = FuzzedInputAndSubValueProvider {
                    input: FuzzedInput::new(value, cache, mutation_step, generation),
                    subvalues,
                };
                let storage_idx_2 = fuzzer.state.pool_storage.insert(stored_input, 1);

                assert_eq!(storage_idx_1, storage_idx_2);

                unsafe { fuzzer.state.set_up_signal_handler() };

                fuzzer.main_loop(true).unwrap_err()
            } else {
                // TODO: send a better error message saying some inputs in the corpus cannot be read
                // TODO: there should be an option to ignore invalid values
                panic!("A value in the input corpus is invalid.");
            }
        }
        FuzzerCommand::Read { input_file } => {
            // no signal handlers are installed, but that should be ok as the exit code won't be 0
            let mut world = World::new(args.clone()).expect(WORLD_NEW_ERROR);
            let value = world.read_input_file(input_file).expect(READ_INPUT_FILE_ERROR);
            let value = serializer.from_data(&value).expect(SERIALIZER_FROM_DATA_ERROR);
            if let Some(cache) = mutator.validate_value(&value) {
                let mutation_step = mutator.default_mutation_step(&value, &cache);
                let input = FuzzedInput::new(value, cache, mutation_step, Generation(0));
                let cplx = input.complexity(&mutator);

                if args.detect_infinite_loop {
                    let _old_time_left = unsafe { libc::alarm(1) };
                    // TODO: I think setitimer should be prefered, but libc
                    // doesn't support it on linux, see:
                    // https://github.com/rust-lang/libc/issues/1347#event-3879031340

                    // let success = unsafe {
                    // let t = itimerval {
                    //     it_interval: libc::timeval { tv_sec: 0, tv_usec: 0 },
                    //     it_value: libc::timeval { tv_sec: 1, tv_usec: 0 },
                    // };
                    // libc::setitimer(ITIMER_REAL, &t, std::ptr::null_mut())
                    // };
                    // assert!(success == 0);
                }

                let result = catch_unwind(AssertUnwindSafe(
                    #[coverage(off)]
                    || (test)(input.value.borrow()),
                ));

                if result.is_err() || !result.unwrap() {
                    world.report_event(FuzzerEvent::TestFailure, None);
                    let content = serializer.to_data(&input.value);
                    world
                        .save_artifact(content, cplx, serializer.extension())
                        .expect(SAVE_ARTIFACTS_ERROR);
                    // in this case we really want to exit with a non-zero termination status here
                    // because the Read command is only used by the input minify command from cargo-fuzzcheck
                    // which checks that a crash happens by looking at the exit code
                    // so we don't want to handle any error
                    exit(TerminationStatus::TestFailure as i32);
                } else {
                    exit(TerminationStatus::Success as i32);
                }
            } else {
                // TODO: send a better error message saying some inputs in the corpus cannot be read
                panic!("A value in the input corpus is invalid.");
            }
        }
    };
    let _ = std::panic::take_hook();

    let found_test_failure =
        unsafe { matches!(reason_for_stopping, ReasonForStopping::TestFailure(_)) || DID_FIND_ANY_TEST_FAILURE };

    FuzzingResult {
        found_test_failure,
        reason_for_stopping,
    }
}
