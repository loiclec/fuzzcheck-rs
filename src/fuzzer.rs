extern crate signal_hook;

use crate::artifact::*;
use crate::code_coverage_sensor::*;
use crate::input::*;
use crate::input_pool::*;
use crate::world::*;

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
        let now = self.world.clock();
        let seconds = (now - self.process_start_time) / 1_000_000;
        self.stats.exec_per_s = self.stats.total_number_of_runs / seconds;
        self.stats.pool_size = self.pool.inputs.len();
        self.stats.score = (self.pool.score * 10.0).round() as usize;
        let avg_cplx = self
            .pool
            .smallest_input_complexity_for_feature
            .values()
            .fold(0.0, |x, n| x + n);
        self.stats.avg_cplx = (avg_cplx * 100.0).round() as usize;
    }

    fn receive_signal(&self, signal: Signal) -> ! {
        self.world.report_event(FuzzerEvent::CaughtSignal(signal), &self.stats);
        // TODO
        std::process::exit(1);
    }
}

struct Fuzzer<Input, Generator, World, TestF>
where
    Input: FuzzerInput,
    Generator: InputGenerator<Input = Input>,
    World: FuzzerWorld<Input = Input, Properties = Generator>,
    TestF: Fn(&Input) -> bool,
{
    state: FuzzerState<Input, Generator, World>,
    generator: Generator,
    test: TestF, // signals_handler
}

impl<Input, Generator, World, TestF> Fuzzer<Input, Generator, World, TestF>
where
    Input: FuzzerInput,
    Generator: InputGenerator<Input = Input>,
    World: FuzzerWorld<Input = Input, Properties = Generator>,
    TestF: Fn(&Input) -> bool,
{
    fn test_input(&mut self, i: usize) {
        let sensor = shared_sensor();
        sensor.clear();
        let input = &self.state.inputs[i];
        sensor.is_recording = true;
        let success = (self.test)(input);
        sensor.is_recording = false;

        if !success {
            self.state
                .world
                .report_event(FuzzerEvent::TestFailure, &self.state.stats);
            let mut features: Vec<Feature> = Vec::new();
            sensor.iterate_over_collected_features(|f| features.push(f)); // TODO use iterator?
            self.state
                .world
                .save_artifact(input, Some(features), ArtifactKind::TestFailure); /* TODO */
            std::process::exit(FuzzerTerminationStatus::TestFailure as i32);
        }
        // self.state.stats.total_number_of_runs += 1;
    }

    fn test_current_inputs(&mut self) {
        for i in 0..self.state.inputs.len() {
            self.test_input(i);
        }
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

    fn process_current_inputs(&mut self) {
        let mut new_pool_elements: Vec<InputPoolElement<Input>> = Vec::new();
        for idx in 0..self.state.inputs.len() {
            self.state.input_idx = idx;
            self.test_input(idx);
            if let Some(new_pool_element) = self.analyze() {
                new_pool_elements.push(new_pool_element);
            }
        }
        if new_pool_elements.is_empty() {
            return;
        }
        let effect = self.state.pool.add::<World>(new_pool_elements);
        effect(&mut self.state.world);

        // TODO: self.state.update_stats();
        self.state.world.report_event(FuzzerEvent::New, &self.state.stats);
    }

    fn process_next_inputs(&mut self) {
        self.state.inputs.clear();
        self.state.input_idx = 0;

        while self.state.inputs.len() < 50 {
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
            self.process_current_inputs();
        }
    }

    fn process_initial_inputs(&mut self) {
        let mut inputs = self.state.world.read_input_corpus();
        if inputs.is_empty() {
            inputs.append(
                &mut self
                    .generator
                    .initial_inputs(self.state.settings.max_input_cplx, self.state.world.rand()),
            );
        }
        inputs.drain_filter(|x| Generator::complexity(x) <= self.state.settings.max_input_cplx);
        self.state.inputs = inputs;
        self.state.input_idx = 0;
        self.process_current_inputs();
    }

    fn main_loop(&mut self) {
        self.state.process_start_time = self.state.world.clock();
        self.state.world.report_event(FuzzerEvent::Start, &self.state.stats);
        self.process_initial_inputs();
        self.state
            .world
            .report_event(FuzzerEvent::DidReadCorpus, &self.state.stats);

        while self.state.stats.total_number_of_runs < self.state.settings.max_nbr_of_runs {
            self.process_next_inputs();
        }
        self.state.world.report_event(FuzzerEvent::Done, &self.state.stats);
    }

    // TODO: minimizing loop
}
