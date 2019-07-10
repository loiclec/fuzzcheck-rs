use core::hash::Hash;

use rand::rngs::ThreadRng;

use crate::fuzzer::input::*;
use crate::fuzzer::input_pool::Feature;

pub type Signal = bool;
type ArtifactKind = bool;
pub type FuzzerStats = bool;

pub enum FuzzerEvent {
    Start,
    Done,
    New,
    DidReadCorpus,
    DidResetPool,
    CaughtSignal(Signal),
    TestFailure
}

pub trait FuzzerWorld {
    type Input: FuzzerInput;
    type Properties: InputProperties<Input=Self::Input>;

    fn clock(&self) -> usize;
    fn read_input_corpus(&self) -> [Self::Input];
    fn read_input_file(&self) -> Self::Input;
    
    fn save_artifact(&self, input: Self::Input, features: Option<Vec<Feature>>, score: Option<f64>, kind: ArtifactKind);
    fn add_to_output_corpus(&self, input: Self::Input);
    fn remove_from_output_corpus(&self, input: Self::Input);
    fn report_event(&self, event: FuzzerEvent, stats: FuzzerStats);

    fn rand(&mut self) -> &mut ThreadRng;
}
