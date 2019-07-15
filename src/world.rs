use rand::rngs::ThreadRng;

use crate::command_line::*;
use crate::input::*;
use serde_json;
use serde_json::Value;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::Hasher;
use std::io::{self, Result};
use std::marker::PhantomData;
use std::path::Path;
use std::time::Instant;

#[derive(Clone, Copy, Default)]
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
    type Generator: InputGenerator<Input = Self::Input>;

    fn start_process(&mut self);
    fn elapsed_time(&self) -> usize;
    fn read_input_corpus(&self) -> Result<Vec<Self::Input>>;
    fn read_input_file(&self) -> Result<Self::Input>;

    fn add_to_output_corpus(&self, input: Self::Input) -> Result<()>;
    fn remove_from_output_corpus(&self, input: Self::Input) -> Result<()>;

    fn save_artifact(&self, input: &Self::Input, cplx: Option<f64>) -> Result<()>;
    fn report_event(&self, event: FuzzerEvent, stats: Option<FuzzerStats>);

    fn rand(&mut self) -> &mut ThreadRng;
}

pub struct CommandLineFuzzerWorld<Input, Generator>
where
    Input: FuzzerInput,
    Generator: InputGenerator<Input = Input>,
{
    settings: CommandLineArguments,
    rng: ThreadRng,
    instant: Instant,
    data: std::marker::PhantomData<Generator>,
}

impl<Input, Generator> CommandLineFuzzerWorld<Input, Generator>
where
    Input: FuzzerInput,
    Generator: InputGenerator<Input = Input>,
{
    pub fn new(settings: CommandLineArguments) -> Self {
        Self {
            settings,
            rng: rand::thread_rng(),
            instant: std::time::Instant::now(),
            data: PhantomData,
        }
    }
}

impl<I, P> FuzzerWorld for CommandLineFuzzerWorld<I, P>
where
    I: FuzzerInput,
    P: InputGenerator<Input = I>,
{
    type Input = I;
    type Generator = P;

    fn start_process(&mut self) {
        self.instant = Instant::now();
    }
    fn elapsed_time(&self) -> usize {
        self.instant.elapsed().as_micros() as usize
    }

    fn read_input_corpus(&self) -> Result<Vec<Self::Input>> {
        if self.settings.corpus_in.is_none() {
            return Result::Ok(vec![]);
        }
        let corpus = self.settings.corpus_in.as_ref().unwrap().as_path();

        if !corpus.is_dir() {
            return Result::Err(io::Error::new(
                io::ErrorKind::Other,
                "The path to the file containing the input is actually a directory.",
            ));
        }
        let mut inputs: Vec<Self::Input> = Vec::new();
        for entry in fs::read_dir(corpus)? {
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
        if let Some(input_file) = &self.settings.input_file {
            let data = fs::read(input_file)?;
            let string = String::from_utf8(data).unwrap();
            let content: &Value = &serde_json::from_str(&string)?;
            let input_content = content.get("input").unwrap_or(content);
            let i = serde_json::from_value(input_content.clone())?;
            Ok(i)
        } else {
            Result::Err(io::Error::new(
                io::ErrorKind::Other,
                "No input file was given as argument",
            ))
        }
    }

    fn add_to_output_corpus(&self, input: Self::Input) -> Result<()> {
        if self.settings.corpus_out.is_none() {
            return Ok(());
        }
        let corpus = self.settings.corpus_out.as_ref().unwrap().as_path();

        if !corpus.is_dir() {
            std::fs::create_dir_all(corpus)?;
        }

        let mut hasher = DefaultHasher::new();
        input.hash(&mut hasher);
        let hash = hasher.finish();
        let name = format!("{:x}", hash);

        let content = serde_json::to_string(&input)?;
        let path = corpus.join(name).with_extension("json");
        fs::write(path, content)?;
        Ok(())
    }

    fn remove_from_output_corpus(&self, input: Self::Input) -> Result<()> {
        if self.settings.corpus_out.is_none() {
            return Ok(());
        }
        let corpus = self.settings.corpus_out.as_ref().unwrap().as_path();

        let mut hasher = DefaultHasher::new();
        input.hash(&mut hasher);
        let hash = hasher.finish();
        let name = format!("{:x}", hash);

        let path = corpus.join(name).with_extension("json");
        fs::remove_file(path)?;
        Ok(())
    }


    fn save_artifact(&self, input: &Self::Input, cplx: Option<f64>) -> Result<()> {
        let default = Path::new("./artifacts/").to_path_buf();
        let artifacts_folder = self.settings.artifacts_folder.as_ref().unwrap_or(&default).as_path();

        if !artifacts_folder.is_dir() {
            std::fs::create_dir_all(artifacts_folder)?;
        }

        let mut hasher = DefaultHasher::new();
        input.hash(&mut hasher);
        let hash = hasher.finish();
        let s = if let Some(cplx) = cplx {
            serde_json::to_string(&json!({"input": input, "cplx": cplx}))?
        } else {
            serde_json::to_string(input)?
        };

        let name = format!("{:x}", hash);
        let path = artifacts_folder.join(name).with_extension("json");
        println!("Saving at {:?}", path);
        fs::write(path, s)?;
        Result::Ok(())
    }

    fn report_event(&self, event: FuzzerEvent, stats: Option<FuzzerStats>) {
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
