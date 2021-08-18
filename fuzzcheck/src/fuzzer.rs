//! Fuzzing engine. Connects the CodeCoverageSensor
//!to the [Pool] and uses an evolutionary algorithm using [Mutator] to find new interesting
//! test inputs.

use crate::code_coverage_sensor::CodeCoverageSensor;
use crate::data_structures::SlabKey;
#[cfg(feature = "ui")]
use crate::pool::Input;
use crate::pool::{AnalyzedFeature, Pool, PoolIndex};
use crate::signals_handler::set_signal_handlers;
use crate::world::World;
use crate::world::WorldAction;
use crate::{traits::Mutator, traits::Serializer, Feature, FuzzedInput};

use fuzzcheck_common::arg::{Arguments, FuzzerCommand};
use fuzzcheck_common::{FuzzerEvent, FuzzerStats};

use libc::{SIGABRT, SIGALRM, SIGBUS, SIGFPE, SIGINT, SIGSEGV, SIGTERM};

use std::borrow::Borrow;
use std::panic::{catch_unwind, RefUnwindSafe, UnwindSafe};
use std::path::Path;
use std::process::exit;
use std::result::Result;

enum FuzzerInputIndex<T: Clone, M: Mutator<T>> {
    None,
    Temporary(FuzzedInput<T, M>),
    Pool(PoolIndex<T, M>),
}

pub(crate) struct AnalysisResult<T: Clone, M: Mutator<T>> {
    // will be left empty if the input is not interesting
    pub existing_features: Vec<SlabKey<AnalyzedFeature<T, M>>>,
    pub new_features: Vec<Feature>,
    // always contains the value of the lowest stack,
    // will need to check if it is lower than the other
    // inputs in the pool
    // pub _lowest_stack: usize,
}

struct FuzzerState<T: Clone, M: Mutator<T>, S: Serializer<Value = T>> {
    sensor: CodeCoverageSensor,
    mutator: M,
    pool: Pool<T, M>,
    arbitrary_step: M::ArbitraryStep,
    input_idx: FuzzerInputIndex<T, M>,
    stats: FuzzerStats,
    settings: Arguments,
    world: World<S>,
}

impl<T: Clone, M: Mutator<T>, S: Serializer<Value = T>> FuzzerState<T, M, S> {
    #[no_coverage]
    fn get_input<'a>(
        fuzzer_input_idx: &'a FuzzerInputIndex<T, M>,
        pool: &'a Pool<T, M>,
    ) -> Option<&'a FuzzedInput<T, M>> {
        match fuzzer_input_idx {
            FuzzerInputIndex::None => None,
            FuzzerInputIndex::Temporary(input) => Some(input),
            FuzzerInputIndex::Pool(idx) => Some(pool.get_ref(*idx)),
        }
    }
}

