//! Fuzzing engine. Connects the CodeCoverageSensor
//!to the [Pool] and uses an evolutionary algorithm using [Mutator] to find new interesting
//! test inputs.

use crate::code_coverage_sensor::CodeCoverageSensor;
use crate::data_structures::RcSlab;
use crate::sensors_and_pools::and_sensor_and_pool::{AndPool, AndSensor};
use crate::sensors_and_pools::artifacts_pool::{ArtifactsPool, TestFailure, TestFailureSensor, TEST_FAILURE};
use crate::sensors_and_pools::noop_sensor::NoopSensor;
use crate::sensors_and_pools::unique_coverage_pool::UniqueCoveragePool;
use crate::sensors_and_pools::unit_pool::UnitPool;
use crate::signals_handler::set_signal_handlers;
use crate::traits::{CompatibleWithSensor, CorpusDelta, EmptyStats, Pool, Sensor};
use crate::traits::{Mutator, Serializer};
use crate::world::World;
use crate::{CSVField, FuzzedInput, ToCSVFields};
use fuzzcheck_common::arg::{Arguments, FuzzerCommand};
use fuzzcheck_common::{FuzzerEvent, FuzzerStats};
use libc::{SIGABRT, SIGALRM, SIGBUS, SIGFPE, SIGINT, SIGSEGV, SIGTERM, SIGTRAP};
use std::borrow::Borrow;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::process::exit;
use std::result::Result;

#[derive(Debug)]
pub enum ReasonForStopping<T> {
    MaximumIterations,
    MaximumDuration,
    ExhaustedPossibleMutations,
    TestFailure(T),
    IOError(std::io::Error),
}

impl<T> From<std::io::Error> for ReasonForStopping<T> {
    #[no_coverage]
    fn from(e: std::io::Error) -> Self {
        Self::IOError(e)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct PoolStorageIndex(usize);

#[cfg(test)]
impl PoolStorageIndex {
    pub fn mock(idx: usize) -> Self {
        Self(idx)
    }
}

enum FuzzerInputIndex<T> {
    None,
    Temporary(T),
    Pool(PoolStorageIndex),
}

struct FuzzerState<T: Clone, M: Mutator<T>, Sens: Sensor, P: Pool>
where
    P: CompatibleWithSensor<Sens>,
{
    mutator: M,
    sensor: Sens,
    pool_storage: RcSlab<FuzzedInput<T, M>>,
    pool: P,
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
}

impl<T: Clone, M: Mutator<T>, Sens: Sensor, P: Pool> FuzzerState<T, M, Sens, P>
where
    P: CompatibleWithSensor<Sens>,
{
    #[no_coverage]
    fn get_input<'a>(
        fuzzer_input_idx: &'a FuzzerInputIndex<FuzzedInput<T, M>>,
        pool_storage: &'a RcSlab<FuzzedInput<T, M>>,
    ) -> Option<&'a FuzzedInput<T, M>> {
        match fuzzer_input_idx {
            FuzzerInputIndex::None => None,
            FuzzerInputIndex::Temporary(input) => Some(input),
            FuzzerInputIndex::Pool(idx) => Some(&pool_storage[idx.0]),
        }
    }
}

#[no_coverage]
fn update_fuzzer_stats(stats: &mut FuzzerStats, world: &mut World) {
    let microseconds = world.elapsed_time_since_last_checkpoint();
    let nbr_runs = stats.total_number_of_runs - stats.number_of_runs_since_last_reset_time;
    let nbr_runs_times_million = nbr_runs * 1_000_000;
    stats.exec_per_s = nbr_runs_times_million / microseconds;

    if microseconds > 1_000_000 {
        world.set_checkpoint_instant();
        stats.number_of_runs_since_last_reset_time = stats.total_number_of_runs;
    }
}

