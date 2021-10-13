use crate::fuzzer::PoolStorageIndex;
use crate::fuzzer::TerminationStatus;
use crate::traits::CorpusDelta;
use crate::traits::EmptyStats;
use crate::CSVField;
use crate::ToCSVFields;
use fuzzcheck_common::arg::Arguments;
use fuzzcheck_common::arg::FuzzerCommand;
use fuzzcheck_common::{FuzzerEvent, FuzzerStats};
use owo_colors::OwoColorize;
use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::fmt::Display;
use std::fs;
use std::fs::File;
use std::fs::OpenOptions;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::io::{self, Result};
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;

impl ToCSVFields for FuzzerStats {
    fn csv_headers(&self) -> Vec<CSVField> {
        vec![
            CSVField::String("nbr_iter".to_string()),
            CSVField::String("iter/s".to_string()),
        ]
    }

    fn to_csv_record(&self) -> Vec<CSVField> {
        vec![
            CSVField::Integer(self.total_number_of_runs as isize),
            CSVField::Integer(self.exec_per_s as isize),
        ]
    }
}

pub struct World {
    settings: Arguments,
    initial_instant: Instant,
    checkpoint_instant: Instant,
    /// keeps track of the hash of each input in the corpus, indexed by the Pool key
    pub corpus: HashMap<(PathBuf, PoolStorageIndex), String>,
    pub stats: Option<RefCell<File>>,
}

impl World {
    #[no_coverage]
    pub fn new(settings: Arguments) -> Result<Self> {
        let stats = if let Some(stats_folder) = &settings.stats_folder {
            let now = SystemTime::now();
            let duration_since_epoch = now.duration_since(SystemTime::UNIX_EPOCH).unwrap();
            let name = format!("{}", duration_since_epoch.as_millis());
            std::fs::create_dir_all(stats_folder)?;
            let path = stats_folder.join(name).with_extension("csv");
            let file = OpenOptions::new().create_new(true).append(true).open(path)?;
            Some(RefCell::new(file))
        } else {
            None
        };
        Ok(Self {
            settings,
            initial_instant: std::time::Instant::now(),
            checkpoint_instant: std::time::Instant::now(),
            corpus: HashMap::new(),
            stats,
        })
    }

    #[no_coverage]
    fn hash(&self, input: &[u8]) -> String {
        let mut hasher = DefaultHasher::new();
        input.hash(&mut hasher);
        let hash = hasher.finish();
        let hash = format!("{:x}", hash);
        hash
    }

    #[no_coverage]
    pub fn append_stats_file(&self, fields: &[CSVField]) -> Result<()> {
        if let Some(stats) = &self.stats {
            let mut stats = stats.try_borrow_mut().unwrap();
            stats.write(&CSVField::to_bytes(fields))?;
        }
        Ok(())
    }

    #[no_coverage]
    pub(crate) fn update_corpus(
        &mut self,
        idx: PoolStorageIndex,
        content: Vec<u8>,
        deltas: &[CorpusDelta],
        extension: &str,
    ) -> Result<()> {
        for delta in deltas {
            let CorpusDelta { path, add, remove } = delta;
            for to_remove_key in remove {
                let hash = self.corpus.remove(&(path.to_path_buf(), *to_remove_key)).unwrap();
                self.remove_from_output_corpus(&path, hash.clone(), extension)?;
            }

            if *add {
                let hash = self.hash(&content);
                let _old = self.corpus.insert((path.to_path_buf(), idx), hash.clone());
                self.add_to_output_corpus(&path, hash.clone(), content.clone(), extension)?;
            }
        }

        Ok(())
    }

    #[no_coverage]
    pub fn add_to_output_corpus(&self, path: &Path, name: String, content: Vec<u8>, extension: &str) -> Result<()> {
        if self.settings.corpus_out.is_none() {
            return Ok(());
        }
        let folder = self.settings.corpus_out.as_ref().unwrap().join(path);

        if !folder.is_dir() {
            std::fs::create_dir_all(&folder)?;
        }

        let path = folder.join(name).with_extension(extension);
        fs::write(path, content)?;

        Ok(())
    }

