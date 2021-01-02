use decent_serde_json_alternative::{FromJson, ToJson};
use fuzzcheck_common::{
    arg::{FuzzerCommand, ResolvedCommandLineArguments},
    ipc,
};
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::{self, Result};
use std::time::Instant;
use std::{cell::RefCell, collections::hash_map::DefaultHasher, net::TcpStream};

use crate::world::{FuzzerEvent, FuzzerStats, WorldAction};
use crate::Serializer;

pub struct TuiWorld<S: Serializer> {
    stream: RefCell<TcpStream>,
    settings: ResolvedCommandLineArguments,
    instant: Instant,
    serializer: S,
}

#[derive(FromJson, ToJson)]
enum TuiMessage {
    AddInput { hash: String, input: String },
    RemoveInput { hash: String, input: String },
    ReportEvent { event: FuzzerEvent, stats: FuzzerStats },
}

impl<S: Serializer> TuiWorld<S> {
    pub fn new(serializer: S, settings: ResolvedCommandLineArguments) -> Self {
        let stream = RefCell::new(TcpStream::connect(settings.socket_address.unwrap()).unwrap());
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
            let message = match a {
                WorldAction::Add(x) => {
                    let (hash, input) = self.hash_and_string_of_input(x);
                    TuiMessage::AddInput { hash, input }
                }
                WorldAction::Remove(x) => {
                    let (hash, input) = self.hash_and_string_of_input(x);
                    TuiMessage::RemoveInput { hash, input }
                }
                WorldAction::ReportEvent(event) => TuiMessage::ReportEvent { event, stats: *stats },
            };

            let mut stream = self.stream.borrow_mut();
            ipc::write(&mut stream, &message.to_json().to_string());
        }
        Ok(())
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
