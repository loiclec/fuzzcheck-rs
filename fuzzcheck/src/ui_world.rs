use decent_serde_json_alternative::{FromJson, ToJson};
use fuzzcheck_common::{
    arg::{FullCommandLineArguments, FuzzerCommand},
    ipc,
};
use std::{fs, time::Duration};
use std::hash::{Hash, Hasher};
use std::io::{self, Result};
use std::time::Instant;
use std::{cell::RefCell, collections::hash_map::DefaultHasher, net::TcpStream};

use crate::world::{WorldAction};

use fuzzcheck_common::ipc::{FuzzerStats, FuzzerEvent, TuiMessage};

use crate::Serializer;

pub struct TuiWorld<S: Serializer> {
    stream: Option<RefCell<TcpStream>>,
    settings: FullCommandLineArguments,
    instant: Instant,
    serializer: S,
}

impl<S: Serializer> TuiWorld<S> {
    pub fn new(serializer: S, settings: FullCommandLineArguments) -> Self {
        let stream  = if let Some(socket_address) = settings.socket_address {
            Some(RefCell::new(TcpStream::connect(socket_address).unwrap()))
        } else {
            None
        };
        Self {
            stream,
            settings,
            instant: std::time::Instant::now(),
            serializer,
        }
    }

    fn hash_and_string_of_input(&self, input: S::Value) -> (String, String) {
        let input = self.serializer.to_data(&input);
        let mut hasher = DefaultHasher::new();
        input.hash(&mut hasher);
        let hash = hasher.finish();
        let hash = format!("{:x}", hash);
        let input = if self.serializer.is_utf8() {
            String::from_utf8_lossy(&input).to_string()
        } else {
            base64::encode(&input)
        };
        (hash, input)
    }

    pub(crate) fn do_actions(&self, actions: Vec<WorldAction<S::Value>>, stats: &FuzzerStats) -> Result<()> {
        for a in actions {
            let is_failure = matches!(a, WorldAction::ReportEvent(FuzzerEvent::TestFailure));
            let message = match a {
                WorldAction::Add(x) => {
                    let (hash, input) = self.hash_and_string_of_input(x);
                    TuiMessage::AddInput { hash, input }
                }
                WorldAction::Remove(x) => {
                    let (hash, input) = self.hash_and_string_of_input(x);
                    TuiMessage::RemoveInput { hash, input }
                }
                WorldAction::ReportEvent(event) => {
                    self.report_event(event.clone(), Some(*stats));
                    TuiMessage::ReportEvent { event, stats: *stats }
                },
            };

            if let Some(stream) = &self.stream {
                let mut stream = stream.borrow_mut();
                ipc::write(&mut stream, &message.to_json().to_string());
            }
        }
        Ok(())
    }
    fn report_event(&self, event: FuzzerEvent, stats: Option<FuzzerStats>) {
    // println uses a lock, which may mess up the signal handling
        match event {
            FuzzerEvent::Start => {
                println!("START");
                return;
            }
            FuzzerEvent::End => {
                //;
                println!("\n======================== END ========================");
                println!(
                    r#"Fuzzcheck cannot generate more arbitrary values of the input type. This may be
because all possible values under the chosen maximum complexity were tested, or
because the mutator does not know how to generate more values."#
                );
                return;
            }
            FuzzerEvent::CrashNoInput => {
                //;
                println!("\n=================== CRASH DETECTED ===================");
                println!(
                    r#"A crash was detected, but the fuzzer cannot recover the crashing input.
This should never happen, and is probably a bug in fuzzcheck. Sorry :("#
                );
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
            FuzzerEvent::CaughtSignal(signal) => {
                match signal {
                    _ => println!("\n================ SIGNAL {} ================", signal),
                }
            },
            FuzzerEvent::TestFailure => {
                println!("\n================ TEST FAILED ================");
            },
            FuzzerEvent::Replace(count) => {
                print!("RPLC {}\t", count);
            }
            FuzzerEvent::ReplaceLowestStack(stack) => {
                print!("STACK {}\n", stack);
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

impl<S: Serializer> TuiWorld<S> {
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

    pub fn save_artifact(&self, input: &S::Value, cplx: f64) -> Result<()> {
        let artifacts_folder = self.settings.artifacts_folder.as_ref();
        if artifacts_folder.is_none() {
            return Ok(());
        }
        let artifacts_folder = artifacts_folder.unwrap().as_path();

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
}
