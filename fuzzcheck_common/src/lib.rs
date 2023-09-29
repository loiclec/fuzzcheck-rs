#![feature(coverage_attribute)]

pub mod arg;

#[derive(Clone, Copy, Default)]
pub struct FuzzerStats {
    pub total_number_of_runs: usize,
    pub number_of_runs_since_last_reset_time: usize,
    pub exec_per_s: usize,
}

#[derive(Clone, Copy)]
pub enum FuzzerEvent {
    Start,
    Stop,
    End,
    CrashNoInput,
    Pulse,
    Done,
    Replace(usize, usize),
    DidReadCorpus,
    CaughtSignal(i32),
    TestFailure,
    None,
}
