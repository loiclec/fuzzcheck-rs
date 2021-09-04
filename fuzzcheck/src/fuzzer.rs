//! Fuzzing engine. Connects the CodeCoverageSensor
//!to the [Pool] and uses an evolutionary algorithm using [Mutator] to find new interesting
//! test inputs.

use crate::code_coverage_sensor::CodeCoverageSensor;
use crate::mutators::either::Either;
use crate::sensors_and_pools::and_sensor_and_pool::{AndPool, AndSensor};
use crate::sensors_and_pools::artifacts_pool::{ArtifactsPool, TestFailure, TestFailureSensor, TEST_FAILURE};
use crate::sensors_and_pools::noop_sensor::NoopSensor;
use crate::sensors_and_pools::unique_coverage_pool::UniqueCoveragePool;
use crate::sensors_and_pools::unit_pool::UnitPool;
use crate::signals_handler::set_signal_handlers;
use crate::traits::{CompatibleWithSensor, EmptyStats, Pool, Sensor};
use crate::traits::{Mutator, Serializer};
use crate::world::World;
use crate::FuzzedInput;
use fuzzcheck_common::arg::{Arguments, FuzzerCommand};
use fuzzcheck_common::{FuzzerEvent, FuzzerStats};
use libc::{SIGABRT, SIGALRM, SIGBUS, SIGFPE, SIGINT, SIGSEGV, SIGTERM};
use std::borrow::Borrow;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::panic::{catch_unwind, RefUnwindSafe, UnwindSafe};
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
    fn from(e: std::io::Error) -> Self {
        Self::IOError(e)
    }
}

enum FuzzerInputIndex<T, PoolIndex> {
    None,
    Temporary(T),
    Pool(PoolIndex),
}

struct FuzzerState<T: Clone, M: Mutator<T>, S: Serializer<Value = T>, Sens: Sensor, P: Pool>
where
    P: CompatibleWithSensor<Sens>,
{
    mutator: M,
    sensor: Sens,
    pool: P,
    /// The step given to the mutator when the fuzzer wants to create a new arbitrary test case
    arbitrary_step: M::ArbitraryStep,
    /// The index of the test case that is being tested
    input_idx: FuzzerInputIndex<FuzzedInput<T, M>, P::Index>,
    /// Various statistics about the fuzzer run
    fuzzer_stats: FuzzerStats,

    settings: Arguments,
    /// The world handles effects
    world: World<S, P::Index>,
}

impl<T: Clone, M: Mutator<T>, S: Serializer<Value = T>, Sens: Sensor, P: Pool<TestCase = FuzzedInput<T, M>>>
    FuzzerState<T, M, S, Sens, P>
where
    P: CompatibleWithSensor<Sens>,
{
    #[no_coverage]
    fn get_input<'a>(
        fuzzer_input_idx: &'a FuzzerInputIndex<FuzzedInput<T, M>, P::Index>,
        pool: &'a P,
    ) -> Option<&'a FuzzedInput<T, M>> {
        match fuzzer_input_idx {
            FuzzerInputIndex::None => None,
            FuzzerInputIndex::Temporary(input) => Some(input),
            FuzzerInputIndex::Pool(idx) => Some(pool.get(*idx)),
        }
    }
}

#[no_coverage]
fn update_fuzzer_stats<B: Serializer, C: Hash + Eq>(stats: &mut FuzzerStats, world: &mut World<B, C>) {
    let microseconds = world.elapsed_time_since_last_checkpoint();
    let nbr_runs = stats.total_number_of_runs - stats.number_of_runs_since_last_reset_time;
    let nbr_runs_times_million = nbr_runs * 1_000_000;
    stats.exec_per_s = nbr_runs_times_million / microseconds;

    if microseconds > 1_000_000 {
        world.set_checkpoint_instant();
        stats.number_of_runs_since_last_reset_time = stats.total_number_of_runs;
    }
}

