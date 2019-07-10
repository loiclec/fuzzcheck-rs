
use crate::fuzzer::input::*;
use crate::fuzzer::input_pool::*;
use crate::fuzzer::world::*;

pub type FuzzerSettings = bool;

pub enum FuzzerTerminationStatus {
    Success = 0,
    Crash = 1,
    TestFailure = 2,
    Unknown = 3
}

struct FuzzerState <Input, Properties, World> 
    where
    Input: FuzzerInput, 
    Properties: InputProperties<Input=Input>, 
    World: FuzzerWorld<Input=Input, Properties=Properties>
{
    pool: InputPool<Input>,
    inputs: Vec<Input>,
    input_index: usize,
    stats: FuzzerStats,
    settings: FuzzerSettings,
    world: World
    //stats, settings, process_start_time
}

impl<Input, Properties, World> FuzzerState<Input, Properties, World>
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

struct Fuzzer<Input, Generator, World> 
    where
    Input: FuzzerInput, 
    Generator: InputGenerator<Input=Input>, 
    World: FuzzerWorld<Input=Input, Properties=Generator> 
{
    state: FuzzerState<Input, Generator, World>,
    generator: Generator,
    test: Fn(Input) -> bool
    // signals_handler
}
