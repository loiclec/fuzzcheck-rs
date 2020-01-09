use crate::code_coverage_sensor::*;
use crate::input::*;
use crate::input_pool::*;
use crate::signals_handler::*;
use crate::world::*;

use fuzzcheck_arg_parser::*;

use std::panic::{catch_unwind, RefUnwindSafe, UnwindSafe};
use std::process::exit;
use std::result::Result;

enum FuzzerInputIndex<I: FuzzedInput> {
    Temporary(UnifiedFuzzedInput<I>),
    Pool(InputPoolIndex),
}

struct FuzzerState<I: FuzzedInput> {
    pool: InputPool<I>,
    input_idx: FuzzerInputIndex<I>,
    stats: FuzzerStats,
    settings: CommandLineArguments,
    world: World<I>,
}

impl<I: FuzzedInput> FuzzerState<I> {
    fn get_input(&self) -> &UnifiedFuzzedInput<I> {
        match &self.input_idx {
            FuzzerInputIndex::Temporary(input) => &input,
            FuzzerInputIndex::Pool(idx) => self.pool.get_ref(*idx),
        }
    }
}

impl<I: FuzzedInput> FuzzerState<I> {
    fn update_stats(&mut self) {
        let microseconds = self.world.elapsed_time();
        self.stats.exec_per_s =
            (((self.stats.total_number_of_runs as f64) / (microseconds as f64)) * 1_000_000.0) as usize;
        self.stats.pool_size = self.pool.size;
        self.stats.score = (self.pool.score() * 10.0).round() as usize;
        self.stats.avg_cplx = (self.pool.average_complexity * 10000.0).round() as usize;
    }

    fn receive_signal(&self, signal: i32) -> ! {
        self.world
            .report_event(FuzzerEvent::CaughtSignal(signal), Some(self.stats));

        match signal {
            4 | 6 | 10 | 11 | 8 => {
                let input = self.get_input();
                let cplx = input.complexity();
                let _ = self.world.save_artifact(&input.value, cplx);

                exit(FuzzerTerminationStatus::Crash as i32);
            }
            2 | 15 => exit(FuzzerTerminationStatus::Success as i32),
            _ => exit(FuzzerTerminationStatus::Unknown as i32),
        }
    }

    unsafe fn set_up_signal_handler(&self) {
        let ptr = NotThreadSafe(self as *const Self);
        handle_signals(vec![4, 6, 10, 11, 8, 2, 15], move |sig| (&*ptr.0).receive_signal(sig));
    }
}

pub struct Fuzzer<F, I>
where
    F: Fn(&I::Value) -> bool,
    I: FuzzedInput,
{
    state: FuzzerState<I>,
    test: F,
}