impl<T: Clone, M: Mutator<T>, S: Serializer<Value = T>, Sens: Sensor, P: Pool<TestCase = FuzzedInput<T, M>>>
    FuzzerState<T, M, S, Sens, P>
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
            SIGABRT | SIGBUS | SIGSEGV | SIGFPE | SIGALRM => {
                if let Some(input) = Self::get_input(&self.input_idx, &self.pool) {
                    let cplx = input.complexity(&self.mutator);
                    let _ = self.world.save_artifact(&input.value, cplx);

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
        set_signal_handlers(move |sig| (&mut *ptr).receive_signal(sig));
    }
}

pub struct Fuzzer<T, FT, F, M, S, Sens, P>
where
    FT: ?Sized,
    T: Clone + Borrow<FT>,
    F: Fn(&FT) -> bool,
    M: Mutator<T>,
    S: Serializer<Value = T>,
    Sens: Sensor,
    P: Pool,
    P: CompatibleWithSensor<Sens>,
    Self: 'static,
{
    state: FuzzerState<T, M, S, Sens, P>,
    test: F,
    phantom: std::marker::PhantomData<FT>,
}

impl<T, FT, F, M, S, Sens, P> Fuzzer<T, FT, F, M, S, Sens, P>
where
    FT: ?Sized,
    T: Clone + Borrow<FT>,
    F: Fn(&FT) -> bool,
    M: Mutator<T>,
    S: Serializer<Value = T>,
    Sens: Sensor,
    P: Pool<TestCase = FuzzedInput<T, M>>,
    P: CompatibleWithSensor<Sens>,
    Self: 'static,
    P::Stats: Default,
{
    #[no_coverage]
    fn new(test: F, mutator: M, sensor: Sens, pool: P, settings: Arguments, world: World<S, P::Index>) -> Self {
        let arbitrary_step = mutator.default_arbitrary_step();
        Fuzzer {
            state: FuzzerState {
                sensor,
                pool,
                mutator,
                arbitrary_step,
                input_idx: FuzzerInputIndex::None,
                fuzzer_stats: FuzzerStats::default(),
                settings,
                world,
            },
            test,
            phantom: std::marker::PhantomData,
        }
    }

    #[no_coverage]
    fn test_and_process_input(&mut self, cplx: f64) -> Result<(), ReasonForStopping<T>> {
        let Fuzzer {
            state:
                FuzzerState {
                    mutator,
                    sensor,
                    pool,
                    input_idx,
                    fuzzer_stats,
                    world,
                    ..
                },
            test,
            ..
        } = self;

        // we have verified in the caller function that there is an input
        let input = FuzzerState::<T, M, S, Sens, P>::get_input(input_idx, pool).unwrap();

        std::panic::set_hook(Box::new(move |panic_info| {
            let mut hasher = DefaultHasher::new();
            panic_info.location().hash(&mut hasher);
            unsafe {
                TEST_FAILURE = Some(TestFailure {
                    display: format!("{}", panic_info),
                    id: hasher.finish(),
                });
            }
        }));

        sensor.start_recording();
        let cell = NotUnwindSafe { value: test };
        let input_cell = NotUnwindSafe {
            value: input.value.borrow(),
        };
        let result = catch_unwind(|| (cell.value)(input_cell.value));
        sensor.stop_recording();
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
            Ok(true) => { false }
        };
        if test_failure && self.state.settings.stop_after_first_failure {
            self.state.world.save_artifact(&input.value, cplx)?;
            return Err(ReasonForStopping::TestFailure(input.value.clone()));
        }

        fuzzer_stats.total_number_of_runs += 1;

        let get_input = match &input_idx {
            FuzzerInputIndex::None => unreachable!(),
            FuzzerInputIndex::Temporary(input) => Either::Right(input),
            FuzzerInputIndex::Pool(idx) => Either::Left(*idx),
        };
        let clone_input = |input: &FuzzedInput<T, M>| input.new_source(mutator);

        pool.process(sensor, get_input, &clone_input, cplx, |corpus_delta, pool_stats| {
            let corpus_delta = corpus_delta.convert(|x| x.value.clone());
            update_fuzzer_stats(fuzzer_stats, world);
            let event = corpus_delta.fuzzer_event();
            world.update_corpus(corpus_delta)?;
            world.report_event(event, Some((fuzzer_stats, pool_stats)));
            Ok(())
        })?;

        Ok(())
    }

    #[no_coverage]
    fn process_next_input(&mut self) -> Result<(), ReasonForStopping<T>> {
        let pool = &mut self.state.pool;
        loop {
            if let Some(idx) = pool.get_random_index() {
                self.state.input_idx = FuzzerInputIndex::Pool(idx);
                let input = pool.get_mut(idx);
                let generation = input.generation;
                if let Some((unmutate_token, cplx)) =
                    input.mutate(&mut self.state.mutator, self.state.settings.max_input_cplx)
                {
                    if cplx < self.state.settings.max_input_cplx {
                        self.test_and_process_input(cplx)?;
                    }
                    let pool = &mut self.state.pool;
                    // Retrieving the input may fail because the input may have been deleted
                    if let Some(input) = pool.retrieve_after_processing(idx, generation) {
                        input.unmutate(&self.state.mutator, unmutate_token);
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
            .filter_map(|value| {
                if let Some((cache, mutation_step)) = self.state.mutator.validate_value(&value) {
                    Some(FuzzedInput::new(value, cache, mutation_step, 0))
                } else {
                    None
                }
            })
            .collect();

        for _ in 0..100 {
            if let Some((input, _)) = self.state.arbitrary_input() {
                inputs.push(input);
            } else {
                break;
            }
        }

        inputs.drain_filter(|i| i.complexity(&self.state.mutator) > self.state.settings.max_input_cplx);
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

        self.state.world.set_start_instant();
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

    /// Reads a corpus of inputs from the [World] and minifies the corpus
    /// such that only the highest-scoring inputs are kept.
    ///
    /// The number of inputs to keep is taken from
    /// [`self.settings.corpus_size`](FuzzerSettings::corpus_size)
    #[no_coverage]
    fn corpus_minifying_loop(&mut self, corpus_size: usize) -> Result<(), ReasonForStopping<T>> {
        self.state.world.report_event(
            FuzzerEvent::Start,
            Some((&self.state.fuzzer_stats, &self.state.pool.stats())),
        );
        self.process_initial_inputs()?;

        let FuzzerState {
            pool,
            fuzzer_stats,
            world,
            ..
        } = &mut self.state;

        world.report_event(FuzzerEvent::DidReadCorpus, Some((fuzzer_stats, pool.stats().clone())));

        pool.minify(corpus_size, |corpus_delta, pool_stats| {
            let corpus_delta = corpus_delta.convert(|x| x.value.clone());
            let event = corpus_delta.fuzzer_event();
            world.update_corpus(corpus_delta)?;
            world.report_event(event, Some((fuzzer_stats, pool_stats)));
            Ok(())
        })?;
        world.report_event(FuzzerEvent::Done, Some((fuzzer_stats, pool.stats())));
        Ok(())
    }
}

struct NotUnwindSafe<T> {
    value: T,
}

impl<T> UnwindSafe for NotUnwindSafe<T> {}
impl<T> RefUnwindSafe for NotUnwindSafe<T> {}

pub enum TerminationStatus {
    Success = 0,
    Crash = 1,
    TestFailure = 2,
    Unknown = 3,
}

#[no_coverage]
pub fn launch<T, FT, F, M, S, Sens, P>(
    test: F,
    mutator: M,
    serializer: S,
    sensor: Sens,
    pool: P,
    mut args: Arguments,
) -> Result<(), ReasonForStopping<T>>
where
    FT: ?Sized,
    T: Clone + Borrow<FT>,
    F: Fn(&FT) -> bool,
    M: Mutator<T>,
    S: Serializer<Value = T>,
    Fuzzer<T, FT, F, M, S, CodeCoverageSensor, UniqueCoveragePool<FuzzedInput<T, M>>>: 'static,
    Sens: Sensor + 'static,
    P: Pool<TestCase = FuzzedInput<T, M>> + CompatibleWithSensor<Sens> + 'static,
{
    let command = &args.command;
    let result = match command {
        FuzzerCommand::Fuzz => {
            let test_failure = TestFailureSensor::default();
            let sensor = AndSensor {
                s1: sensor,
                s2: test_failure,
            };
            let artifacts_pool = ArtifactsPool::new("artifacts");
            let pool = AndPool {
                p1: pool,
                p2: artifacts_pool,
                ratio_choose_first: 254,
                rng: fastrand::Rng::new(),
            };
            let mut fuzzer = Fuzzer::new(
                test,
                mutator,
                sensor,
                pool,
                args.clone(),
                World::new(serializer, args.clone()),
            );
            unsafe { fuzzer.state.set_up_signal_handler() };
            fuzzer.main_loop()?
        }
        FuzzerCommand::MinifyInput { input_file } => {
            let world = World::new(serializer, args.clone());
            let value = world.read_input_file(input_file)?;
            if let Some((cache, mutation_step)) = mutator.validate_value(&value) {
                args.max_input_cplx = mutator.complexity(&value, &cache) - 0.01;

                let sensor = AndSensor {
                    s1: sensor,
                    s2: NoopSensor,
                };
                let pool = AndPool {
                    p1: pool,
                    p2: UnitPool::new(FuzzedInput::new(value, cache, mutation_step, 0)),
                    ratio_choose_first: 240,
                    rng: fastrand::Rng::new(),
                };
                let mut fuzzer = Fuzzer::<_, _, _, _, _, _, _>::new(test, mutator, sensor, pool, args.clone(), world);

                unsafe { fuzzer.state.set_up_signal_handler() };

                fuzzer.main_loop()?
            } else {
                // TODO: send a better error message saying some inputs in the corpus cannot be read
                // TODO: there should be an option to ignore invalid values
                println!("A value in the input corpus is invalid.");
                Ok(())
            }
        }
        FuzzerCommand::Read { input_file } => {
            // no signal handlers are installed, but that should be ok as the exit code won't be 0
            let mut world = World::<_, ()>::new(serializer, args.clone());
            let value = world.read_input_file(input_file)?;
            if let Some((cache, mutation_step)) = mutator.validate_value(&value) {
                let input = FuzzedInput::new(value, cache, mutation_step, 0);
                let cplx = input.complexity(&mutator);

                let cell = NotUnwindSafe { value: test };
                let input_cell = NotUnwindSafe {
                    value: input.value.borrow(),
                };
                let result = catch_unwind(|| (cell.value)(input_cell.value));

                if result.is_err() || !result.unwrap() {
                    world.report_event::<EmptyStats>(FuzzerEvent::TestFailure, None);
                    world.save_artifact(&input.value, cplx)?;
                    // in this case we really want to exit with a non-zero termination status here
                    // because the Read command is only used by the input minify command from cargo-fuzzcheck
                    // which checks that a crash happens by looking at the exit code
                    // so we don't want to handle any error
                    exit(TerminationStatus::TestFailure as i32);
                }
            } else {
                // TODO: send a better error message saying some inputs in the corpus cannot be read
                println!("A value in the input corpus is invalid.");
            }
            Ok(())
        }
        FuzzerCommand::MinifyCorpus { corpus_size } => {
            let mut fuzzer = Fuzzer::<_, _, _, _, _, _, _>::new(
                test,
                mutator,
                sensor,
                pool,
                args.clone(),
                World::new(serializer, args.clone()),
            );
            // fuzzer.sensor_and_pool.update
            unsafe { fuzzer.state.set_up_signal_handler() };

            fuzzer.corpus_minifying_loop(*corpus_size)
        }
    };

    let _ = std::panic::take_hook();

    result
}
