//! This is the interface between the fuzzer and the rest of the world.
//! It manages the fuzzing corpus in the file system as well as the terminal
//! output.
//!

// In the future it would be nice to make it a trait so that it is easy to
// create different “World” implementations.

use fuzzcheck_arg_parser::{CommandLineArguments, FuzzerCommand};
use std::collections::hash_map::DefaultHasher;
use std::fs;

use std::hash::{Hash, Hasher};
use std::io::{self, Result};

use std::path::Path;
use std::time::Instant;

use crate::{Feature, Serializer};

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

#[derive(Clone)]
pub enum FuzzerEvent {
    Start,
    Done,
    New,
    Replace(usize),
    Remove,
    DidReadCorpus,
    CaughtSignal(i32),
    TestFailure,
}

#[derive(Clone)]
pub(crate) enum WorldAction<T> {
    Remove(T),
    Add(T, Vec<Feature>),
    ReportEvent(FuzzerEvent),
}

pub struct World<S: Serializer> {
    settings: CommandLineArguments,
    instant: Instant,
    serializer: S,
}

impl<S: Serializer> World<S> {
    pub fn new(serializer: S, settings: CommandLineArguments) -> Self {
        Self {
            settings,
            instant: std::time::Instant::now(),
            serializer,
        }
    }

    pub(crate) fn do_actions(&self, actions: Vec<WorldAction<S::Value>>, stats: &FuzzerStats) -> Result<()> {
        for a in actions {
            match a {
                WorldAction::Add(x, _) => {
                    self.add_to_output_corpus(&x)?;
                }
                WorldAction::Remove(x) => {
                    self.remove_from_output_corpus(&x)?;
                }
                WorldAction::ReportEvent(e) => match e {
                    FuzzerEvent::New | FuzzerEvent::Remove | FuzzerEvent::Replace(_) => {
                        self.report_event(e, Some(*stats))
                    }
                    _ => self.report_event(e, None),
                },
            }
        }
        Ok(())
    }
}

impl<S: Serializer> World<S> {
    pub fn set_start_time(&mut self) {
        self.instant = Instant::now();
    }
    pub fn elapsed_time(&self) -> usize {
        self.instant.elapsed().as_micros() as usize
    }

    pub fn read_input_corpus(&self) -> Result<Vec<S::Value>> {
        if self.settings.corpus_in.is_none() {
            return Result::Ok(vec![]);
        }
        let corpus = self.settings.corpus_in.as_ref().unwrap().as_path();

        if !corpus.is_dir() {
            return Result::Err(io::Error::new(
                io::ErrorKind::Other,
                "The corpus path is not a directory.",
            ));
        }
        let mut inputs: Vec<S::Value> = Vec::new();
        for entry in fs::read_dir(corpus)? {
            let entry = entry?;
            if entry.path().is_dir() {
                continue;
            }
            let data = fs::read(entry.path())?;
            if let Some(i) = self.serializer.from_data(&data) {
                inputs.push(i);
            }
        }
        Ok(inputs)
    }
    pub fn read_input_file(&self) -> Result<S::Value> {
        if let Some(input_file) = &self.settings.input_file {
            let data = fs::read(input_file)?;
            if let Some(input) = self.serializer.from_data(&data) {
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

    pub fn add_to_output_corpus(&self, input: &S::Value) -> Result<()> {
        if self.settings.corpus_out.is_none() {
            return Ok(());
        }
        let corpus = self.settings.corpus_out.as_ref().unwrap().as_path();

        if !corpus.is_dir() {
            std::fs::create_dir_all(corpus)?;
        }

        let mut hasher = DefaultHasher::new();
        let content = self.serializer.to_data(&input);
        content.hash(&mut hasher);
        let hash = hasher.finish();
        let name = format!("{:x}", hash);
        let path = corpus.join(name).with_extension(self.serializer.extension());
        fs::write(path, content)?;

        Ok(())
    }

    pub fn remove_from_output_corpus(&self, input: &S::Value) -> Result<()> {
        if self.settings.corpus_out.is_none() {
            return Ok(());
        }
        let corpus = self.settings.corpus_out.as_ref().unwrap().as_path();

        let mut hasher = DefaultHasher::new();
        let content = self.serializer.to_data(&input);
        content.hash(&mut hasher);
        let hash = hasher.finish();
        let name = format!("{:x}", hash);

        let path = corpus.join(name).with_extension(self.serializer.extension());
        let _ = fs::remove_file(path);

        Ok(())
    }

    pub fn save_artifact(&self, input: &S::Value, cplx: f64) -> Result<()> {
        let default = Path::new("./artifacts/").to_path_buf();
        let artifacts_folder = self.settings.artifacts_folder.as_ref().unwrap_or(&default).as_path();

        if !artifacts_folder.is_dir() {
            std::fs::create_dir_all(artifacts_folder)?;
        }

        let mut hasher = DefaultHasher::new();
        let content = self.serializer.to_data(&input);
        content.hash(&mut hasher);
        let hash = hasher.finish();

        let name = if let FuzzerCommand::MinifyInput | FuzzerCommand::Read = self.settings.command {
            format!("{:.0}--{:x}", cplx * 100.0, hash)
        } else {
            format!("{:x}", hash)
        };

        let path = artifacts_folder.join(name).with_extension(self.serializer.extension());
        println!("Saving at {:?}", path);
        fs::write(path, content)?;
        Result::Ok(())
    }

    pub fn report_event(&self, event: FuzzerEvent, stats: Option<FuzzerStats>) {
        match event {
            FuzzerEvent::Start => {
                println!("START");
                return;
            }
            FuzzerEvent::Done => {
                println!("DONE");
                return;
            }
            FuzzerEvent::New => print!("NEW\t"),
            FuzzerEvent::Remove => print!("REMOVE\t"),
            FuzzerEvent::DidReadCorpus => {
                println!("FINISHED READING CORPUS");
                return;
            }
            FuzzerEvent::CaughtSignal(signal) => match signal {
                4 | 6 | 10 | 11 | 8 => println!("\n================ CRASH DETECTED ================"),
                2 | 15 => println!("\n================ RUN INTERRUPTED ================"),
                _ => println!("\n================ SIGNAL {:?} ================", signal),
            },
            FuzzerEvent::TestFailure => println!("\n================ TEST FAILED ================"),
            FuzzerEvent::Replace(count) => {
                print!("RPLC {}\t", count);
            }
        };
        if let Some(stats) = stats {
            print!("{}\t", stats.total_number_of_runs);
            print!("score: {:.2}\t", stats.score);
            print!("pool: {}\t", stats.pool_size);
            print!("exec/s: {}\t", stats.exec_per_s);
            print!("cplx: {:.2}\t", stats.avg_cplx);
            println!();
        }
    }
}
