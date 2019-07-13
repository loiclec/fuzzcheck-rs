use crate::artifact::*;
use crate::code_coverage_sensor::*;
use crate::command_line::*;
use crate::input::*;
use crate::input_pool::*;
use crate::signals_handler::*;
use crate::structopt::StructOpt;
use crate::world::*;
use std::result::Result;

struct NotThreadSafe<T>(T);
struct NotUnwindSafe<T> {
    value: T
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

struct FuzzerState<Input, Properties, World>
where
    Input: FuzzerInput,
    Properties: InputProperties<Input = Input>,
    World: FuzzerWorld<Input = Input, Properties = Properties>,
{
    pool: InputPool<Input>,
    inputs: Vec<Input>,
    input_idx: usize,
    stats: FuzzerStats,
    settings: FuzzerSettings,
    world: World,
    process_start_time: usize,
}

impl<Input, Properties, World> FuzzerState<Input, Properties, World>
where
    Input: FuzzerInput,
    Properties: InputProperties<Input = Input>,
    World: FuzzerWorld<Input = Input, Properties = Properties>,
{
    fn update_stats(&mut self) {
        let microseconds = self.world.elapsed_time();
        self.stats.exec_per_s = (((self.stats.total_number_of_runs as f64) / (microseconds as f64)) * 1_000_000.0) as usize;
        self.stats.pool_size = self.pool.inputs.len();
        self.stats.score = (self.pool.score * 10.0).round() as usize;
        let avg_cplx = self
            .pool
            .smallest_input_complexity_for_feature
            .values()
            .fold(0.0, |x, n| x + n)
            / (self.pool.smallest_input_complexity_for_feature.len() as f64);
        self.stats.avg_cplx = (avg_cplx * 100.0).round() as usize;
    }

    fn receive_signal(&self, signal: i32) -> ! {
        self.world
            .report_event(FuzzerEvent::CaughtSignal(signal), Some(self.stats));

        match signal {
            4 | 6 | 10 | 11 | 8 => {
                let _ = self.world
                    .save_artifact(&self.inputs[self.input_idx], ArtifactKind::Crash);
                std::process::exit(FuzzerTerminationStatus::Crash as i32);
            }
            2 | 15 => {
                std::process::exit(FuzzerTerminationStatus::Success as i32)
            },
            _ => std::process::exit(FuzzerTerminationStatus::Unknown as i32),
        }
    }

    unsafe fn set_up_signal_handler(&self) {
        let ptr = NotThreadSafe(self as *const Self);
        handle_signals(vec![4, 6, 10, 11, 8, 2, 15], move |sig| {
            (&*ptr.0).receive_signal(sig)
        });
    }
}

pub struct Fuzzer<Input, Generator, World, TestF>
where
    Input: FuzzerInput,
    Generator: InputGenerator<Input = Input>,
    World: FuzzerWorld<Input = Input, Properties = Generator>,
    TestF: Fn(&Input) -> bool,
{
    state: FuzzerState<Input, Generator, World>,
    generator: Generator,
    test: TestF,
}

impl<Input, Generator, World, TestF> Fuzzer<Input, Generator, World, TestF>
where
    Input: FuzzerInput,
    Generator: InputGenerator<Input = Input>,
    World: FuzzerWorld<Input = Input, Properties = Generator>,
    TestF: Fn(&Input) -> bool,
{
    pub fn new(
        test: TestF,
        generator: Generator,
        settings: FuzzerSettings,
        world: World,
    ) -> Fuzzer<Input, Generator, World, TestF> {
        Fuzzer {
            state: FuzzerState {
                pool: InputPool::new(),
                inputs: vec![generator.base_input()],
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
}

impl<Input, Generator, World, TestF> Fuzzer<Input, Generator, World, TestF>
where
    Input: FuzzerInput,
    Generator: InputGenerator<Input = Input>,
    World: FuzzerWorld<Input = Input, Properties = Generator>,
    TestF: Fn(&Input) -> bool,
{
    fn test_input(&mut self, i: usize) -> Result<(), std::io::Error> {
        let sensor = shared_sensor();
        sensor.clear();
        let input = &self.state.inputs[i];
        sensor.is_recording = true;
        
        let cell = NotUnwindSafe {
            value: &self,
        };
        let input_cell = NotUnwindSafe {
            value: input,
        };
        let result = std::panic::catch_unwind(|| {
            (cell.value.test)(input_cell.value)
        });
        sensor.is_recording = false;

        if result.is_err() || !result.unwrap() {
            self.state
                .world
                .report_event(FuzzerEvent::TestFailure, Some(self.state.stats));
            let mut features: Vec<Feature> = Vec::new();
            sensor.iterate_over_collected_features(|f| features.push(f)); // TODO use iterator?
            self.state.world.save_artifact(input, ArtifactKind::TestFailure)?;
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

    fn analyze(&mut self) -> Option<InputPoolElement<Input>> {
        let mut features: Vec<Feature> = Vec::new();

        let mut best_input_for_a_feature = false;

        let cur_input_cplx = Generator::adjusted_complexity(&self.state.inputs[self.state.input_idx]);
        let sensor = shared_sensor();
        sensor.iterate_over_collected_features(|feature| {
            if let Some(old_cplx) = self.state.pool.smallest_input_complexity_for_feature.get(&feature) {
                if cur_input_cplx < *old_cplx {
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
        let mut new_pool_elements: Vec<InputPoolElement<Input>> = Vec::new();
        for idx in 0 .. self.state.inputs.len() {
            self.state.input_idx = idx;
            self.test_input(idx)?;
            if let Some(new_pool_element) = self.analyze() {
                new_pool_elements.push(new_pool_element);
            }
        }
        if new_pool_elements.is_empty() {
            return Ok(());
        }
        let effect = self.state.pool.add::<World>(new_pool_elements);
        effect(&mut self.state.world);

        self.state.update_stats();
        self.state.world.report_event(FuzzerEvent::New, Some(self.state.stats));

        Ok(())
    }

    fn process_next_inputs(&mut self) -> Result<(), std::io::Error> {
        self.state.inputs.clear();
        self.state.input_idx = 0;

        while self.state.inputs.len() < 10 {
            let idx = self.state.pool.random_index(self.state.world.rand());
            let pool_element = self.state.pool.get(idx);
            let mut new_input = pool_element.input.clone();

            let mut cplx = pool_element.complexity - 1.0; // TODO: why - 1.0?
            for _ in 0..self.state.settings.mutate_depth {
                if self.state.stats.total_number_of_runs >= self.state.settings.max_nbr_of_runs
                    || !self.generator.mutate(
                        &mut new_input,
                        self.state.settings.max_input_cplx - cplx,
                        self.state.world.rand(),
                    )
                {
                    break;
                }
                cplx = Generator::complexity(&new_input);
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
                    .initial_inputs(self.state.settings.max_input_cplx, self.state.world.rand()),
            );
        }
        inputs.drain_filter(|x| Generator::complexity(x) > self.state.settings.max_input_cplx);

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

        while self.state.stats.total_number_of_runs < self.state.settings.max_nbr_of_runs {
            self.process_next_inputs()?;
        }
        self.state
            .world
            .report_event(FuzzerEvent::Done, Some(self.state.stats));

        Ok(())
    }

    fn minimize_loop(&mut self) -> Result<(), std::io::Error> {
        // TODO: change name of this function
        self.state.world.start_process();

        self.state.world.report_event(FuzzerEvent::Start, Some(self.state.stats));
        let input = self.state.world.read_input_file()?;
        
        let complexity = Generator::complexity(&input);
        let adjusted_complexity = Generator::adjusted_complexity(&input);

        let favored_input = InputPoolElement::new(input, adjusted_complexity, vec![]);
        self.state.pool.favored_input = Some(favored_input);
        let effect = self.state.pool.update_scores();
        effect(&mut self.state.world);
        self.state.settings.max_input_cplx = complexity - 0.01;
        while self.state.stats.total_number_of_runs < self.state.settings.max_nbr_of_runs {
            self.process_next_inputs()?;
        }
        self.state.world.report_event(FuzzerEvent::Done, Some(self.state.stats));
        Ok(())
    }
}

pub enum CommandLineFuzzer<G, F> 
where
    G: InputGenerator,
    F: Fn(&G::Input) -> bool
{
    Phantom(std::marker::PhantomData<G>, std::marker::PhantomData<F>)
}
impl<G, F> CommandLineFuzzer<G, F> 
where
    G: InputGenerator,
    F: Fn(&G::Input) -> bool
{
    pub fn launch(test: F, generator: G) -> Result<(), std::io::Error> {
        let args = CommandLineArguments::from_args();
        
        let settings = args.settings;
        let world_info = args.world_info;
        
        let command = settings.command;
        

        let mut fuzzer = Fuzzer::new(test, generator, settings, CommandLineFuzzerWorld::new(world_info));
        unsafe { fuzzer.state.set_up_signal_handler() };
        match command {
            FuzzerCommand::Fuzz => &fuzzer.main_loop()?,
            _ => unimplemented!("only fuzz command is supported for now"),
        };
        Ok(())
    }
}
