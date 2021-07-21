//! Fuzzing engine. Connects the CodeCoverageSensor
//!to the [Pool] and uses an evolutionary algorithm using [Mutator] to find new interesting
//! test inputs.

use crate::nix_subset as nix;
#[cfg(feature = "ui")]
use crate::pool::Input;
use crate::pool::{AnalyzedFeature, Pool, PoolIndex};
use crate::signals_handler::{set_signal_handlers, set_timer};
use crate::world::World;
use crate::world::WorldAction;
use crate::{
    code_coverage_sensor,
    data_structures::{LargeStepFindIter, SlabKey},
};
use crate::{Feature, FuzzedInput, Mutator, Serializer};

use fuzzcheck_common::{FuzzerEvent, FuzzerStats};

use fuzzcheck_common::arg::{FullCommandLineArguments, FuzzerCommand};
use nix::signal;

use std::panic::{catch_unwind, RefUnwindSafe, UnwindSafe};
use std::process::exit;
use std::result::Result;

use std::borrow::Borrow;

use std::convert::TryFrom;

enum FuzzerInputIndex<T: Clone, M: Mutator<T>> {
    None,
    Temporary(FuzzedInput<T, M>),
    Pool(PoolIndex<T, M>),
}

struct AnalysisCache<T: Clone, M: Mutator<T>> {
    existing_features: Vec<SlabKey<AnalyzedFeature<T, M>>>,
    new_features: Vec<Feature>,
}
impl<T: Clone, M: Mutator<T>> Default for AnalysisCache<T, M> {
    fn default() -> Self {
        Self {
            existing_features: Vec::new(),
            new_features: Vec::new(),
        }
    }
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
    mutator: M,
    pool: Pool<T, M>,
    arbitrary_step: M::ArbitraryStep,
    input_idx: FuzzerInputIndex<T, M>,
    stats: FuzzerStats,
    settings: FullCommandLineArguments,
    world: World<S>,
    analysis_cache: AnalysisCache<T, M>,
}

impl<T: Clone, M: Mutator<T>, S: Serializer<Value = T>> FuzzerState<T, M, S> {
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
    fn update_stats(&mut self) {
        let microseconds = self.world.elapsed_time_since_last_checkpoint();

        let nbr_runs = self.stats.total_number_of_runs - self.stats.number_of_runs_since_last_reset_time;
        let nbr_runs_times_million = nbr_runs * 1_000_000;
        self.stats.exec_per_s = nbr_runs_times_million / microseconds;

        self.stats.pool_size = self.pool.len();
        self.stats.score = self.pool.score();
        self.stats.avg_cplx = self.pool.average_complexity;
        if microseconds > 1_000_000 {
            self.world.set_checkpoint_instant();
            self.stats.number_of_runs_since_last_reset_time = self.stats.total_number_of_runs;
        }
    }

    fn receive_signal(&mut self, signal: i32) -> ! {
        use signal::Signal::{self, *};
        if let Ok(signal) = Signal::try_from(signal) {
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
        } else {
            exit(TerminationStatus::Unknown as i32)
        }
    }

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
    pub fn new(test: F, mutator: M, settings: FullCommandLineArguments, world: World<S>) -> Self {
        let arbitrary_step = mutator.default_arbitrary_step();
        Fuzzer {
            state: FuzzerState {
                mutator,
                pool: Pool::default(),
                arbitrary_step,
                input_idx: FuzzerInputIndex::None,
                stats: FuzzerStats::new(),
                settings,
                world,
                analysis_cache: AnalysisCache::default(),
            },
            test,
            phantom: std::marker::PhantomData,
        }
    }

    fn max_iter(&self) -> usize {
        if self.state.settings.max_nbr_of_runs == 0 {
            usize::max_value()
        } else {
            self.state.settings.max_nbr_of_runs
        }
    }

