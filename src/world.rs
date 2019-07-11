use rand::rngs::ThreadRng;

use crate::artifact::*;
use crate::input::*;
use crate::input_pool::Feature;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json;
use std::fs;
use std::fs::File;
use std::io::{self, Result};
use std::path::Path;
use std::time::Instant;

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
    Deleted(usize),
    New,
    DidReadCorpus,
    CaughtSignal(i32),
    TestFailure,
}

pub trait FuzzerWorld {
    type Input: FuzzerInput;
    type Properties: InputProperties<Input = Self::Input>;

    fn start_process(&mut self);
    fn elapsed_time(&self) -> usize;
    fn read_input_corpus(&self) -> Result<Vec<Self::Input>>;
    fn read_input_file(&self) -> Result<Self::Input>;

    fn save_artifact(&self, input: &Self::Input, features: Option<Vec<Feature>>, kind: ArtifactKind);
    fn report_event(&self, event: FuzzerEvent, stats: Option<&FuzzerStats>);

    fn rand(&mut self) -> &mut ThreadRng;
}

pub struct CommandLineFuzzerInfo<'a> {
    rng: ThreadRng,
    instant: Instant,
    input_file: &'a Path,
    input_folder: &'a Path,
    output_folder: &'a Path,
    artifacts_foldeer: &'a Path,
}

pub struct CommandLineFuzzerWorld<'a, Input, Properties>
where
    Input: FuzzerInput,
    Properties: InputProperties<Input = Input>,
{
    info: CommandLineFuzzerInfo<'a>,
    rng: ThreadRng,
    data: std::marker::PhantomData<Properties>,
}

impl<'a, I, P> FuzzerWorld for CommandLineFuzzerWorld<'a, I, P>
where
    I: FuzzerInput,
    P: InputProperties<Input = I>,
{
    type Input = I;
    type Properties = P;

    fn start_process(&mut self) {
        self.info.instant = Instant::now();
    }
    fn elapsed_time(&self) -> usize {
        self.info.instant.elapsed().as_secs() as usize
    }

    fn read_input_corpus(&self) -> Result<Vec<Self::Input>> {
        let dir = self.info.input_folder;
        if !dir.is_dir() {
            return Result::Err(io::Error::new(
                io::ErrorKind::Other,
                "The path to the file containing the input is actually a directory.",
            ));
        }
        let mut inputs: Vec<Self::Input> = Vec::new();
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            if entry.path().is_dir() {
                continue;
            }
            let data = fs::read(entry.path())?;
            let string = String::from_utf8(data).unwrap();
            let i: Self::Input = serde_json::from_str(&string)?;
            inputs.push(i);
        }
        Ok(inputs)
    }
    fn read_input_file(&self) -> Result<Self::Input> {
        let data = fs::read(self.info.input_file)?;
        let string = String::from_utf8(data).unwrap();
        Ok(serde_json::from_str(&string)?)
    }
    fn save_artifact(&self, input: &Self::Input, features: Option<Vec<Feature>>, kind: ArtifactKind) {}
    fn report_event(&self, event: FuzzerEvent, stats: Option<&FuzzerStats>) {
        if let FuzzerEvent::Deleted(count) = event {
            print!("DELETED {:?}", count);
            return;
        }
        match event {
            FuzzerEvent::Start => print!("START"),
            FuzzerEvent::Done => print!("DONE"),
            FuzzerEvent::New => print!("NEW\t"),
            FuzzerEvent::DidReadCorpus => print!("FINISHED READING CORPUS"),
            FuzzerEvent::CaughtSignal(signal) => match signal {
                4 | 6 | 10 | 11 | 8 => print!("\n================ CRASH DETECTED ================"),
                2 | 15 => print!("\n================ RUN INTERRUPTED ================"),
                _ => print!("\n================ SIGNAL {:?} ================", signal),
            },
            FuzzerEvent::TestFailure => print!("\n================ TEST FAILED ================"),
            FuzzerEvent::Deleted(_) => unreachable!("Deleted case handled separately above"),
        };
        if let Some(stats) = stats {
            print!("{:?}\t", stats.total_number_of_runs);
            print!("score: {:?}\t", stats.score);
            print!("corp: {:?}\t", stats.pool_size);
            print!("exec/s: {:?}\t", stats.exec_per_s);
            print!("cplx: {:?}\t", stats.avg_cplx);
        }
    }

    fn rand(&mut self) -> &mut ThreadRng {
        &mut self.rng
    }
}
