use rand::rngs::ThreadRng;

use crate::artifact::*;
use crate::command_line::*;
use crate::input::*;
use serde_json;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::Hasher;
use std::io::{self, Result};
use std::marker::PhantomData;
use std::time::Instant;

#[derive(Default)]
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

    fn save_artifact(&self, input: &Self::Input, kind: ArtifactKind) -> Result<()>;
    fn report_event(&self, event: FuzzerEvent, stats: Option<&FuzzerStats>);

    fn rand(&mut self) -> &mut ThreadRng;
}

pub struct CommandLineFuzzerWorld<Input, Properties>
where
    Input: FuzzerInput,
    Properties: InputProperties<Input = Input>,
{
    info: CommandLineFuzzerInfo,
    rng: ThreadRng,
    instant: Instant,
    data: std::marker::PhantomData<Properties>,
}

impl<Input, Properties> CommandLineFuzzerWorld<Input, Properties>
where
    Input: FuzzerInput,
    Properties: InputProperties<Input = Input>,
{
    pub fn new(info: CommandLineFuzzerInfo) -> Self {
        Self {
            info,
            rng: rand::thread_rng(),
            instant: std::time::Instant::now(),
            data: PhantomData,
        }
    }
}

impl<I, P> FuzzerWorld for CommandLineFuzzerWorld<I, P>
where
    I: FuzzerInput,
    P: InputProperties<Input = I>,
{
    type Input = I;
    type Properties = P;

    fn start_process(&mut self) {
        self.instant = Instant::now();
    }
    fn elapsed_time(&self) -> usize {
        self.instant.elapsed().as_secs() as usize
    }

    fn read_input_corpus(&self) -> Result<Vec<Self::Input>> {
        if let Some(dir) = &self.info.input_folder {
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
        } else {
            Result::Err(io::Error::new(
                io::ErrorKind::Other,
                "No input file was given as argument",
            ))
        }
    }
    fn read_input_file(&self) -> Result<Self::Input> {
        if let Some(input_file) = &self.info.input_file {
            let data = fs::read(input_file)?;
            let string = String::from_utf8(data).unwrap();
            Ok(serde_json::from_str(&string)?)
        } else {
            Result::Err(io::Error::new(
                io::ErrorKind::Other,
                "No input file was given as argument",
            ))
        }
    }

    fn save_artifact(&self, input: &Self::Input, kind: ArtifactKind) -> Result<()> {
        if let Some(artifacts_folder) = &self.info.artifacts_folder {
            let mut hasher = DefaultHasher::new();
            input.hash(&mut hasher);
            let hash = hasher.finish();
            let s = serde_json::to_string(input)?;
            let name = format!("{:x}", hash);
            let path = artifacts_folder.join(name);
            println!("Saving {:?} at {:?}", kind, path);
            fs::write(path, s)?;
            Result::Ok(())
        } else {
            let s = serde_json::to_string(input)?;
            println!("{}", s);
            Result::Err(io::Error::new(
                io::ErrorKind::Other,
                "No artifacts folder was given as argument",
            ))
        }
    }

    fn report_event(&self, event: FuzzerEvent, stats: Option<&FuzzerStats>) {
        if let FuzzerEvent::Deleted(count) = event {
            println!("DELETED {:?}", count);
            return;
        }
        match event {
            FuzzerEvent::Start => print!("START"),
            FuzzerEvent::Done => print!("DONE"),
            FuzzerEvent::New => print!("NEW\t"),
            FuzzerEvent::DidReadCorpus => print!("FINISHED READING CORPUS"),
            FuzzerEvent::CaughtSignal(signal) => match signal {
                4 | 6 | 10 | 11 | 8 => println!("\n================ CRASH DETECTED ================"),
                2 | 15 => println!("\n================ RUN INTERRUPTED ================"),
                _ => println!("\n================ SIGNAL {:?} ================", signal),
            },
            FuzzerEvent::TestFailure => println!("\n================ TEST FAILED ================"),
            FuzzerEvent::Deleted(_) => unreachable!("Deleted case handled separately above"),
        };
        if let Some(stats) = stats {
            print!("{:?}\t", stats.total_number_of_runs);
            print!("score: {:?}\t", stats.score);
            print!("corp: {:?}\t", stats.pool_size);
            print!("exec/s: {:?}\t", stats.exec_per_s);
            print!("cplx: {:?}\t", stats.avg_cplx);
        }
        println!();
    }

    fn rand(&mut self) -> &mut ThreadRng {
        &mut self.rng
    }
}
