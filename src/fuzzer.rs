use crate::code_coverage_sensor::*;
use crate::command_line::*;
use crate::input::*;
use crate::input_pool::*;
use crate::signals_handler::*;
use crate::world::*;

use std::result::Result;

use rand::seq::SliceRandom;
use rand::thread_rng;

struct NotThreadSafe<T>(T);
struct NotUnwindSafe<T> {
    value: T,
}

unsafe impl<T> Send for NotThreadSafe<T> {}
impl<T> std::panic::UnwindSafe for NotUnwindSafe<T> {}
impl<T> std::panic::RefUnwindSafe for NotUnwindSafe<T> {}

pub enum FuzzerTerminationStatus {
    Success = 0,
    Crash = 1,
    TestFailure = 2,
    Unknown = 3,
}

struct FuzzerState<T, G>
where
    T: Clone,
    G: InputGenerator<Input = T>,
{
    pool: InputPool<T>,
    inputs: Vec<T>,
    input_idx: usize,
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
        self.stats.pool_size = self.pool.inputs.len();
        self.stats.score = (self.pool.score * 10.0).round() as usize;
        let avg_cplx: f64 = 0.0;
        self.stats.avg_cplx = (avg_cplx * 100.0).round() as usize;
    }

    fn receive_signal(&self, signal: i32) -> ! {
        self.world
            .report_event(FuzzerEvent::CaughtSignal(signal), Some(self.stats));

        match signal {
            4 | 6 | 10 | 11 | 8 => {
                let input = &self.inputs[self.input_idx];
                let _ = self.world.save_artifact(input, G::complexity(input));

                std::process::exit(FuzzerTerminationStatus::Crash as i32);
            }
            2 | 15 => std::process::exit(FuzzerTerminationStatus::Success as i32),
            _ => std::process::exit(FuzzerTerminationStatus::Unknown as i32),
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
                inputs: vec![],
                input_idx: 0,
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

    fn test_input(&mut self, i: usize) -> Result<(), std::io::Error> {
        let sensor = shared_sensor();
        sensor.clear();
        let input = &self.state.inputs[i];
        sensor.is_recording = true;

        let cell = NotUnwindSafe { value: &self };
        let input_cell = NotUnwindSafe { value: input };
        let result = std::panic::catch_unwind(|| (cell.value.test)(input_cell.value));
        sensor.is_recording = false;

        if result.is_err() || !result.unwrap() {
            self.state
                .world
                .report_event(FuzzerEvent::TestFailure, Some(self.state.stats));
            let mut features: Vec<Feature> = Vec::new();
            sensor.iterate_over_collected_features(|f| features.push(f));
            self.state.world.save_artifact(input, G::complexity(&input))?;
            std::process::exit(FuzzerTerminationStatus::TestFailure as i32);
        }
        self.state.stats.total_number_of_runs += 1;
        Ok(())
    }

    fn test_current_inputs(&mut self) -> Result<(), std::io::Error> {
        for i in 0..self.state.inputs.len() {
            self.test_input(i)?;
        }
        Ok(())
    }

    fn analyze(&mut self) -> Option<InputPoolElement<T>> {
        let mut features: Vec<Feature> = Vec::new();

        let mut best_input_for_a_feature = false;

        let cur_input_cplx = G::adjusted_complexity(&self.state.inputs[self.state.input_idx]);
        let sensor = shared_sensor();
        sensor.iterate_over_collected_features( |feature| {
            // TODO: here I use the elements_for_feature instead
            // and maybe I can save the references and pass it to the `add` function to speed up things
            // and I could even estimate the score of the input!
            // it won't be correct but may be good enough
            if let Some(old_input_idx) = self.state.pool.inputs_of_feature.entry(feature.clone()).or_default().last() {
                let old_cplx = self.state.pool.inputs[*old_input_idx].as_ref().unwrap().complexity;
                if cur_input_cplx < old_cplx {
                    best_input_for_a_feature = true;
                }
            } else {
                best_input_for_a_feature = true;
            }
            features.push(feature);
        });
        if best_input_for_a_feature {
            Some(InputPoolElement::new(
                self.state.inputs[self.state.input_idx].clone(),
                cur_input_cplx,
                features,
            ))
        } else {
            None
        }
    }

    fn process_current_inputs(&mut self) -> Result<(), std::io::Error> {
        let mut new_pool_elements: Vec<InputPoolElement<T>> = Vec::new();
        for idx in 0..self.state.inputs.len() {
            self.state.input_idx = idx;
            self.test_input(idx)?;
            if let Some(new_pool_element) = self.analyze() {
                new_pool_elements.push(new_pool_element);
            }
        }
        if new_pool_elements.is_empty() {
            return Ok(());
        }
        // TODO: not ideal
        for e in new_pool_elements {
            let effect = self.state.pool.add(e);
            effect(&mut self.state.world)?;
        }

        self.state.update_stats();
        self.state.world.report_event(FuzzerEvent::New, Some(self.state.stats));

        Ok(())
    }

    fn process_next_inputs(&mut self) -> Result<(), std::io::Error> {
        self.state.inputs.clear();
        self.state.input_idx = 0;
        while self.state.inputs.len() < 10 {
            let idx = self.state.pool.random_index();
            let pool_element = self.state.pool.get(idx);
            let mut new_input = pool_element.input.clone();

            let mut cplx = pool_element.complexity - 1.0; // TODO: why - 1.0?
            for _ in 0..self.state.settings.mutate_depth {
                if self.state.stats.total_number_of_runs >= self.max_iter()
                    || !self.generator.mutate(
                        &mut new_input,
                        self.state.settings.max_input_cplx - cplx
                    )
                {
                    break;
                }
                cplx = G::complexity(&new_input);
                if cplx >= self.state.settings.max_input_cplx {
                    continue;
                }
                self.state.inputs.push(new_input.clone());
            }
            self.process_current_inputs()?;
        }
        Ok(())
    }

    fn process_initial_inputs(&mut self) -> Result<(), std::io::Error> {
        let mut inputs = self.state.world.read_input_corpus().unwrap_or_default();
        if inputs.is_empty() {
            inputs.append(
                &mut self
                    .generator
                    .initial_inputs(self.state.settings.max_input_cplx),
            );
        }
        inputs.drain_filter(|x| G::complexity(x) > self.state.settings.max_input_cplx);

        self.state.inputs.append(&mut inputs);
        self.state.input_idx = 0;
        self.process_current_inputs()?;
        Ok(())
    }

    fn main_loop(&mut self) -> Result<(), std::io::Error> {
        self.state.world.start_process();
        self.state
            .world
            .report_event(FuzzerEvent::Start, Some(self.state.stats));
        self.process_initial_inputs()?;
        self.state
            .world
            .report_event(FuzzerEvent::DidReadCorpus, Some(self.state.stats));

        // TODO: explain reset_pool_time
        while self.state.stats.total_number_of_runs < self.max_iter() {
            self.process_next_inputs()?;
        }
        self.state.world.report_event(FuzzerEvent::Done, Some(self.state.stats));

        Ok(())
    }

    fn shrink_loop(&mut self) -> Result<(), std::io::Error> {
        self.state.world.start_process();
        self.state
            .world
            .report_event(FuzzerEvent::Start, Some(self.state.stats));
        self.process_initial_inputs()?;
        self.state
            .world
            .report_event(FuzzerEvent::DidReadCorpus, Some(self.state.stats));
        while self.state.pool.inputs.len() > self.state.settings.corpus_size {
            let effect = self.state.pool.remove_lowest();
            effect(&mut self.state.world)?;
            self.state.update_stats();
        }
        self.state.world.report_event(FuzzerEvent::Done, Some(self.state.stats));
        Ok(())

    }

    fn minimize_loop(&mut self) -> Result<(), std::io::Error> {
        // TODO: change name of this function
        self.state.world.start_process();

        self.state
            .world
            .report_event(FuzzerEvent::Start, Some(self.state.stats));
        let input = self.state.world.read_input_file()?;

        let complexity = G::complexity(&input);

        let adjusted_complexity = G::adjusted_complexity(&input);

        let favored_input = InputPoolElement::new(input, adjusted_complexity, vec![]);
        self.state.pool.favored_input = Some(favored_input);
        // TODO: proper handling of favored_input
        // let effect = self.state.pool.update_scores();
        // effect(&mut self.state.world)?;
        self.state.settings.max_input_cplx = complexity - 0.01;
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
    let app = setup_app();

    let args = CommandLineArguments::from_arg_matches(&app.get_matches());

    let command = args.command;

    let mut fuzzer = Fuzzer::new(test, generator, args.clone(), World::new(args));
    unsafe { fuzzer.state.set_up_signal_handler() };
    match command {
        FuzzerCommand::Fuzz => fuzzer.main_loop()?,
        FuzzerCommand::Minimize => fuzzer.minimize_loop()?,
        FuzzerCommand::Read => {
            fuzzer.state.inputs = vec![fuzzer.state.world.read_input_file()?];
            fuzzer.state.input_idx = 0;
            fuzzer.test_current_inputs()?;
        },
        FuzzerCommand::Shrink => fuzzer.shrink_loop()?,
    };
    Ok(())
}