impl<T: Clone, M: Mutator<T>, S: Serializer<Value = T>> FuzzerState<T, M, S>
where
    Self: 'static,
{
    #[no_coverage]
    fn update_stats(&mut self) {
        let microseconds = self.world.elapsed_time_since_last_checkpoint();

        let nbr_runs = self.stats.total_number_of_runs - self.stats.number_of_runs_since_last_reset_time;
        let nbr_runs_times_million = nbr_runs * 1_000_000;
        self.stats.exec_per_s = nbr_runs_times_million / microseconds;

        self.stats.pool_size = self.pool.len();
        self.stats.score = self.pool.score();
        self.stats.avg_cplx = self.pool.average_complexity as f64;
        if microseconds > 1_000_000 {
            self.world.set_checkpoint_instant();
            self.stats.number_of_runs_since_last_reset_time = self.stats.total_number_of_runs;
        }
    }
    #[no_coverage]
    fn receive_signal(&mut self, signal: i32) -> ! {
        self.world
            .do_actions(
                vec![WorldAction::ReportEvent(FuzzerEvent::CaughtSignal(signal as i32))],
                &self.stats,
            )
            .unwrap();

        match signal {
            SIGABRT | SIGBUS | SIGSEGV | SIGFPE | SIGALRM => {
                if let Some(input) = Self::get_input(&self.input_idx, &self.pool) {
                    let cplx = input.complexity(&self.mutator);
                    let _ = self.world.save_artifact(&input.value, cplx);

                    exit(TerminationStatus::Crash as i32);
                } else {
                    self.world
                        .do_actions(vec![WorldAction::ReportEvent(FuzzerEvent::CrashNoInput)], &self.stats)
                        .unwrap();
                    //let _ = self.world.report_event(FuzzerEvent::CrashNoInput, Some(self.stats));

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
            Some((FuzzedInput::new(v, cache, step), cplx))
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

pub struct Fuzzer<T, FT, F, M, S>
where
    FT: ?Sized,
    T: Clone + Borrow<FT>,
    F: Fn(&FT) -> bool,
    M: Mutator<T>,
    S: Serializer<Value = T>,
    Self: 'static,
{
    state: FuzzerState<T, M, S>,
    test: F,
    phantom: std::marker::PhantomData<(T, FT)>,
}

impl<T, FT, F, M, S> Fuzzer<T, FT, F, M, S>
where
    FT: ?Sized,
    T: Clone + Borrow<FT>,
    F: Fn(&FT) -> bool,
    M: Mutator<T>,
    S: Serializer<Value = T>,
{
    #[no_coverage]
    pub fn new(test: F, mutator: M, sensor: CodeCoverageSensor, settings: Arguments, world: World<S>) -> Self {
        let arbitrary_step = mutator.default_arbitrary_step();
        Fuzzer {
            state: FuzzerState {
                sensor,
                mutator,
                pool: Pool::default(),
                arbitrary_step,
                input_idx: FuzzerInputIndex::None,
                stats: FuzzerStats::new(),
                settings,
                world,
            },
            test,
            phantom: std::marker::PhantomData,
        }
    }

    #[no_coverage]
    fn test_input(
        test: &F,
        mutator: &M,
        input: &FuzzedInput<T, M>,
        world: &mut World<S>,
        stats: FuzzerStats,
        sensor: &mut CodeCoverageSensor,
    ) -> Result<(), std::io::Error> {
        unsafe {
            sensor.clear();
        }

        unsafe {
            sensor.start_recording();
        }

        let cell = NotUnwindSafe { value: test };
        let input_cell = NotUnwindSafe {
            value: input.value.borrow(),
        };
        let result = catch_unwind(|| (cell.value)(input_cell.value));

        unsafe {
            sensor.stop_recording();
        }

        if result.is_err() || !result.unwrap() {
            world.do_actions(vec![WorldAction::ReportEvent(FuzzerEvent::TestFailure)], &stats)?;
            world.save_artifact(&input.value, input.complexity(mutator))?;
            exit(TerminationStatus::TestFailure as i32);
        }

        Ok(())
    }

    #[no_coverage]
    fn analyze(&mut self, cur_input_cplx: f64) -> Option<AnalysisResult<T, M>> {
        let mut best_input_for_a_feature = false;

        let slab_features = &self.state.pool.slab_features;

        unsafe {
            // let mut step_iter = LargeStepFindIter::new(&self.state.pool.features);
            for i in 0..self.state.sensor.coverage.len() {
                let mut features = self
                    .state
                    .pool
                    .features
                    .get_unchecked(
                        self.state
                            .pool
                            .features_range_for_coverage_index
                            .get_unchecked(i)
                            .clone(),
                    )
                    .iter()
                    .peekable();
                self.state.sensor.iterate_over_collected_features(
                    i,
                    #[no_coverage]
                    |feature| loop {
                        if let Some(f_iter) = features.peek() {
                            if f_iter.feature < feature {
                                let _ = features.next();
                                continue;
                            } else if f_iter.feature == feature {
                                let f = &slab_features[f_iter.key];
                                if cur_input_cplx < f.least_complexity {
                                    best_input_for_a_feature = true;
                                }
                                break;
                            } else {
                                best_input_for_a_feature = true;
                                break;
                            }
                        } else {
                            best_input_for_a_feature = true;
                            break;
                        }
                    },
                );
            }
        }
        if best_input_for_a_feature {
            let mut existing_features = Vec::new();
            let mut new_features = Vec::new();

            unsafe {
                for i in 0..self.state.sensor.coverage.len() {
                    let mut features = self
                        .state
                        .pool
                        .features
                        .get_unchecked(
                            self.state
                                .pool
                                .features_range_for_coverage_index
                                .get_unchecked(i)
                                .clone(),
                        )
                        .iter()
                        .peekable();
                    self.state.sensor.iterate_over_collected_features(
                        i,
                        #[no_coverage]
                        |feature| loop {
                            if let Some(f_iter) = features.peek() {
                                if f_iter.feature < feature {
                                    let _ = features.next();
                                    continue;
                                } else if f_iter.feature == feature {
                                    existing_features.push(f_iter.key);
                                    break;
                                } else {
                                    new_features.push(feature);
                                    break;
                                }
                            } else {
                                new_features.push(feature);
                                break;
                            }
                        },
                    );
                }
                Some(AnalysisResult {
                    existing_features,
                    new_features,
                })
            }
        } else {
            None
        }
    }

    #[no_coverage]
    fn test_input_and_analyze(&mut self, cplx: f64) -> Result<(), std::io::Error> {
        self.state.world.handle_user_message();

        // we have verified in the caller function that there is an input
        {
            let input = FuzzerState::<T, M, S>::get_input(&self.state.input_idx, &self.state.pool).unwrap();

            Self::test_input(
                &self.test,
                &self.state.mutator,
                input,
                &mut self.state.world,
                self.state.stats,
                &mut self.state.sensor,
            )?;
            self.state.stats.total_number_of_runs += 1;
        }
        if let Some(result) = self.analyze(cplx) {
            #[allow(unused_variables)]
            let new_input_key = {
                // call state.get_input again to satisfy borrow checker
                let input_cloned = FuzzerState::<T, M, S>::get_input(&self.state.input_idx, &self.state.pool)
                    .unwrap()
                    .new_source(&self.state.mutator);
                let (actions, new_input_key) = self.state.pool.add(input_cloned, cplx, result);
                self.state
                    .pool
                    .update_feature_ranges_for_coverage(&self.state.sensor.index_ranges);
                self.state.update_stats();
                self.state.world.do_actions(actions, &self.state.stats)?;
                new_input_key
            };
            #[cfg(feature = "ui")]
            if let Some(new_input_key) = new_input_key {
                self.send_coverage_location(new_input_key)?;
            }

            Ok(())
        } else {
            Ok(())
        }
    }
    #[cfg(feature = "ui")]
    #[no_coverage]
    fn send_coverage_location(&mut self, input_key: SlabKey<Input<T, M>>) -> Result<(), std::io::Error> {
        code_coverage_sensor::clear();
        unsafe {
            code_coverage_sensor::TRACE_PC_GUARD_IMPL = code_coverage_sensor::record_location_guard;
        }
        code_coverage_sensor::start_recording();
        let cell = NotUnwindSafe { value: &self.test };
        let input = FuzzerState::<T, M, S>::get_input(&self.state.input_idx, &self.state.pool).unwrap();
        let input_cell = NotUnwindSafe {
            value: input.value.borrow(),
        };
        let result = catch_unwind(|| (cell.value)(input_cell.value));
        assert!(!result.is_err() && result.unwrap());
        code_coverage_sensor::stop_recording();
        unsafe {
            code_coverage_sensor::TRACE_PC_GUARD_IMPL = code_coverage_sensor::trace_pc_guard_increase_counter;
        }
        let action = unsafe {
            code_coverage_sensor::with_coverage_map(|coverage_map| {
                self.state
                    .pool
                    .send_coverage_information_for_input(input_key, coverage_map)
            })
        };
        self.state.world.do_actions(vec![action], &self.state.stats)?;
        Ok(())
    }

    #[no_coverage]
    fn process_next_inputs(&mut self) -> Result<(), std::io::Error> {
        if let Some(idx) = self.state.pool.random_index() {
            self.state.input_idx = FuzzerInputIndex::Pool(idx);
            let input = self.state.pool.get(idx);
            if let Some((unmutate_token, cplx)) =
                input.mutate(&mut self.state.mutator, self.state.settings.max_input_cplx)
            {
                if cplx < self.state.settings.max_input_cplx {
                    self.test_input_and_analyze(cplx)?;
                }

                // Retrieving the input may fail because the input may have been deleted
                if let Some(input) = self
                    .state
                    .pool
                    .retrieve_source_input_for_unmutate(idx, self.state.stats.total_number_of_runs)
                {
                    input.unmutate(&self.state.mutator, unmutate_token);
                }

                Ok(())
            } else {
                self.state.pool.mark_input_as_dead_end(idx);
                self.process_next_inputs()
            }
        } else if let Some((input, cplx)) = self.state.arbitrary_input() {
            self.state.input_idx = FuzzerInputIndex::Temporary(input);

            if cplx < self.state.settings.max_input_cplx {
                self.test_input_and_analyze(cplx)?;
            }

            Ok(())
        } else {
            self.state
                .world
                .do_actions(vec![WorldAction::ReportEvent(FuzzerEvent::End)], &self.state.stats)?;
            //self.state.world.report_event(FuzzerEvent::End, None);
            exit(TerminationStatus::Success as i32);
        }
    }

    #[no_coverage]
    fn process_initial_inputs(&mut self) -> Result<(), std::io::Error> {
        let mut inputs: Vec<FuzzedInput<T, M>> = self
            .state
            .world
            .read_input_corpus()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|value| {
                if let Some((cache, mutation_step)) = self.state.mutator.validate_value(&value) {
                    Some(FuzzedInput::new(value, cache, mutation_step))
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

        self.state.world.set_start_instant();
        self.state.world.set_checkpoint_instant();
        for input in inputs {
            let cplx = input.complexity(&self.state.mutator);
            self.state.input_idx = FuzzerInputIndex::Temporary(input);
            self.test_input_and_analyze(cplx)?;
        }

        Ok(())
    }

    #[no_coverage]
    fn main_loop(&mut self) -> Result<!, std::io::Error> {
        self.state
            .world
            .do_actions(vec![WorldAction::ReportEvent(FuzzerEvent::Start)], &self.state.stats)?;
        self.process_initial_inputs()?;
        self.state.world.do_actions(
            vec![WorldAction::ReportEvent(FuzzerEvent::DidReadCorpus)],
            &self.state.stats,
        )?;

        let mut next_milestone = (self.state.stats.total_number_of_runs + 100_000) * 2;
        loop {
            self.process_next_inputs()?;
            if self.state.stats.total_number_of_runs >= next_milestone {
                self.state.update_stats();
                self.state
                    .world
                    .do_actions(vec![WorldAction::ReportEvent(FuzzerEvent::Pulse)], &self.state.stats)?;
                next_milestone = self.state.stats.total_number_of_runs * 2;
            }
        }
    }

    /// Reads a corpus of inputs from the [World] and minifies the corpus
    /// such that only the highest-scoring inputs are kept.
    ///
    /// The number of inputs to keep is taken from
    /// [`self.settings.corpus_size`](FuzzerSettings::corpus_size)
    #[no_coverage]
    fn corpus_minifying_loop(&mut self, corpus_size: usize) -> Result<(), std::io::Error> {
        self.state
            .world
            .do_actions(vec![WorldAction::ReportEvent(FuzzerEvent::Start)], &self.state.stats)?;

        self.process_initial_inputs()?;

        self.state.world.do_actions(
            vec![WorldAction::ReportEvent(FuzzerEvent::DidReadCorpus)],
            &self.state.stats,
        )?;

        while self.state.pool.len() > corpus_size {
            let actions = self.state.pool.remove_lowest_scoring_input();
            self.state.update_stats();
            self.state.world.do_actions(actions, &self.state.stats)?;
        }

        self.state
            .world
            .do_actions(vec![WorldAction::ReportEvent(FuzzerEvent::Done)], &self.state.stats)?;
        Ok(())
    }

    #[no_coverage]
    fn input_minifying_loop(&mut self, file: &Path) -> Result<(), std::io::Error> {
        self.state
            .world
            .do_actions(vec![WorldAction::ReportEvent(FuzzerEvent::Start)], &self.state.stats)?;
        let value = self.state.world.read_input_file(file)?;

        if let Some((cache, mutation_step)) = self.state.mutator.validate_value(&value) {
            let input = FuzzedInput::<T, M>::new(value, cache, mutation_step);
            let input_cplx = input.complexity(&self.state.mutator);
            self.state.settings.max_input_cplx = input_cplx - 0.01;

            self.state.pool.add_favored_input(input);

            self.state.world.set_start_instant();
            self.state.world.set_checkpoint_instant();
            loop {
                self.process_next_inputs()?;
            }
        } else {
            todo!()
        }
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
pub fn launch<T, FT, F, M, S, Exclude, Keep>(
    test: F,
    mutator: M,
    sensor_exclude_files: Exclude,
    sensor_keep_files: Keep,
    serializer: S,
    args: Arguments,
) -> Result<(), std::io::Error>
where
    FT: ?Sized,
    T: Clone + Borrow<FT>,
    F: Fn(&FT) -> bool,
    M: Mutator<T>,
    S: Serializer<Value = T>,
    Fuzzer<T, FT, F, M, S>: 'static,
    Exclude: Fn(&Path) -> bool,
    Keep: Fn(&Path) -> bool,
{
    let command = &args.command;

    let sensor = CodeCoverageSensor::new(sensor_exclude_files, sensor_keep_files);

    let mut fuzzer = Fuzzer::new(
        test,
        mutator,
        sensor,
        args.clone(),
        World::new(serializer, args.clone()),
    );
    unsafe { fuzzer.state.set_up_signal_handler() };
    fuzzer
        .state
        .pool
        .update_feature_ranges_for_coverage(&fuzzer.state.sensor.index_ranges);
    match command {
        FuzzerCommand::Fuzz => fuzzer.main_loop()?,
        FuzzerCommand::MinifyInput { input_file } => fuzzer.input_minifying_loop(input_file)?,
        FuzzerCommand::Read { input_file } => {
            let value = fuzzer.state.world.read_input_file(input_file)?;
            if let Some((cache, mutation_step)) = fuzzer.state.mutator.validate_value(&value) {
                fuzzer.state.input_idx = FuzzerInputIndex::Temporary(FuzzedInput::new(value, cache, mutation_step));
                let input = FuzzerState::<T, M, S>::get_input(&fuzzer.state.input_idx, &fuzzer.state.pool).unwrap();
                Fuzzer::<T, FT, F, M, S>::test_input(
                    &fuzzer.test,
                    &fuzzer.state.mutator,
                    input,
                    &mut fuzzer.state.world,
                    fuzzer.state.stats,
                    &mut fuzzer.state.sensor,
                )?;
            } else {
                todo!()
            }
        }
        FuzzerCommand::MinifyCorpus { corpus_size } => fuzzer.corpus_minifying_loop(*corpus_size)?,
    };
    Ok(())
}
