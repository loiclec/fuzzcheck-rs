use rand::rngs::ThreadRng;

use crate::artifact::*;
use crate::input::*;
use crate::input_pool::Feature;

pub type Signal = bool;

pub enum FuzzerCommand {
    Minimize,
    Fuzz,
    Read,
}

pub struct FuzzerSettings {
    pub command: FuzzerCommand,
    pub max_nbr_of_runs: usize,
    pub max_input_cplx: f64,
    pub mutate_depth: usize,
}

pub struct FuzzerStats {
    pub total_number_of_runs: usize,
    pub score: usize,
    pub pool_size: usize,
    pub exec_per_s: usize,
    pub avg_cplx: usize,
}

// public struct FuzzerStats {
//     public var totalNumberOfRuns: Int = 0
//     public var score: Int = 0
//     public var poolSize: Int = 0
//     public var executionsPerSecond: Int = 0
//     public var averageComplexity: Int = 0
//     public var rss: Int = 0
// }

// public struct FuzzerSettings {

//     public enum Command: String {
//         case minimize
//         case fuzz
//         case read
//     }

//     public var command: Command
//     public var maxNumberOfRuns: Int
//     public var maxInputComplexity: Double
//     public var mutateDepth: Int

//     public init(command: Command = .fuzz, maxNumberOfRuns: Int = Int.max, maxInputComplexity: Double = 256.0, mutateDepth: Int = 3) {
//         self.command = command
//         self.maxNumberOfRuns = maxNumberOfRuns
//         self.maxInputComplexity = maxInputComplexity
//         self.mutateDepth = mutateDepth
//     }
// }

pub enum FuzzerEvent {
    Start,
    Done,
    New,
    DidReadCorpus,
    DidResetPool,
    CaughtSignal(Signal),
    TestFailure,
}

pub trait FuzzerWorld {
    type Input: FuzzerInput;
    type Properties: InputProperties<Input = Self::Input>;

    fn clock(&self) -> usize;
    fn read_input_corpus(&self) -> Vec<Self::Input>;
    fn read_input_file(&self) -> Self::Input;

    fn save_artifact(&self, input: &Self::Input, features: Option<Vec<Feature>>, kind: ArtifactKind);
    fn add_to_output_corpus(&self, input: Self::Input);
    fn remove_from_output_corpus(&self, input: Self::Input);
    fn report_event(&self, event: FuzzerEvent, stats: &FuzzerStats);

    fn rand(&mut self) -> &mut ThreadRng;
}
