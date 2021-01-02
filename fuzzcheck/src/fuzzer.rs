//! Fuzzing engine. Connects the [CodeCoverageSensor](crate::code_coverage_sensor::CodeCoverageSensor)
//!to the [Pool] and uses an evolutionary algorithm using [Mutator] to find new interesting
//! test inputs.

use crate::data_structures::{LargeStepFindIter, SlabKey};
use crate::nix_subset as nix;
use crate::pool::{AnalyzedFeature, Pool, PoolIndex};
use crate::signals_handler::{set_signal_handlers, set_timer};
use crate::ui_world::TuiWorld;
use crate::world::{FuzzerEvent, FuzzerStats};
use crate::{code_coverage_sensor::shared_sensor, world::WorldAction};
use crate::{Feature, FuzzedInput, Mutator, Serializer};

use fuzzcheck_common::arg::{FuzzerCommand, ResolvedCommandLineArguments};

use nix::signal;

use std::panic::{catch_unwind, RefUnwindSafe, UnwindSafe};
use std::process::exit;
use std::result::Result;

use std::borrow::Borrow;

use std::convert::TryFrom;

enum FuzzerInputIndex<M: Mutator> {
    None,
    Temporary(FuzzedInput<M>),
    Pool(PoolIndex<M>),
}

struct AnalysisCache<M: Mutator> {
    existing_features: Vec<SlabKey<AnalyzedFeature<M>>>,
    new_features: Vec<Feature>,
}
impl<M: Mutator> Default for AnalysisCache<M> {
    fn default() -> Self {
        Self {
            existing_features: Vec::new(),
            new_features: Vec::new(),
        }
    }
}

pub(crate) struct AnalysisResult<M: Mutator> {
    // will be left empty if the input is not interesting
    pub existing_features: Vec<SlabKey<AnalyzedFeature<M>>>,
    pub new_features: Vec<Feature>,
    // always contains the value of the lowest stack,
    // will need to check if it is lower than the other
    // inputs in the pool
    pub lowest_stack: usize,
}

struct FuzzerState<M: Mutator, S: Serializer<Value = M::Value>> {
    mutator: M,
    pool: Pool<M>,
    arbitrary_step: M::ArbitraryStep,
    input_idx: FuzzerInputIndex<M>,
    stats: FuzzerStats,
    settings: ResolvedCommandLineArguments,
    world: TuiWorld<S>,
    analysis_cache: AnalysisCache<M>,
}

impl<M: Mutator, S: Serializer<Value = M::Value>> FuzzerState<M, S> {
    fn get_input(&self) -> Option<&FuzzedInput<M>> {
        match &self.input_idx {
            FuzzerInputIndex::None => None,
            FuzzerInputIndex::Temporary(input) => Some(input),
            FuzzerInputIndex::Pool(idx) => Some(self.pool.get_ref(*idx)),
        }
    }
}

impl<M: Mutator, S: Serializer<Value = M::Value>> FuzzerState<M, S>
where
    Self: 'static,
{
    fn update_stats(&mut self) {
        let microseconds = self.world.elapsed_time();

        let nbr_runs = self.stats.total_number_of_runs - self.stats.number_of_runs_since_last_reset_time;
        let nbr_runs_times_million = nbr_runs * 1_000_000;
        self.stats.exec_per_s = nbr_runs_times_million / microseconds;

        self.stats.pool_size = self.pool.len();
        self.stats.score = self.pool.score();
        self.stats.avg_cplx = self.pool.average_complexity;
        if microseconds > 1_000_000 {
            self.world.set_start_time();
            self.stats.number_of_runs_since_last_reset_time = self.stats.total_number_of_runs;
        }
    }

    fn receive_signal(&self, signal: i32) -> ! {
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
                    if let Some(input) = self.get_input() {
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
                SIGINT | SIGTERM => exit(TerminationStatus::Success as i32),
                _ => exit(TerminationStatus::Unknown as i32),
            }
        } else {
            exit(TerminationStatus::Unknown as i32)
        }
    }

    fn arbitrary_input(&mut self) -> Option<FuzzedInput<M>> {
        if let Some((v, cache)) = self
            .mutator
            .ordered_arbitrary(&mut self.arbitrary_step, self.settings.max_input_cplx)
        {
            let mutation_step = self.mutator.initial_step_from_value(&v);
            Some(FuzzedInput::new(v, cache, mutation_step))
        } else {
            None
        }
    }

    unsafe fn set_up_signal_handler(&self) {
        let ptr = self as *const Self;
        set_signal_handlers(move |sig| (&*ptr).receive_signal(sig));
    }
}

