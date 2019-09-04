use crate::code_coverage_sensor::*;
use crate::input::*;
use crate::input_pool::*;
use crate::signals_handler::*;
use crate::world::*;

use fuzzcheck_arg_parser::*;

use std::panic::{catch_unwind, RefUnwindSafe, UnwindSafe};
use std::process::exit;
use std::result::Result;

struct FuzzerState<T, G>
where
    T: Clone,
    G: InputGenerator<Input = T>,
{
    pool: InputPool<T>,
    input: T,
    stats: FuzzerStats,
    settings: CommandLineArguments,
    world: World<T, G>,
    process_start_time: usize,
}

impl<T, G> FuzzerState<T, G>
where
    T: Clone,
    G: InputGenerator<Input = T>,
{
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
                let _ = self.world.save_artifact(&self.input, G::complexity(&self.input));

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

pub struct Fuzzer<T, F, G>
where
    T: Clone,
    F: Fn(&T) -> bool,
    G: InputGenerator<Input = T>,
{
    state: FuzzerState<T, G>,
    generator: G,
    test: F,
}

impl<T, F, G> Fuzzer<T, F, G>
where
    T: Clone,
    F: Fn(&T) -> bool,
    G: InputGenerator<Input = T>,
{
    pub fn new(test: F, generator: G, settings: CommandLineArguments, world: World<T, G>) -> Self {
        Fuzzer {
            state: FuzzerState {
                pool: InputPool::new(),
                input: G::base_input(),
                stats: FuzzerStats::new(),
                settings,
                world,
                process_start_time: 0,
            },
            generator,
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

    fn test_input(&mut self) -> Result<(), std::io::Error> {
        let sensor = shared_sensor();
        sensor.clear();

        sensor.is_recording = true;

        let cell = NotUnwindSafe { value: &self };
        let input_cell = NotUnwindSafe {
            value: &self.state.input,
        };
        let result = catch_unwind(|| (cell.value.test)(input_cell.value));

        sensor.is_recording = false;

        if result.is_err() || !result.unwrap() {
            self.state
                .world
                .report_event(FuzzerEvent::TestFailure, Some(self.state.stats));
            let mut features: Vec<Feature> = Vec::new();
            sensor.iterate_over_collected_features(|f| features.push(f));
            self.state
                .world
                .save_artifact(&self.state.input, G::complexity(&self.state.input))?;
            exit(FuzzerTerminationStatus::TestFailure as i32);
        }
        self.state.stats.total_number_of_runs += 1;
        Ok(())
    }

    fn analyze(&mut self) -> Option<(f64, Vec<Feature>)> {
        let mut features: Vec<Feature> = Vec::new();

        let mut best_input_for_a_feature = false;

        let cur_input_cplx = G::complexity(&self.state.input);
        let sensor = shared_sensor();

        let mut score_estimate: f64 = 0.0;
        let mut score_to_exceed: f64 = core::f64::INFINITY;
        let mut matched_least_complex = false;

        sensor.iterate_over_collected_features(|feature| {
            score_estimate += self.state.pool.predicted_feature_score(feature);

            if let Some((old_cplx, cur_input_score)) = self.state.pool.least_complex_input_for_feature(feature) {
                if cur_input_cplx < old_cplx {
                    best_input_for_a_feature = true;
                } else if (cur_input_cplx - old_cplx).abs() < std::f64::EPSILON {
                    matched_least_complex = true;
                    score_to_exceed = score_to_exceed.min(cur_input_score);
                }
            } else {
                best_input_for_a_feature = true;
            }
            features.push(feature);
        });

        if best_input_for_a_feature || (matched_least_complex && score_estimate > score_to_exceed) {
            Some((cur_input_cplx, features))
        } else {
            None
        }
    }

    fn test_input_and_analyze(&mut self) -> Result<(), std::io::Error> {
        self.test_input()?;

        if let Some((cplx, input)) = self.analyze() {
            let actions = self.state.pool.add(self.state.input.clone(), cplx, input);
            self.state.world.do_actions(actions)?;
        } else {
            return Ok(());
        }

        self.state.update_stats();
        self.state.world.report_event(FuzzerEvent::New, Some(self.state.stats));

        Ok(())
    }

    fn process_next_inputs(&mut self) -> Result<(), std::io::Error> {
        let idx = self.state.pool.random_index();
        let i = self.state.pool.get(idx);
        self.state.input = i;
        let mut cplx = G::complexity(&self.state.input);
        for _ in 0..self.state.settings.mutate_depth {
            if self.state.stats.total_number_of_runs >= self.max_iter()
                || !self
                    .generator
                    .mutate(&mut self.state.input, self.state.settings.max_input_cplx - cplx)
            {
                break;
            }
            cplx = G::complexity(&self.state.input);
            if cplx >= self.state.settings.max_input_cplx {
                continue;
            } else {
                self.test_input_and_analyze()?;
            }
        }

        Ok(())
    }

    fn process_initial_inputs(&mut self) -> Result<(), std::io::Error> {
        let mut inputs = self.state.world.read_input_corpus().unwrap_or_default();
        if inputs.is_empty() {
            inputs.append(&mut self.generator.initial_inputs(self.state.settings.max_input_cplx));
        }
        inputs.drain_filter(|x| G::complexity(x) > self.state.settings.max_input_cplx);

        for input in inputs {
            self.state.input = input;
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

    fn shrink_loop(&mut self) -> Result<(), std::io::Error> {
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
        let input = self.state.world.read_input_file()?;

        self.state.settings.max_input_cplx = G::complexity(&input) - 0.01;

        self.state.pool.add_favored_input(input.clone());

        loop {
            self.process_next_inputs()?;
        }
    }
}

pub fn launch<T, F, G>(test: F, generator: G) -> Result<(), std::io::Error>
where
    T: Clone,
    F: Fn(&T) -> bool,
    G: InputGenerator<Input = T>,
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

    let mut fuzzer = Fuzzer::new(test, generator, args.clone(), World::new(args));
    unsafe { fuzzer.state.set_up_signal_handler() };
    match command {
        FuzzerCommand::Fuzz => fuzzer.main_loop()?,
        FuzzerCommand::MinifyInput => fuzzer.input_minifying_loop()?,
        FuzzerCommand::Read => {
            fuzzer.state.input = fuzzer.state.world.read_input_file()?;
            fuzzer.test_input()?;
        }
        FuzzerCommand::MinifyCorpus => fuzzer.shrink_loop()?,
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
