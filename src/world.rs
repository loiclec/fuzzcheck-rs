use rand::rngs::ThreadRng;

use crate::artifact::*;
use crate::input::*;
use crate::input_pool::Feature;

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
impl FuzzerStats {
    pub fn new() -> FuzzerStats {
        FuzzerStats {
            total_number_of_runs: 0,
            score: 0,
            pool_size: 0,
            exec_per_s: 0,
            avg_cplx: 0,
        }
    }
}

pub enum FuzzerEvent {
    Start,
    Done,
    New,
    DidReadCorpus,
    DidResetPool,
    CaughtSignal(i32),
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
