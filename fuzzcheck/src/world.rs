#[cfg(feature = "ui")]
use decent_serde_json_alternative::FromJson;
#[cfg(feature = "ui")]
use decent_serde_json_alternative::ToJson;
use fuzzcheck_common::arg::{FullCommandLineArguments, FuzzerCommand};
#[cfg(feature = "ui")]
use fuzzcheck_common::ipc::{self, MessageUserToFuzzer, TuiMessage, TuiMessageEvent};
#[cfg(feature = "ui")]
use std::net::TcpStream;

use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::{self, Result};
use std::time::Instant;

use fuzzcheck_common::{FuzzerEvent, FuzzerStats};

use crate::{fuzzer::TerminationStatus, Serializer};

pub(crate) enum WorldAction<T> {
    Remove(T),
    Add(T),
    ReportEvent(FuzzerEvent),
}

pub struct World<S: Serializer> {
    #[cfg(feature = "ui")]
    stream: Option<TcpStream>,
    settings: FullCommandLineArguments,
    initial_instant: Instant,
    checkpoint_instant: Instant,
    #[cfg(feature = "ui")]
    pause_at_next_event: bool,
    serializer: S,
}

impl<S: Serializer> World<S> {
    pub fn new(serializer: S, settings: FullCommandLineArguments) -> Self {
        #[cfg(feature = "ui")]
        let stream = if let Some(socket_address) = settings.socket_address {
            Some(TcpStream::connect(socket_address).unwrap())
        } else {
            None
        };
        #[cfg(feature = "ui")]
        let pause_at_next_event = stream.is_some();
        Self {
            #[cfg(feature = "ui")]
            stream,
            settings,
            initial_instant: std::time::Instant::now(),
            checkpoint_instant: std::time::Instant::now(),
            #[cfg(feature = "ui")]
            pause_at_next_event,
            serializer,
        }
    }

    #[cfg(not(feature = "ui"))]
    fn hash_and_string_of_input(&self, input: S::Value) -> (String, Vec<u8>) {
        let input = self.serializer.to_data(&input);
        let mut hasher = DefaultHasher::new();
        input.hash(&mut hasher);
        let hash = hasher.finish();
        let hash = format!("{:x}", hash);
        (hash, input)
    }

    #[cfg(feature = "ui")]
    fn hash_and_string_of_input(&self, input: S::Value) -> (String, Vec<u8>) {
        let input = self.serializer.to_data(&input);
        let mut hasher = DefaultHasher::new();
        input.hash(&mut hasher);
        let hash = hasher.finish();
        let hash = format!("{:x}", hash);
        let input = if self.serializer.is_utf8() {
            String::from_utf8_lossy(&input).to_string()
        } else {
            base64::encode(&input)
        }
        .into_bytes();
        (hash, input)
    }

    #[allow(unused_variables)]
    pub(crate) fn do_actions(&mut self, actions: Vec<WorldAction<S::Value>>, stats: &FuzzerStats) -> Result<()> {
        for a in actions {
            let message = match a {
                WorldAction::Add(x) => {
                    let (hash, input) = self.hash_and_string_of_input(x);
                    self.add_to_output_corpus(hash.clone(), input.clone())?;
                    #[cfg(feature = "ui")]
                    TuiMessage::AddInput {
                        hash,
                        input: String::from_utf8_lossy(&input).to_string(),
                    }
                }
                WorldAction::Remove(x) => {
                    let (hash, input) = self.hash_and_string_of_input(x);
                    self.remove_from_output_corpus(hash.clone())?;
                    #[cfg(feature = "ui")]
                    TuiMessage::RemoveInput {
                        hash,
                        input: String::from_utf8_lossy(&input).to_string(),
                    }
                }
                WorldAction::ReportEvent(event) => {
                    self.report_event(event.clone(), Some(*stats));
                    #[cfg(feature = "ui")]
                    TuiMessage::ReportEvent(TuiMessageEvent {
                        event,
                        stats: *stats,
                        time_ms: self.elapsed_time_since_start() / 1000,
                    })
                }
            };

            #[cfg(feature = "ui")]
            self.write_to_stream(&message);
        }
        #[cfg(feature = "ui")]
        if self.pause_at_next_event {
            self.pause_until_unpause_message();
        }
        Ok(())
    }

    pub fn add_to_output_corpus(&self, name: String, content: Vec<u8>) -> Result<()> {
        if self.settings.corpus_out.is_none() {
            return Ok(());
        }
        let corpus = self.settings.corpus_out.as_ref().unwrap().as_path();

        if !corpus.is_dir() {
            std::fs::create_dir_all(corpus)?;
        }

        let path = corpus.join(name).with_extension(self.serializer.extension());
        fs::write(path, content)?;

        Ok(())
    }