    #[no_coverage]
    pub fn remove_from_output_corpus(&self, path: &Path, name: String, extension: &str) -> Result<()> {
        if self.settings.corpus_out.is_none() {
            return Ok(());
        }
        let corpus = self.settings.corpus_out.as_ref().unwrap().as_path().join(path);

        let path = corpus.join(name).with_extension(extension);
        let _ = fs::remove_file(path);

        Ok(())
    }

    #[no_coverage]
    pub(crate) fn report_event<PoolStats: Display + ToCSVFields>(
        &self,
        event: FuzzerEvent,
        stats: Option<(&FuzzerStats, &PoolStats)>,
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
            FuzzerEvent::DidReadCorpus => {
                println!("{}", "FINISHED READING CORPUS".yellow());
                return;
            }
            FuzzerEvent::CaughtSignal(signal) => println!("\n================ SIGNAL {} ================", signal),

            FuzzerEvent::TestFailure => {
                println!("\n================ TEST FAILED ================");
            }
            FuzzerEvent::Replace(add, sub) => {
                if add != 0 {
                    print!("+{} ", add.yellow());
                } else {
                    print!("   ");
                }
                if sub != 0 {
                    print!("-{}", add.yellow());
                } else {
                    print!("  ");
                }
                print!("\t");
            }
            FuzzerEvent::None => return,
        };
        if let Some((fuzzer_stats, pool_stats)) = stats {
            print!("{} ", fuzzer_stats.total_number_of_runs.yellow());
            print!("{} ", pool_stats.yellow());
            print!("{} {}", "iter/s:".yellow(), fuzzer_stats.exec_per_s.yellow());

            println!();
            let time_since_start = self.initial_instant.elapsed();
            let mut stats_fields = vec![CSVField::Integer(time_since_start.as_millis() as isize)];
            stats_fields.extend(fuzzer_stats.to_csv_record());
            stats_fields.extend(pool_stats.to_csv_record());
            self.append_stats_file(&stats_fields)
                .expect("cannot write to stats file");
        }
    }

    // #[no_coverage]
    // pub fn set_start_instant(&mut self) {
    //     self.initial_instant = Instant::now();
    // }
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
    pub fn read_input_corpus(&self) -> Result<Vec<Vec<u8>>> {
        if self.settings.corpus_in.is_none() {
            return Result::Ok(vec![]);
        }
        let corpus = self.settings.corpus_in.as_ref().unwrap().as_path();
        let mut values = vec![];
        self.read_input_corpus_rec(corpus, &mut values)?;
        Ok(values)
    }
    fn read_input_corpus_rec(&self, corpus: &Path, values: &mut Vec<Vec<u8>>) -> Result<()> {
        if !corpus.exists() {
            return Ok(());
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
                values.push(data);
            }
        }
        Ok(())
    }

    #[no_coverage]
    pub fn read_input_file(&self, file: &Path) -> Result<Vec<u8>> {
        let data = fs::read(file)?;
        Ok(data)
    }

    #[no_coverage]
    pub fn save_artifact(&mut self, content: Vec<u8>, cplx: f64, extension: &str) -> Result<()> {
        let artifacts_folder = self.settings.artifacts_folder.as_ref();
        if artifacts_folder.is_none() {
            return Ok(());
        }
        let artifacts_folder = artifacts_folder.unwrap().as_path();

        if !artifacts_folder.is_dir() {
            std::fs::create_dir_all(artifacts_folder)?;
        }

        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        let hash = hasher.finish();

        let name = if let FuzzerCommand::MinifyInput { .. } | FuzzerCommand::Read { .. } = self.settings.command {
            format!("{:.0}--{:x}", cplx * 100.0, hash)
        } else {
            format!("{:x}", hash)
        };

        let path = artifacts_folder.join(&name).with_extension(extension);
        println!("Failing test case found. Saving at {:?}", path);
        fs::write(path, &content)?;

        Result::Ok(())
    }

    #[no_coverage]
    pub fn stop(&mut self) -> ! {
        self.report_event::<EmptyStats>(FuzzerEvent::Stop, None);
        std::process::exit(TerminationStatus::Success as i32);
    }
}
