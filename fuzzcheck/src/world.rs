use crate::traits::CorpusDelta;
use crate::traits::EmptyStats;
use crate::{fuzzer::TerminationStatus, traits::Serializer};
use fuzzcheck_common::arg::Arguments;
use fuzzcheck_common::arg::FuzzerCommand;
use fuzzcheck_common::{FuzzerEvent, FuzzerStats};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::fmt::Display;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::{self, Result};
use std::path::Path;
use std::time::Duration;
use std::time::Instant;
use owo_colors::OwoColorize;

pub struct World<S: Serializer, CorpusKey: Hash + Eq> {
    settings: Arguments,
    initial_instant: Instant,
    checkpoint_instant: Instant,
    pub serializer: S,
    /// keeps track of the hash of each input in the corpus, indexed by the Pool key
    pub corpus: HashMap<CorpusKey, String>,
}

impl<S: Serializer, CorpusKey: Hash + Eq> World<S, CorpusKey> {
    #[no_coverage]
    pub fn new(serializer: S, settings: Arguments) -> Self {
        Self {
            settings,
            initial_instant: std::time::Instant::now(),
            checkpoint_instant: std::time::Instant::now(),
            serializer,
            corpus: HashMap::new(),
        }
    }

    #[no_coverage]
    fn hash_and_string_of_input(&self, input: &S::Value) -> (String, Vec<u8>) {
        let input = self.serializer.to_data(input);
        let mut hasher = DefaultHasher::new();
        input.hash(&mut hasher);
        let hash = hasher.finish();
        let hash = format!("{:x}", hash);
        (hash, input)
    }

    #[no_coverage]
    pub(crate) fn update_corpus(&mut self, delta: CorpusDelta<S::Value, CorpusKey>) -> Result<()> {
        let CorpusDelta { path, add, remove } = delta;
        for to_remove_key in remove {
            let hash = self.corpus.remove(&to_remove_key).unwrap();
            self.remove_from_output_corpus(&path, hash.clone())?;
        }

        if let Some((content, key)) = add {
            let (hash, input) = self.hash_and_string_of_input(&content);
            let _old = self.corpus.insert(key, hash.clone());
            self.add_to_output_corpus(&path, hash.clone(), input.clone())?;
        }

        Ok(())
    }

    #[no_coverage]
    pub fn add_to_output_corpus(&self, path: &Path, name: String, content: Vec<u8>) -> Result<()> {
        if self.settings.corpus_out.is_none() {
            return Ok(());
        }
        let folder = self.settings.corpus_out.as_ref().unwrap().join(path);

        if !folder.is_dir() {
            std::fs::create_dir_all(&folder)?;
        }

        let path = folder.join(name).with_extension(self.serializer.extension());
        fs::write(path, content)?;

        Ok(())
    }

    #[no_coverage]
    pub fn remove_from_output_corpus(&self, path: &Path, name: String) -> Result<()> {
        if self.settings.corpus_out.is_none() {
            return Ok(());
        }
        let corpus = self.settings.corpus_out.as_ref().unwrap().as_path().join(path);

        let path = corpus.join(name).with_extension(self.serializer.extension());
        let _ = fs::remove_file(path);

        Ok(())
    }

    #[no_coverage]
    pub(crate) fn report_event<PoolStats: Display>(
        &self,
        event: FuzzerEvent,
        stats: Option<(&FuzzerStats, PoolStats)>,
    ) {
        // println uses a lock, which may mess up the signal handling
        match event {
            FuzzerEvent::Start => {
                println!("{}", "START".yellow());
                return;
            }
            FuzzerEvent::Pulse => {
                print!("{}\t", "PULSE".yellow());
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
                println!("{}", "DONE".yellow());
                return;
            }
            FuzzerEvent::New => print!("{}\t", "NEW".yellow()),
            FuzzerEvent::Remove(count) => print!("{} {}\t", "REMOVE".yellow(), count.yellow()),
            FuzzerEvent::DidReadCorpus => {
                println!("{}", "FINISHED READING CORPUS".yellow());
                return;
            }
            FuzzerEvent::CaughtSignal(signal) => println!("\n================ SIGNAL {} ================", signal),

            FuzzerEvent::TestFailure => {
                println!("\n================ TEST FAILED ================");
            }
            FuzzerEvent::Replace(count) => {
                print!("{} {}\t", "RPLC".yellow(), count.yellow());
            }
            FuzzerEvent::None => return,
        };
        if let Some((fuzzer_stats, pool_stats)) = stats {
            print!("{} ", fuzzer_stats.total_number_of_runs.yellow());
            print!("{} ", pool_stats.yellow());
            print!("{} {}", "iter/s:".yellow(), fuzzer_stats.exec_per_s.yellow());

            println!();
        }
    }

    #[no_coverage]
    pub fn set_start_instant(&mut self) {
        self.initial_instant = Instant::now();
    }
    #[no_coverage]
    pub fn set_checkpoint_instant(&mut self) {
        self.checkpoint_instant = Instant::now();
    }
    #[no_coverage]
    pub fn elapsed_time_since_start(&self) -> Duration {
        self.initial_instant.elapsed()
    }
    #[no_coverage]
    pub fn elapsed_time_since_last_checkpoint(&self) -> usize {
        self.checkpoint_instant.elapsed().as_micros() as usize
    }

    #[no_coverage]
    pub fn read_input_corpus(&self) -> Result<Vec<S::Value>> {
        if self.settings.corpus_in.is_none() {
            return Result::Ok(vec![]);
        }
        let corpus = self.settings.corpus_in.as_ref().unwrap().as_path();
        let mut values = vec![];
        self.read_input_corpus_rec(corpus, &mut values)?;
        Ok(values)
    }
    fn read_input_corpus_rec(&self, corpus: &Path, values: &mut Vec<S::Value>) -> Result<()> {
        if !corpus.exists() {
            return Ok(())
        }
        if !corpus.is_dir() {
            return Result::Err(io::Error::new(
                io::ErrorKind::Other,
                "The corpus path is not a directory.",
            ));
        }
        for entry in fs::read_dir(corpus)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                self.read_input_corpus_rec(&path, values)?;
            } else {
                let data = fs::read(path)?;
                if let Some(i) = self.serializer.from_data(&data) {
                    values.push(i);
                }
            }
        }
        Ok(())
    }

    #[no_coverage]
    pub fn read_input_file(&self, file: &Path) -> Result<S::Value> {
        let data = fs::read(file)?;
        if let Some(input) = self.serializer.from_data(&data) {
            Ok(input)
        } else {
            Result::Err(io::Error::new(
                io::ErrorKind::Other,
                "The file could not be decoded into a valid input.",
            ))
        }
    }

    #[no_coverage]
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
        let content = self.serializer.to_data(input);
        content.hash(&mut hasher);
        let hash = hasher.finish();

        let name = if let FuzzerCommand::MinifyInput { .. } | FuzzerCommand::Read { .. } = self.settings.command {
            format!("{:.0}--{:x}", cplx * 100.0, hash)
        } else {
            format!("{:x}", hash)
        };

        let path = artifacts_folder.join(&name).with_extension(self.serializer.extension());
        println!("Saving at {:?}", path);
        fs::write(path, &content)?;

        Result::Ok(())
    }

    #[no_coverage]
    pub fn stop(&mut self) -> ! {
        self.report_event::<EmptyStats>(FuzzerEvent::Stop, None);
        std::process::exit(TerminationStatus::Success as i32);
    }
}