    pub fn remove_from_output_corpus(&self, name: String) -> Result<()> {
        if self.settings.corpus_out.is_none() {
            return Ok(());
        }
        let corpus = self.settings.corpus_out.as_ref().unwrap().as_path();

        let path = corpus.join(name).with_extension(self.serializer.extension());
        let _ = fs::remove_file(path);

        Ok(())
    }

    fn report_event(&self, event: FuzzerEvent, stats: Option<FuzzerStats>) {
        // println uses a lock, which may mess up the signal handling
        match event {
            FuzzerEvent::Start => {
                println!("START");
                return;
            }
            FuzzerEvent::Stop => {
                println!("\n======================== STOPPED ========================");
                println!(r#"The fuzzer was stopped."#);
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
            FuzzerEvent::CaughtSignal(signal) => match signal {
                _ => println!("\n================ SIGNAL {} ================", signal),
            },
            FuzzerEvent::TestFailure => {
                println!("\n================ TEST FAILED ================");
            }
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

    pub fn set_start_instant(&mut self) {
        self.initial_instant = Instant::now();
    }
    pub fn set_checkpoint_instant(&mut self) {
        self.checkpoint_instant = Instant::now();
    }
    pub fn elapsed_time_since_start(&self) -> usize {
        self.initial_instant.elapsed().as_micros() as usize
    }
    pub fn elapsed_time_since_last_checkpoint(&self) -> usize {
        self.checkpoint_instant.elapsed().as_micros() as usize
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

    pub fn save_artifact(&mut self, input: &S::Value, cplx: f64) -> Result<()> {
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

        let path = artifacts_folder.join(&name).with_extension(self.serializer.extension());
        println!("Saving at {:?}", path);
        fs::write(path, &content)?;

        #[cfg(feature = "ui")]
        {
            let message = TuiMessage::SaveArtifact {
                hash: name,
                input: String::from_utf8_lossy(&content).to_string(),
            };
            self.write_to_stream(&message);
        }
        Result::Ok(())
    }

    #[cfg(feature = "ui")]
    pub fn read_message_from_user(&mut self, blocking: bool) -> Option<MessageUserToFuzzer> {
        if let Some(stream) = &mut self.stream {
            let _ = stream.set_nonblocking(blocking);
            let received = ipc::read(stream);
            let _ = stream.set_nonblocking(false);
            let received = received?;
            let parsed_json = json::parse(&received).ok()?;
            let message = MessageUserToFuzzer::from_json(&parsed_json)?;
            Some(message)
        } else {
            None
        }
    }

    #[cfg(feature = "ui")]
    fn write_to_stream(&mut self, message: &TuiMessage) {
        if let Some(stream) = &mut self.stream {
            ipc::write(stream, &message.to_json().to_string());
        }
    }

    pub fn pause_until_unpause_message(&mut self) {
        #[cfg(feature = "ui")]
        {
            let start_pause = Instant::now();

            self.write_to_stream(&TuiMessage::Paused);
            'waiting_loop: loop {
                match self.read_message_from_user(false) {
                    Some(MessageUserToFuzzer::UnPause) => {
                        self.pause_at_next_event = false;
                        self.write_to_stream(&TuiMessage::UnPaused);
                        break 'waiting_loop;
                    }
                    Some(MessageUserToFuzzer::Pause) => continue 'waiting_loop,
                    Some(MessageUserToFuzzer::Stop) => self.stop(),
                    Some(MessageUserToFuzzer::UnPauseUntilNextEvent) => {
                        self.pause_at_next_event = true;
                        self.write_to_stream(&TuiMessage::UnPaused);
                        break 'waiting_loop;
                    }
                    None => {
                        todo!() //break 'waiting_loop
                    }
                }
            }
            let time_paused = start_pause.elapsed();
            self.checkpoint_instant = self.checkpoint_instant.checked_add(time_paused).unwrap();
            self.initial_instant = self.initial_instant.checked_add(time_paused).unwrap();
        }
    }

    pub fn handle_user_message(&mut self) {
        #[cfg(feature = "ui")]
        {
            match self.read_message_from_user(true) {
                Some(MessageUserToFuzzer::Pause) => {
                    self.pause_until_unpause_message();
                }
                Some(MessageUserToFuzzer::UnPause) => {}
                Some(MessageUserToFuzzer::UnPauseUntilNextEvent) => {}
                Some(MessageUserToFuzzer::Stop) => self.stop(),
                None => {}
            }
        }
    }

    pub fn stop(&mut self) -> ! {
        #[cfg(feature = "ui")]
        self.write_to_stream(&TuiMessage::Stopped);
        self.report_event(FuzzerEvent::Stop, None);
        std::process::exit(TerminationStatus::Success as i32);
    }
}
