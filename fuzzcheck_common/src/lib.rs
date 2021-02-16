pub mod arg;

#[cfg(feature = "ui")]
pub mod ipc;
#[cfg(feature = "ui")]
use decent_serde_json_alternative::{FromJson, ToJson};

#[cfg_attr(feature = "ui", derive(FromJson, ToJson))]
#[derive(Clone, Copy, Default)]
pub struct FuzzerStats {
    pub total_number_of_runs: usize,
    pub number_of_runs_since_last_reset_time: usize,
    pub score: f64,
    pub pool_size: usize,
    pub exec_per_s: usize,
    pub avg_cplx: f64,
}

impl FuzzerStats {
    pub fn new() -> FuzzerStats {
        FuzzerStats {
            total_number_of_runs: 0,
            number_of_runs_since_last_reset_time: 0,
            score: 0.0,
            pool_size: 0,
            exec_per_s: 0,
            avg_cplx: 0.0,
        }
    }
}

#[cfg_attr(feature = "ui", derive(FromJson, ToJson))]
#[derive(Clone, Copy)]
pub enum FuzzerEvent {
    Start,
    Stop,
    End,
    CrashNoInput,
    Pulse,
    Done,
    New,
    Replace(usize),
    ReplaceLowestStack(usize),
    Remove,
    DidReadCorpus,
    CaughtSignal(i32),
    TestFailure,
}
