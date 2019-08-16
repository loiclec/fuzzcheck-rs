use crate::command_line::*;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::Hash;
use std::hash::Hasher;
use std::io::{self, Result};

use std::path::Path;
use std::time::Instant;

use std::marker::PhantomData;

use crate::input::InputGenerator;
use crate::input_pool::Feature;

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

pub struct World<T, G>
where
    T: Hash + Clone,
    G: InputGenerator<Input = T>,
{
    settings: CommandLineArguments,
    instant: Instant,
    phantom: PhantomData<G>,
}

impl<T, G> World<T, G>
where
    T: Hash + Clone,
    G: InputGenerator<Input = T>,
{
    pub fn new(settings: CommandLineArguments) -> Self {
        Self {
            settings,
            instant: std::time::Instant::now(),
            phantom: PhantomData,
        }
    }
}

impl<T, G> World<T, G>
where
    T: Hash + Clone,
    G: InputGenerator<Input = T>,
{
    pub fn start_process(&mut self) {
        self.instant = Instant::now();
    }
    pub fn elapsed_time(&self) -> usize {
        self.instant.elapsed().as_micros() as usize
    }

    pub fn read_input_corpus(&self) -> Result<Vec<T>> {
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
        let mut inputs: Vec<T> = Vec::new();
        for entry in fs::read_dir(corpus)? {
            let entry = entry?;
            if entry.path().is_dir() {
                continue;
            }
            let data = fs::read(entry.path())?;
            if let Some(i) = G::from_data(&data) {
                inputs.push(i);
            } else {
                continue;
            }
        }
        Ok(inputs)
    }
    pub fn read_input_file(&self) -> Result<T> {
        if let Some(input_file) = &self.settings.input_file {
            let data = fs::read(input_file)?;
            if let Some(input) = G::from_data(&data) {
                Ok(input)
            } else {
                Result::Err(io::Error::new(
                    io::ErrorKind::Other,
                    "The file could not be decoded into a valid input.",
                ))
            }
        } else {
            Result::Err(io::Error::new(
                io::ErrorKind::Other,
                "No input file was given as argument",
            ))
        }
    }

    pub fn add_to_output_corpus(&self, input: T, features: Vec<Feature>) -> Result<()> {
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

        let content = G::to_data(&input);
        let path = corpus.join(name).with_extension("json");
        fs::write(path, content)?;

        if self.settings.debug {
            let name = format!("{:x}-features", hash);
            let content = serde_json::to_vec_pretty(&features).unwrap();
            let path = corpus.join(name).with_extension("json");
            fs::write(path, content)?;
        }

        Ok(())
    }

    pub fn remove_from_output_corpus(&self, input: T) -> Result<()> {
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

        if self.settings.debug {
            let name = format!("{:x}-features", hash);
            let path = corpus.join(name).with_extension("json");
            let _ = fs::remove_file(path);
        }

        Ok(())
    }

    pub fn save_artifact(&self, input: &T, cplx: f64) -> Result<()> {
        let default = Path::new("./artifacts/").to_path_buf();
        let artifacts_folder = self.settings.artifacts_folder.as_ref().unwrap_or(&default).as_path();

        if !artifacts_folder.is_dir() {
            std::fs::create_dir_all(artifacts_folder)?;
        }

        let mut hasher = DefaultHasher::new();
        input.hash(&mut hasher);
        let hash = hasher.finish();
        let content = G::to_data(&input);

        let name = if let FuzzerCommand::Minimize | FuzzerCommand::Read = self.settings.command {
            format!("{:.0}--{:x}", cplx * 100.0, hash)
        } else {
            format!("{:x}", hash)
        };

        let path = artifacts_folder.join(name).with_extension("json");
        println!("Saving at {:?}", path);
        fs::write(path, content)?;
        Result::Ok(())
    }

    pub fn report_event(&self, event: FuzzerEvent, stats: Option<FuzzerStats>) {
        match event {
            FuzzerEvent::Start => {
                println!("START");
                return
            },
            FuzzerEvent::Done => {
                println!("DONE");
                return
            },
            FuzzerEvent::New => print!("NEW\t"),
            FuzzerEvent::DidReadCorpus => {
                println!("FINISHED READING CORPUS");
                return
            },
            FuzzerEvent::CaughtSignal(signal) => match signal {
                4 | 6 | 10 | 11 | 8 => println!("\n================ CRASH DETECTED ================"),
                2 | 15 => println!("\n================ RUN INTERRUPTED ================"),
                _ => println!("\n================ SIGNAL {:?} ================", signal),
            },
            FuzzerEvent::TestFailure => println!("\n================ TEST FAILED ================"),
            FuzzerEvent::Deleted(count) => {
                println!("DELETED {:?}", count);
                return
            }
        };
        if let Some(stats) = stats {
            print!("{:?}\t", stats.total_number_of_runs);
            print!("score: {:?}\t", stats.score);
            print!("pool: {:?}\t", stats.pool_size);
            print!("exec/s: {:?}\t", stats.exec_per_s);
            print!("cplx: {:?}\t", stats.avg_cplx);
            println!();
        }
    }
}