    fn test_input(
        test: &F,
        mutator: &M,
        input: &FuzzedInput<T, M>,
        timeout: usize,
        world: &mut World<S>,
        stats: FuzzerStats,
    ) -> Result<(), std::io::Error> {
        code_coverage_sensor::clear();

        if timeout != 0 {
            set_timer(timeout);
        }

        code_coverage_sensor::start_recording();

        let cell = NotUnwindSafe { value: test };
        let input_cell = NotUnwindSafe {
            value: input.value.borrow(),
        };
        let result = catch_unwind(|| (cell.value)(input_cell.value));

        code_coverage_sensor::stop_recording();

        if timeout != 0 {
            set_timer(0);
        }

        if result.is_err() || !result.unwrap() {
            world.do_actions(vec![WorldAction::ReportEvent(FuzzerEvent::TestFailure)], &stats)?;
            //world.report_event(FuzzerEvent::TestFailure, Some(stats));
            let mut features: Vec<Feature> = Vec::new();
            code_coverage_sensor::iterate_over_collected_features(|f| features.push(f));
            world.save_artifact(&input.value, input.complexity(mutator))?;
            exit(TerminationStatus::TestFailure as i32);
        }

        Ok(())
    }

    fn analyze(&mut self, cur_input_cplx: f64) -> Option<AnalysisResult<T, M>> {
        let mut best_input_for_a_feature = false;

        let slab_features = &self.state.pool.slab_features;

        let mut step_iter = LargeStepFindIter::new(&self.state.pool.features);

        let existing_features = &mut self.state.analysis_cache.existing_features;
        let new_features = &mut self.state.analysis_cache.new_features;

        code_coverage_sensor::iterate_over_collected_features(|feature| {
            if let Some(f_for_iter) = step_iter.find(|feature_for_iter| feature_for_iter.feature.cmp(&feature)) {
                if f_for_iter.feature == feature {
                    existing_features.push(f_for_iter.key);
                    let f = &slab_features[f_for_iter.key];
                    if cur_input_cplx < f.least_complexity {
                        best_input_for_a_feature = true;
                    }
                } else {
                    best_input_for_a_feature = true;
                    new_features.push(feature);
                }
            } else {
                best_input_for_a_feature = true; // the feature goes at the end of the pool, and it is new
                new_features.push(feature);
            }
        });
        // let lowest_stack = code_coverage_sensor::lowest_stack();
        let result = if best_input_for_a_feature {
            Some(AnalysisResult {
                existing_features: existing_features.clone(),
                new_features: new_features.clone(),
                // _lowest_stack: lowest_stack,
            })
        }
        /*else if lowest_stack < self.state.pool.lowest_stack() {
            Some(AnalysisResult {
                existing_features: vec![],
                new_features: vec![],
                _lowest_stack: lowest_stack,
            })
        } */
        else {
            None
        };

        existing_features.clear();
        new_features.clear();

        result
    }