pub struct Fuzzer<T, F, M, S>
where
    T: ?Sized,
    M::Value: Borrow<T>,
    F: Fn(&T) -> bool,
    M: Mutator,
    S: Serializer<Value = M::Value>,
    Self: 'static,
{
    state: FuzzerState<M, S>,
    test: F,
    phantom: std::marker::PhantomData<T>,
}

impl<T, F, M, S> Fuzzer<T, F, M, S>
where
    T: ?Sized,
    M::Value: Borrow<T>,
    F: Fn(&T) -> bool,
    M: Mutator,
    S: Serializer<Value = M::Value>,
{
    pub fn new(test: F, mutator: M, settings: ResolvedCommandLineArguments, world: TuiWorld<S>) -> Self {
        Fuzzer {
            state: FuzzerState {
                mutator,
                pool: Pool::default(),
                arbitrary_step: <_>::default(),
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
        input: &FuzzedInput<M>,
        timeout: usize,
        world: &TuiWorld<S>,
        stats: FuzzerStats,
    ) -> Result<(), std::io::Error> {
        let sensor = shared_sensor();
        sensor.clear();

        if timeout != 0 {
            set_timer(timeout);
        }

        sensor.start_recording();

        let cell = NotUnwindSafe { value: test };
        let input_cell = NotUnwindSafe {
            value: input.value.borrow(),
        };
        let result = catch_unwind(|| (cell.value)(input_cell.value));

        sensor.stop_recording();

        if timeout != 0 {
            set_timer(0);
        }

        if result.is_err() || !result.unwrap() {
            world.do_actions(vec![WorldAction::ReportEvent(FuzzerEvent::TestFailure)], &stats)?;
            //world.report_event(FuzzerEvent::TestFailure, Some(stats));
            let mut features: Vec<Feature> = Vec::new();
            sensor.iterate_over_collected_features(|f| features.push(f));
            world.save_artifact(&input.value, input.complexity(mutator))?;
            exit(TerminationStatus::TestFailure as i32);
        }

        Ok(())
    }

    fn analyze(&mut self, cur_input_cplx: f64) -> Option<AnalysisResult<M>> {
        let mut best_input_for_a_feature = false;

        let sensor = shared_sensor();

        let slab_features = &self.state.pool.slab_features;

        let mut step_iter = LargeStepFindIter::new(&self.state.pool.features);

        let existing_features = &mut self.state.analysis_cache.existing_features;
        let new_features = &mut self.state.analysis_cache.new_features;

        sensor.iterate_over_collected_features(|feature| {
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

        let result = if best_input_for_a_feature {
            Some(AnalysisResult {
                existing_features: existing_features.clone(),
                new_features: new_features.clone(),
                lowest_stack: sensor.lowest_stack,
            })
        } else if sensor.lowest_stack < self.state.pool.lowest_stack() {
            Some(AnalysisResult {
                existing_features: vec![],
                new_features: vec![],
                lowest_stack: sensor.lowest_stack,
            })
        } else {
            None
        };

        existing_features.clear();
        new_features.clear();

        result
    }

    fn test_input_and_analyze(&mut self) -> Result<(), std::io::Error> {
        // we have verified in the caller function that there is an input
        let input = self.state.get_input().unwrap();
        let cplx = input.complexity(&self.state.mutator);
        Self::test_input(
            &self.test,
            &self.state.mutator,
            &input,
            self.state.settings.timeout,
            &self.state.world,
            self.state.stats,
        )?;
        self.state.stats.total_number_of_runs += 1;

        if let Some(result) = self.analyze(cplx) {
            // call state.get_input again to satisfy borrow checker
            let input_cloned = self.state.get_input().unwrap().new_source(&self.state.mutator);
            let actions = self
                .state
                .pool
                .add(input_cloned, cplx, result, self.state.stats.total_number_of_runs);
            self.state.update_stats();
            self.state.world.do_actions(actions, &self.state.stats)?;

            Ok(())
        } else {
            Ok(())
        }
    }

    fn process_next_inputs(&mut self) -> Result<(), std::io::Error> {
        if let Some(idx) = self.state.pool.random_index() {
            self.state.input_idx = FuzzerInputIndex::Pool(idx);
            let input = self.state.pool.get(idx);

            if let Some(unmutate_token) = input.mutate(&mut self.state.mutator, self.state.settings.max_input_cplx) {
                let cplx = input.complexity(&self.state.mutator);

                if cplx < self.state.settings.max_input_cplx {
                    self.test_input_and_analyze()?;
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
            if let Some(input) = self.state.arbitrary_input() {
                let cplx = input.complexity(&self.state.mutator);
                self.state.input_idx = FuzzerInputIndex::Temporary(input);

                if cplx < self.state.settings.max_input_cplx {
                    self.test_input_and_analyze()?;
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
        let mut inputs: Vec<FuzzedInput<M>> = self
            .state
            .world
            .read_input_corpus()
            .unwrap_or_default()
            .into_iter()
            .map(|value| {
                let cache = self.state.mutator.cache_from_value(&value);
                let mutation_step = self.state.mutator.initial_step_from_value(&value);
                FuzzedInput::new(value, cache, mutation_step)
            })
            .collect();

        if let Some(arbitrary) = FuzzedInput::default(&mut self.state.mutator) {
            inputs.push(arbitrary);
        }
        for _ in 0..100 {
            if let Some(input) = self.state.arbitrary_input() {
                inputs.push(input);
            } else {
                break;
            }
        }

        inputs.drain_filter(|i| i.complexity(&self.state.mutator) > self.state.settings.max_input_cplx);
        assert!(!inputs.is_empty());

        self.state.world.set_start_time();
        for input in inputs {
            self.state.input_idx = FuzzerInputIndex::Temporary(input);
            self.test_input_and_analyze()?;
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

        while self.state.stats.total_number_of_runs < self.max_iter() {
            self.process_next_inputs()?;
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
        let cache = self.state.mutator.cache_from_value(&value);
        let mutation_step = self.state.mutator.initial_step_from_value(&value);
        let input = FuzzedInput::<M>::new(value, cache, mutation_step);
        let input_cplx = input.complexity(&self.state.mutator);
        self.state.settings.max_input_cplx = input_cplx - 0.01;

        self.state.pool.add_favored_input(input);

        self.state.world.set_start_time();
        loop {
            self.process_next_inputs()?;
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

pub fn launch<T, F, M, S>(
    test: F,
    mutator: M,
    serializer: S,
    args: ResolvedCommandLineArguments,
) -> Result<(), std::io::Error>
where
    T: ?Sized,
    M::Value: Borrow<T>,
    F: Fn(&T) -> bool,
    M: Mutator,
    S: Serializer<Value = M::Value>,
    Fuzzer<T, F, M, S>: 'static,
{
    let command = args.command;

    let mut fuzzer = Fuzzer::new(test, mutator, args.clone(), TuiWorld::new(serializer, args));
    unsafe { fuzzer.state.set_up_signal_handler() };
    match command {
        FuzzerCommand::Fuzz => fuzzer.main_loop()?,
        FuzzerCommand::MinifyInput => fuzzer.input_minifying_loop()?,
        FuzzerCommand::Read => {
            let value = fuzzer.state.world.read_input_file()?;
            let cache = fuzzer.state.mutator.cache_from_value(&value);
            let mutation_step = fuzzer.state.mutator.initial_step_from_value(&value);

            fuzzer.state.input_idx = FuzzerInputIndex::Temporary(FuzzedInput::new(value, cache, mutation_step));
            let input = fuzzer.state.get_input().unwrap();
            Fuzzer::<T, F, M, S>::test_input(
                &fuzzer.test,
                &fuzzer.state.mutator,
                &input,
                fuzzer.state.settings.timeout,
                &fuzzer.state.world,
                fuzzer.state.stats,
            )?;
        }
        FuzzerCommand::MinifyCorpus => fuzzer.corpus_minifying_loop()?,
    };
    Ok(())
}