impl<T: Clone, M: Mutator<T>, Sens: Sensor, P: Pool> FuzzerState<T, M, Sens, P>
where
    P: CompatibleWithSensor<Sens>,
    Self: 'static,
{
    #[no_coverage]
    fn receive_signal(&mut self, signal: i32) -> ! {
        self.world.report_event(
            FuzzerEvent::CaughtSignal(signal as i32),
            Some((&self.fuzzer_stats, &self.pool.stats())),
        );

        match signal {
            SIGABRT | SIGBUS | SIGSEGV | SIGFPE | SIGALRM | SIGTRAP => {
                if let Some(input) = Self::get_input(&self.input_idx, &self.pool_storage) {
                    let cplx = input.complexity(&self.mutator);
                    let content = self.serializer.to_data(&input.value);
                    let _ = self.world.save_artifact(content, cplx, self.serializer.extension());

                    exit(TerminationStatus::Crash as i32);
                } else {
                    self.world.report_event(
                        FuzzerEvent::CrashNoInput,
                        Some((&self.fuzzer_stats, &self.pool.stats())),
                    );
                    exit(TerminationStatus::Crash as i32);
                }
            }
            SIGINT | SIGTERM => self.world.stop(),
            _ => exit(TerminationStatus::Unknown as i32),
        }
    }
    #[no_coverage]
    fn arbitrary_input(&mut self) -> Option<(FuzzedInput<T, M>, f64)> {
        if let Some((v, cplx)) = self
            .mutator
            .ordered_arbitrary(&mut self.arbitrary_step, self.settings.max_input_cplx)
        {
            let (cache, step) = self.mutator.validate_value(&v).unwrap();
            Some((FuzzedInput::new(v, cache, step, 0), cplx))
        } else {
            None
        }
    }
    #[no_coverage]
    unsafe fn set_up_signal_handler(&mut self) {
        let ptr = self as *mut Self;
        set_signal_handlers(
            #[no_coverage]
            move |sig| (&mut *ptr).receive_signal(sig),
        );
    }
}

pub struct Fuzzer<T, M, Sens, P>
where
    T: Clone,
    M: Mutator<T>,
    Sens: Sensor,
    P: Pool,
    P: CompatibleWithSensor<Sens>,
    Self: 'static,
{
    state: FuzzerState<T, M, Sens, P>,
    test: Box<dyn Fn(&T) -> bool>,
}