impl<F, I> Fuzzer<F, I>
where
    F: Fn(&I::Value) -> bool,
    I: FuzzedInput,
{
    pub fn new(test: F, settings: CommandLineArguments, world: World<I>) -> Self {
        Fuzzer {
            state: FuzzerState {
                pool: InputPool::new(),
                input_idx: FuzzerInputIndex::Temporary(UnifiedFuzzedInput::default()),
                stats: FuzzerStats::new(),
                settings,
                world,
            },
            test,
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
        input: &UnifiedFuzzedInput<I>,
        world: &World<I>,
        stats: FuzzerStats,
    ) -> Result<(), std::io::Error> {
        let sensor = shared_sensor();
        sensor.clear();

        sensor.is_recording = true;

        let cell = NotUnwindSafe { value: &test };
        let input_cell = NotUnwindSafe { value: &input.value };
        let result = catch_unwind(|| (cell.value)(input_cell.value));

        sensor.is_recording = false;

        if result.is_err() || !result.unwrap() {
            world.report_event(FuzzerEvent::TestFailure, Some(stats));
            let mut features: Vec<Feature> = Vec::new();
            sensor.iterate_over_collected_features(|f| features.push(f));
            world.save_artifact(&input.value, input.complexity())?;
            exit(FuzzerTerminationStatus::TestFailure as i32);
        }

        Ok(())
    }

    fn analyze(&mut self, cur_input_cplx: f64) -> Option<Vec<Feature>> {
        let mut best_input_for_a_feature = false;

        let sensor = shared_sensor();

        let mut score_estimate: f64 = 0.0;
        let mut score_to_exceed: f64 = core::f64::INFINITY;
        let mut matched_least_complex = false;

        sensor.iterate_over_collected_features(|feature| {
            let (predicted, least_complex) = self
                .state
                .pool
                .predicted_feature_score_and_least_complex_input_for_feature(feature);

            score_estimate += predicted;

            if let Some((old_cplx, cur_input_score)) = least_complex {
                if cur_input_cplx < old_cplx {
                    best_input_for_a_feature = true;
                } else if (cur_input_cplx - old_cplx).abs() < std::f64::EPSILON {
                    matched_least_complex = true;
                    score_to_exceed = score_to_exceed.min(cur_input_score);
                }
            } else {
                best_input_for_a_feature = true;
            }
        });

        if best_input_for_a_feature || (matched_least_complex && score_estimate > score_to_exceed) {
            let mut features: Vec<Feature> = Vec::new();
            sensor.iterate_over_collected_features(|feature| {
                features.push(feature);
            });
            Some(features)
        } else {
            None
        }
    }

    fn test_input_and_analyze(&mut self) -> Result<(), std::io::Error> {
        let input = self.state.get_input();
        let cplx = input.complexity();
        Self::test_input(&self.test, &input, &self.state.world, self.state.stats)?;
        self.state.stats.total_number_of_runs += 1;

        if let Some(features) = self.analyze(cplx) {
            let input_cloned = self.state.get_input().new_source();
            let actions = self.state.pool.add(input_cloned, features);
            self.state.world.do_actions(actions)?;
            self.state.update_stats();
            self.state.world.report_event(FuzzerEvent::New, Some(self.state.stats));

            Ok(())
        } else {
            Ok(())
        }
    }

    fn process_next_inputs(&mut self) -> Result<(), std::io::Error> {
        let idx = self.state.pool.random_index();
        self.state.input_idx = FuzzerInputIndex::Pool(idx);
        let input = self.state.pool.get(idx);
        // let cloned_input = input.clone();

        let mutate_token = input.mutate(self.state.settings.max_input_cplx);
        let cplx = input.complexity();

        if cplx < self.state.settings.max_input_cplx {
            self.test_input_and_analyze()?;
        }
        if let Some(input) = self.state.pool.get_opt(idx) {
            input.unmutate(mutate_token);
        // assert_eq!(input.value, cloned_input.value);
        // assert_eq!(cloned_input.complexity(), input.complexity());
        } else {
            // println!("deleted the source input");
        }

        Ok(())
    }

    fn process_initial_inputs(&mut self) -> Result<(), std::io::Error> {
        let mut inputs: Vec<UnifiedFuzzedInput<I>> = self
            .state
            .world
            .read_input_corpus()
            .unwrap_or_default()
            .into_iter()
            .map(|value| {
                let state = I::state_from_value(&value);
                UnifiedFuzzedInput { value, state }
            })
            .collect();

        if inputs.is_empty() {
            for i in 0..100 {
                let v = I::arbitrary(i, self.state.settings.max_input_cplx);
                let v_state = I::state_from_value(&v);
                inputs.push(UnifiedFuzzedInput::new((v, v_state)));
            }
        }
        inputs.push(UnifiedFuzzedInput::default());
        inputs.drain_filter(|i| i.complexity() > self.state.settings.max_input_cplx);
        assert!(!inputs.is_empty());
        for input in inputs {
            self.state.input_idx = FuzzerInputIndex::Temporary(input);
            self.test_input_and_analyze()?;
        }

        Ok(())
    }

    fn main_loop(&mut self) -> Result<(), std::io::Error> {
        self.state.world.set_start_time();
        self.state
            .world
            .report_event(FuzzerEvent::Start, Some(self.state.stats));
        self.process_initial_inputs()?;
        self.state
            .world
            .report_event(FuzzerEvent::DidReadCorpus, Some(self.state.stats));

        while self.state.stats.total_number_of_runs < self.max_iter() {
            self.process_next_inputs()?;
        }
        self.state.world.report_event(FuzzerEvent::Done, Some(self.state.stats));

        Ok(())
    }

    fn corpus_minifying_loop(&mut self) -> Result<(), std::io::Error> {
        self.state.world.set_start_time();
        self.state
            .world
            .report_event(FuzzerEvent::Start, Some(self.state.stats));
        self.process_initial_inputs()?;
        self.state
            .world
            .report_event(FuzzerEvent::DidReadCorpus, Some(self.state.stats));
        while self.state.pool.size > self.state.settings.corpus_size {
            let actions = self.state.pool.remove_lowest();
            self.state.world.do_actions(actions)?;
            self.state.update_stats();
        }
        self.state.world.report_event(FuzzerEvent::Done, Some(self.state.stats));
        Ok(())
    }

    fn input_minifying_loop(&mut self) -> Result<(), std::io::Error> {
        self.state.world.set_start_time();

        self.state
            .world
            .report_event(FuzzerEvent::Start, Some(self.state.stats));
        let value = self.state.world.read_input_file()?; // TODO: world should return a UnifiedFuzzedInput?
        let state = I::state_from_value(&value);
        let input = UnifiedFuzzedInput { value, state };
        let input_cplx = input.complexity();
        self.state.settings.max_input_cplx = input_cplx - 0.01;

        self.state.pool.add_favored_input(input);

        loop {
            self.process_next_inputs()?;
        }
    }
}

pub fn launch<F, I>(test: F) -> Result<(), std::io::Error>
where
    F: Fn(&I::Value) -> bool,
    I: FuzzedInput,
{
    let env_args: Vec<_> = std::env::args().collect();
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

    let args = match CommandLineArguments::from_parser(&parser, &env_args[1..], DEFAULT_ARGUMENTS) {
        Ok(r) => r,
        Err(e) => {
            println!("{}\n\n{}", e, help);
            std::process::exit(1);
        }
    };

    let command = args.command;

    let mut fuzzer = Fuzzer::<F, I>::new(test, args.clone(), World::new(args));
    unsafe { fuzzer.state.set_up_signal_handler() };
    match command {
        FuzzerCommand::Fuzz => fuzzer.main_loop()?,
        FuzzerCommand::MinifyInput => fuzzer.input_minifying_loop()?,
        FuzzerCommand::Read => {
            let value = fuzzer.state.world.read_input_file()?;
            let state = I::state_from_value(&value);
            fuzzer.state.input_idx = FuzzerInputIndex::Temporary(UnifiedFuzzedInput::new((value, state)));
            let input = fuzzer.state.get_input();
            Fuzzer::test_input(&fuzzer.test, &input, &fuzzer.state.world, fuzzer.state.stats)?;
        }
        FuzzerCommand::MinifyCorpus => fuzzer.corpus_minifying_loop()?,
    };
    Ok(())
}

struct NotThreadSafe<T>(T);
struct NotUnwindSafe<T> {
    value: T,
}

unsafe impl<T> Send for NotThreadSafe<T> {}
impl<T> UnwindSafe for NotUnwindSafe<T> {}
impl<T> RefUnwindSafe for NotUnwindSafe<T> {}

pub enum FuzzerTerminationStatus {
    Success = 0,
    Crash = 1,
    TestFailure = 2,
    Unknown = 3,
}
