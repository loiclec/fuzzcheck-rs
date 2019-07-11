
use crate::input::*;
use crate::input_pool::*;
use crate::world::*;
use crate::code_coverage_sensor::*;
use crate::artifact::*;

pub type FuzzerSettings = bool;

pub enum FuzzerTerminationStatus {
    Success = 0,
    Crash = 1,
    TestFailure = 2,
    Unknown = 3
}

struct FuzzerState <'a, Input, Properties, World> 
    where
    Input: FuzzerInput, 
    Properties: InputProperties<Input=Input>, 
    World: FuzzerWorld<Input=Input, Properties=Properties>
{
    pool: InputPool<Input>,
    inputs: Vec<Input>,
    input: &'a Input,
    stats: FuzzerStats,
    settings: FuzzerSettings,
    world: World
    //stats, settings, process_start_time
}

impl<Input, Properties, World> FuzzerState<'_, Input, Properties, World>
    where
    Input: FuzzerInput, 
    Properties: InputProperties<Input=Input>, 
    World: FuzzerWorld<Input=Input, Properties=Properties>
{
    fn receive_signal(&self, signal: Signal) -> ! {
        self.world.report_event(FuzzerEvent::CaughtSignal(signal), self.stats);
        // TODO
        std::process::exit(1);
    }
}

struct Fuzzer<'a, Input, Generator, World, TestF> 
    where
    Input: FuzzerInput, 
    Generator: InputGenerator<Input=Input>, 
    World: FuzzerWorld<Input=Input, Properties=Generator>,
    TestF: Fn(&Input) -> bool
{
    state: FuzzerState<'a, Input, Generator, World>,
    generator: Generator,
    test: TestF
    // signals_handler
}

impl<Input, Generator, World, TestF> Fuzzer<'_, Input, Generator, World, TestF> 
    where
    Input: FuzzerInput, 
    Generator: InputGenerator<Input=Input>, 
    World: FuzzerWorld<Input=Input, Properties=Generator>,
    TestF: Fn(&Input) -> bool
{
    fn test_input(&mut self, i: usize) {
        let sensor = shared_sensor();
        sensor.clear();
        let input = &self.state.inputs[i];
        sensor.is_recording = true;
        let success = (self.test)(input);
        sensor.is_recording = false;

        if !success {
            self.state.world.report_event(FuzzerEvent::TestFailure, self.state.stats); /* TODO */
            let mut features: Vec<Feature> = Vec::new();
            sensor.iterate_over_collected_features(|f| features.push(f)); // TODO use iterator?
            self.state.world.save_artifact(input, Some(features), ArtifactKind::TestFailure); /* TODO */
            std::process::exit(FuzzerTerminationStatus::TestFailure as i32);
        }
        // self.state.stats.total_number_of_runs += 1;
    }

    fn test_current_inputs(&mut self) {
        for i in 0 .. self.state.inputs.len() {
            self.test_input(i);
        }
    }

    fn analyze(&mut self) -> Option<InputPoolElement<Input>> {
        let mut features: Vec<Feature> = Vec::new();
        
        let mut best_input_for_a_feature = false;
        
        let cur_input_cplx = Generator::adjusted_complexity(self.state.input);
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
                self.state.input.clone(),
                cur_input_cplx,
                features
            ))
        } else {
            None
        }
    }
    
}