impl<T, M, Sens, P> Fuzzer<T, M, Sens, P>
where
    T: Clone,
    M: Mutator<T>,
    Sens: Sensor,
    P: Pool,
    P: CompatibleWithSensor<Sens>,
    Self: 'static,
{
    #[no_coverage]
    fn new(
        test: Box<dyn Fn(&T) -> bool>,
        mutator: M,
        serializer: Box<dyn Serializer<Value = T>>,
        sensor: Sens,
        pool: P,
        settings: Arguments,
        world: World,
    ) -> Self {
        let arbitrary_step = mutator.default_arbitrary_step();
        Fuzzer {
            state: FuzzerState {
                sensor,
                pool_storage: RcSlab::new(),
                pool,
                mutator,
                arbitrary_step,
                input_idx: FuzzerInputIndex::None,
                fuzzer_stats: FuzzerStats::default(),
                settings,
                serializer,
                world,
            },
            test,
        }
    }

    #[no_coverage]
    fn test_and_process_input(&mut self, cplx: f64) -> Result<(), ReasonForStopping<T>> {
        let Fuzzer {
            state:
                FuzzerState {
                    mutator,
                    sensor,
                    pool_storage,
                    pool,
                    input_idx,
                    fuzzer_stats,
                    serializer,
                    world,
                    ..
                },
            test,
            ..
        } = self;

        // we have verified in the caller function that there is an input
        let input = FuzzerState::<T, M, Sens, P>::get_input(input_idx, pool_storage).unwrap();

        std::panic::set_hook(Box::new(
            #[no_coverage]
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

        sensor.start_recording();
        let result = catch_unwind(AssertUnwindSafe(
            #[no_coverage]
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
        sensor.stop_recording();
        if test_failure && self.state.settings.stop_after_first_failure {
            let serialized_input = serializer.to_data(&input.value);
            self.state
                .world
                .save_artifact(serialized_input, cplx, serializer.extension())?;
            return Err(ReasonForStopping::TestFailure(input.value.clone()));
        }

        fuzzer_stats.total_number_of_runs += 1;

        let input_id = PoolStorageIndex(pool_storage.next_slot());

        let deltas = pool.process(input_id, sensor, cplx);

        if !deltas.is_empty() {
            let add_ref_count = deltas.iter().fold(
                0,
                #[no_coverage]
                |acc, delta| if delta.add { acc + 1 } else { acc },
            );
            update_fuzzer_stats(fuzzer_stats, world);
            let event = CorpusDelta::fuzzer_event(&deltas);
            let content = if add_ref_count > 0 {
                serializer.to_data(&input.value)
            } else {
                vec![]
            };
            world.update_corpus(input_id, content, &deltas, serializer.extension())?;
            world.report_event(event, Some((fuzzer_stats, &pool.stats())));
            if add_ref_count > 0 {
                let new_input = input.new_source(mutator);
                // here I don't check the complexity of the new input,
                // but because of the way mutators work (real possibility of
                // inconsistent complexities), then its complexity may be higher
                // than the maximum allowed one
                pool_storage.insert(new_input, add_ref_count);
            }
            for delta in deltas {
                for r in delta.remove {
                    pool_storage.remove(r.0);
                }
            }
        }

        Ok(())
    }

    #[no_coverage]
    fn process_next_input(&mut self) -> Result<(), ReasonForStopping<T>> {
        let FuzzerState {
            pool_storage,
            pool,
            input_idx,
            mutator,
            settings,
            ..
        } = &mut self.state;
        loop {
            if let Some(idx) = pool.get_random_index() {
                *input_idx = FuzzerInputIndex::Pool(idx);
                let input = &mut pool_storage[idx.0];
                let generation = input.generation;
                if let Some((unmutate_token, cplx)) = input.mutate(mutator, settings.max_input_cplx) {
                    if cplx < self.state.settings.max_input_cplx {
                        self.test_and_process_input(cplx)?;
                    }

                    // Retrieving the input may fail because the input may have been deleted
                    if let Some(input) = self.state.pool_storage.get_mut(idx.0) {
                        if input.generation == generation {
                            input.unmutate(&self.state.mutator, unmutate_token);
                        }
                    }

                    break Ok(());
                } else {
                    pool.mark_test_case_as_dead_end(idx);
                    continue;
                }
            } else if let Some((input, cplx)) = self.state.arbitrary_input() {
                self.state.input_idx = FuzzerInputIndex::Temporary(input);

                if cplx < self.state.settings.max_input_cplx {
                    self.test_and_process_input(cplx)?;
                }

                break Ok(());
            } else {
                self.state.world.report_event(
                    FuzzerEvent::End,
                    Some((&self.state.fuzzer_stats, &self.state.pool.stats())),
                );
                break Err(ReasonForStopping::ExhaustedPossibleMutations);
            }
        }
    }

    #[no_coverage]
    fn process_initial_inputs(&mut self) -> Result<(), ReasonForStopping<T>> {
        let mut inputs: Vec<FuzzedInput<T, M>> = self
            .state
            .world
            .read_input_corpus()?
            .into_iter()
            .filter_map(
                #[no_coverage]
                |value| {
                    let value = self.state.serializer.from_data(&value)?;
                    let (cache, mutation_step) = self.state.mutator.validate_value(&value)?;
                    Some(FuzzedInput::new(value, cache, mutation_step, 0))
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

        inputs.drain_filter(
            #[no_coverage]
            |i| i.complexity(&self.state.mutator) > self.state.settings.max_input_cplx,
        );
        assert!(!inputs.is_empty());

        self.state.world.set_checkpoint_instant();
        for input in inputs {
            let cplx = input.complexity(&self.state.mutator);
            self.state.input_idx = FuzzerInputIndex::Temporary(input);
            self.test_and_process_input(cplx)?;
        }

        Ok(())
    }

    #[no_coverage]
    fn main_loop(&mut self) -> Result<!, ReasonForStopping<T>> {
        self.state.world.report_event(
            FuzzerEvent::Start,
            Some((&self.state.fuzzer_stats, &self.state.pool.stats())),
        );
        self.process_initial_inputs()?;
        self.state.world.report_event(
            FuzzerEvent::DidReadCorpus,
            Some((&self.state.fuzzer_stats, &self.state.pool.stats())),
        );

        self.state.world.set_checkpoint_instant();
        let mut next_milestone = (self.state.fuzzer_stats.total_number_of_runs + 100_000) * 2;
        loop {
            let duration_since_beginning = self.state.world.elapsed_time_since_start();
            if duration_since_beginning > self.state.settings.maximum_duration {
                return Err(ReasonForStopping::MaximumDuration);
            }
            if self.state.fuzzer_stats.total_number_of_runs >= self.state.settings.maximum_iterations {
                return Err(ReasonForStopping::MaximumIterations);
            }
            self.process_next_input()?;
            if self.state.fuzzer_stats.total_number_of_runs >= next_milestone {
                update_fuzzer_stats(&mut self.state.fuzzer_stats, &mut self.state.world);
                self.state.world.report_event(
                    FuzzerEvent::Pulse,
                    Some((&self.state.fuzzer_stats, &self.state.pool.stats())),
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

#[no_coverage]
pub fn launch<T, M, Sens, P>(
    test: Box<dyn Fn(&T) -> bool>,
    mutator: M,
    serializer: Box<dyn Serializer<Value = T>>,
    sensor: Sens,
    pool: P,
    mut args: Arguments,
) -> Result<(), ReasonForStopping<T>>
where
    T: Clone,
    M: Mutator<T>,
    Fuzzer<T, M, CodeCoverageSensor, UniqueCoveragePool>: 'static,
    Sens: Sensor + 'static,
    P: Pool + CompatibleWithSensor<Sens> + 'static,
{
    let command = &args.command;
    let reason_for_stopping = match command {
        FuzzerCommand::Fuzz => {
            if !args.stop_after_first_failure {
                let test_failure = TestFailureSensor::default();
                let sensor = AndSensor {
                    s1: sensor,
                    s2: test_failure,
                };
                let artifacts_pool = ArtifactsPool::new("artifacts");
                let pool = AndPool::new(pool, artifacts_pool, 254);
                let mut fuzzer = Fuzzer::new(
                    test,
                    mutator,
                    serializer,
                    sensor,
                    pool,
                    args.clone(),
                    World::new(args.clone())?,
                );

                let mut stats_headers = vec![CSVField::String("time".to_string())];
                stats_headers.extend(fuzzer.state.fuzzer_stats.csv_headers());
                stats_headers.extend(fuzzer.state.pool.stats().csv_headers());
                fuzzer.state.world.append_stats_file(&stats_headers)?;
                unsafe { fuzzer.state.set_up_signal_handler() };

                let reason_for_stopping = fuzzer.main_loop().unwrap_err();
                if !matches!(reason_for_stopping, ReasonForStopping::IOError(_)) {
                    let mut contents = fuzzer.state.pool.serialized();
                    contents.extend(fuzzer.state.sensor.serialized());
                    contents.extend(fuzzer.state.world.serialized());
                    fuzzer.state.world.write_stats_content(contents)?;
                }
                reason_for_stopping
            } else {
                let mut fuzzer = Fuzzer::new(
                    test,
                    mutator,
                    serializer,
                    sensor,
                    pool,
                    args.clone(),
                    World::new(args.clone())?,
                );
                unsafe { fuzzer.state.set_up_signal_handler() };

                let mut stats_headers = vec![CSVField::String("time".to_string())];
                stats_headers.extend(fuzzer.state.fuzzer_stats.csv_headers());
                stats_headers.extend(fuzzer.state.pool.stats().csv_headers());
                fuzzer.state.world.append_stats_file(&stats_headers)?;
                let reason_for_stopping = fuzzer.main_loop().unwrap_err();
                if !matches!(reason_for_stopping, ReasonForStopping::IOError(_)) {
                    let mut contents = fuzzer.state.pool.serialized();
                    contents.extend(fuzzer.state.sensor.serialized());
                    contents.extend(fuzzer.state.world.serialized());
                    fuzzer.state.world.write_stats_content(contents)?;
                }
                reason_for_stopping
            }
        }
        FuzzerCommand::MinifyInput { input_file } => {
            let world = World::new(args.clone())?;
            let value = world.read_input_file(input_file)?;
            let value = serializer.from_data(&value).ok_or(std::io::Error::new(
                std::io::ErrorKind::Other,
                "The file could not be decoded into a valid input.",
            ))?;
            if let Some((cache, mutation_step)) = mutator.validate_value(&value) {
                args.max_input_cplx = mutator.complexity(&value, &cache) - 0.01;

                let sensor = AndSensor {
                    s1: sensor,
                    s2: NoopSensor,
                };

                let unit_pool = UnitPool::new(PoolStorageIndex(0));
                let pool = AndPool::new(pool, unit_pool, 240);
                let mut fuzzer = Fuzzer::new(test, mutator, serializer, sensor, pool, args.clone(), world);
                fuzzer
                    .state
                    .pool_storage
                    .insert(FuzzedInput::new(value, cache, mutation_step, 0), 1);

                unsafe { fuzzer.state.set_up_signal_handler() };

                fuzzer.main_loop().unwrap_err()
            } else {
                // TODO: send a better error message saying some inputs in the corpus cannot be read
                // TODO: there should be an option to ignore invalid values
                panic!("A value in the input corpus is invalid.");
            }
        }
        FuzzerCommand::Read { input_file } => {
            // no signal handlers are installed, but that should be ok as the exit code won't be 0
            let mut world = World::new(args.clone())?;
            let value = world.read_input_file(input_file)?;
            let value = serializer.from_data(&value).ok_or(std::io::Error::new(
                std::io::ErrorKind::Other,
                "The file could not be decoded into a valid input.",
            ))?;
            if let Some((cache, mutation_step)) = mutator.validate_value(&value) {
                let input = FuzzedInput::new(value, cache, mutation_step, 0);
                let cplx = input.complexity(&mutator);

                let result = catch_unwind(AssertUnwindSafe(
                    #[no_coverage]
                    || (test)(input.value.borrow()),
                ));

                if result.is_err() || !result.unwrap() {
                    world.report_event::<EmptyStats>(FuzzerEvent::TestFailure, None);
                    let content = serializer.to_data(&input.value);
                    world.save_artifact(content, cplx, serializer.extension())?;
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

    Err(reason_for_stopping)
}