    fn test_input_and_analyze(&mut self, cplx: f64) -> Result<(), std::io::Error> {
        self.state.world.handle_user_message();

        // we have verified in the caller function that there is an input
        {
            let input = FuzzerState::<T, M, S>::get_input(&self.state.input_idx, &self.state.pool).unwrap();

            Self::test_input(
                &self.test,
                &self.state.mutator,
                &input,
                self.state.settings.timeout,
                &mut self.state.world,
                self.state.stats,
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
                let (actions, new_input_key) =
                    self.state
                        .pool
                        .add(input_cloned, cplx, result, self.state.stats.total_number_of_runs);
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
        } else {
            if let Some((input, cplx)) = self.state.arbitrary_input() {
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
    }

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

    fn main_loop(&mut self) -> Result<(), std::io::Error> {
        self.state
            .world
            .do_actions(vec![WorldAction::ReportEvent(FuzzerEvent::Start)], &self.state.stats)?;
        self.process_initial_inputs()?;
        self.state.world.do_actions(
            vec![WorldAction::ReportEvent(FuzzerEvent::DidReadCorpus)],
            &self.state.stats,
        )?;

        let mut next_milestone = (self.state.stats.total_number_of_runs + 100_000) * 2;
        while self.state.stats.total_number_of_runs < self.max_iter() {
            self.process_next_inputs()?;
            if self.state.stats.total_number_of_runs >= next_milestone {
                self.state.update_stats();
                self.state
                    .world
                    .do_actions(vec![WorldAction::ReportEvent(FuzzerEvent::Pulse)], &self.state.stats)?;
                next_milestone = self.state.stats.total_number_of_runs * 2;
            }
        }
        self.state
            .world
            .do_actions(vec![WorldAction::ReportEvent(FuzzerEvent::Done)], &self.state.stats)?;

        Ok(())
    }

    /// Reads a corpus of inputs from the [World] and minifies the corpus
    /// such that only the highest-scoring inputs are kept.
    ///
    /// The number of inputs to keep is taken from
    /// [`self.settings.corpus_size`](FuzzerSettings::corpus_size)
    fn corpus_minifying_loop(&mut self) -> Result<(), std::io::Error> {
        self.state
            .world
            .do_actions(vec![WorldAction::ReportEvent(FuzzerEvent::Start)], &self.state.stats)?;

        self.process_initial_inputs()?;

        self.state.world.do_actions(
            vec![WorldAction::ReportEvent(FuzzerEvent::DidReadCorpus)],
            &self.state.stats,
        )?;

        while self.state.pool.len() > self.state.settings.corpus_size {
            let actions = self.state.pool.remove_lowest_scoring_input();
            self.state.update_stats();
            self.state.world.do_actions(actions, &self.state.stats)?;
        }

        self.state
            .world
            .do_actions(vec![WorldAction::ReportEvent(FuzzerEvent::Done)], &self.state.stats)?;
        Ok(())
    }

    fn input_minifying_loop(&mut self) -> Result<(), std::io::Error> {
        self.state
            .world
            .do_actions(vec![WorldAction::ReportEvent(FuzzerEvent::Start)], &self.state.stats)?;
        let value = self.state.world.read_input_file()?;

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

pub fn launch<T, FT, F, M, S>(
    test: F,
    mutator: M,
    serializer: S,
    args: FullCommandLineArguments,
) -> Result<(), std::io::Error>
where
    FT: ?Sized,
    T: Clone + Borrow<FT>,
    F: Fn(&FT) -> bool,
    M: Mutator<T>,
    S: Serializer<Value = T>,
    Fuzzer<T, FT, F, M, S>: 'static,
{
    let command = args.command;

    let mut fuzzer = Fuzzer::new(test, mutator, args.clone(), World::new(serializer, args));
    unsafe { fuzzer.state.set_up_signal_handler() };

    match command {
        FuzzerCommand::Fuzz => fuzzer.main_loop()?,
        FuzzerCommand::MinifyInput => fuzzer.input_minifying_loop()?,
        FuzzerCommand::Read => {
            let value = fuzzer.state.world.read_input_file()?;
            if let Some((cache, mutation_step)) = fuzzer.state.mutator.validate_value(&value) {
                fuzzer.state.input_idx = FuzzerInputIndex::Temporary(FuzzedInput::new(value, cache, mutation_step));
                let input = FuzzerState::<T, M, S>::get_input(&fuzzer.state.input_idx, &fuzzer.state.pool).unwrap();
                Fuzzer::<T, FT, F, M, S>::test_input(
                    &fuzzer.test,
                    &fuzzer.state.mutator,
                    &input,
                    fuzzer.state.settings.timeout,
                    &mut fuzzer.state.world,
                    fuzzer.state.stats,
                )?;
            } else {
                todo!()
            }
        }
        FuzzerCommand::MinifyCorpus => fuzzer.corpus_minifying_loop()?,
    };
    Ok(())
}
