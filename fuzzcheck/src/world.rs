use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::fs;
use std::fs::{File, OpenOptions};
use std::hash::{Hash, Hasher};
use std::io::{self, Result, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use fuzzcheck_common::arg::{Arguments, FuzzerCommand};
use fuzzcheck_common::{FuzzerEvent, FuzzerStats};
use nu_ansi_term::Color;

use crate::fuzzer::{PoolStorageIndex, TerminationStatus};
use crate::traits::{CorpusDelta, SaveToStatsFolder, Stats};
use crate::{CSVField, ToCSV};

impl ToCSV for FuzzerStats {
    #[coverage(off)]
    fn csv_headers(&self) -> Vec<CSVField> {
        vec![
            CSVField::String("nbr_iter".to_string()),
            CSVField::String("iter/s".to_string()),
        ]
    }
    #[coverage(off)]
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
    pub stats_folder: Option<PathBuf>,
}

impl World {
    #[coverage(off)]
    pub fn new(settings: Arguments) -> Result<Self> {
        let (stats, stats_folder) = if let Some(stats_folder) = &settings.stats_folder {
            let now = SystemTime::now();
            let duration_since_epoch = now.duration_since(SystemTime::UNIX_EPOCH).unwrap();
            let name = format!("{}", duration_since_epoch.as_millis());
            let stats_folder = stats_folder.join(name);
            std::fs::create_dir_all(&stats_folder)?;
            let path = stats_folder.join("events").with_extension("csv");
            let file = OpenOptions::new().create_new(true).append(true).open(path)?;
            (Some(RefCell::new(file)), Some(stats_folder))
        } else {
            (None, None)
        };
        Ok(Self {
            settings,
            initial_instant: std::time::Instant::now(),
            checkpoint_instant: std::time::Instant::now(),
            corpus: HashMap::new(),
            stats,
            stats_folder,
        })
    }

    #[coverage(off)]
    fn hash(&self, input: &[u8]) -> String {
        let mut hasher = DefaultHasher::new();
        input.hash(&mut hasher);
        let hash = hasher.finish();
        let hash = format!("{:x}", hash);
        hash
    }

    #[coverage(off)]
    pub fn append_stats_file(&self, fields: &[CSVField]) -> Result<()> {
        if let Some(stats) = &self.stats {
            let mut stats = stats.try_borrow_mut().unwrap();
            stats.write_all(&CSVField::to_bytes(fields))?;
        }
        Ok(())
    }

    #[coverage(off)]
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
                self.remove_from_output_corpus(path, hash.clone(), extension)?;
            }

            if *add {
                let hash = self.hash(&content);
                let _old = self.corpus.insert((path.to_path_buf(), idx), hash.clone());
                self.add_to_output_corpus(path, hash.clone(), content.clone(), extension)?;
            }
        }

        Ok(())
    }

    #[coverage(off)]
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

    #[coverage(off)]
    pub fn remove_from_output_corpus(&self, path: &Path, name: String, extension: &str) -> Result<()> {
        if self.settings.corpus_out.is_none() {
            return Ok(());
        }
        let corpus = self.settings.corpus_out.as_ref().unwrap().as_path().join(path);

        let path = corpus.join(name).with_extension(extension);
        let _ = fs::remove_file(path);

        Ok(())
    }

    #[coverage(off)]
    pub(crate) fn report_event(&self, event: FuzzerEvent, stats: Option<(&FuzzerStats, &dyn Stats)>) {
        // println uses a lock, which may mess up the signal handling
        let time_since_start = self.initial_instant.elapsed();
        let time_since_start_display = {
            let time_since_start_millis = time_since_start.as_millis();
            if time_since_start_millis > 10_000 {
                let time_since_start_seconds = time_since_start.as_secs();
                format!("{}s ", time_since_start_seconds)
            } else {
                format!("{}ms ", time_since_start_millis)
            }
        };
        print!("{} ", time_since_start_display);
        match event {
            FuzzerEvent::Start => {
                println!("{}", Color::Yellow.paint("START"));
                return;
            }
            FuzzerEvent::Pulse => {
                print!("{} ", Color::Yellow.paint("PULSE"));
            }
            FuzzerEvent::Stop => {
                println!("\n======================== STOPPED ========================");
                println!(r#"The fuzzer was stopped."#);
                return;
            }
            FuzzerEvent::End => {
                println!("\n======================== END ========================");
                println!(
                    r#"Fuzzcheck cannot generate more arbitrary values of the input type. This may be
because all possible values under the chosen maximum complexity were tested, or
because the mutator does not know how to generate more values."#
                );
                return;
            }
            FuzzerEvent::CrashNoInput => {
                println!("\n=================== CRASH DETECTED ===================");
                println!(
                    r#"A crash was detected, but the fuzzer cannot recover the crashing input.
This should never happen, and is probably a bug in fuzzcheck. Sorry :("#
                );
                return;
            }
            FuzzerEvent::Done => {
                println!("{}", Color::Yellow.paint("DONE"));
                return;
            }
            FuzzerEvent::DidReadCorpus => {
                println!("{}", Color::Yellow.paint("FINISHED READING CORPUS"));
                return;
            }
            FuzzerEvent::CaughtSignal(signal) => println!("\n================ SIGNAL {} ================", signal),

            FuzzerEvent::TestFailure => {
                println!("\n================ TEST FAILED ================");
            }
            FuzzerEvent::Replace(_, _) => {}
            FuzzerEvent::None => return,
        };
        if let Some((fuzzer_stats, pool_stats)) = stats {
            print!(
                "{} ",
                Color::Yellow.paint(format!("{}", fuzzer_stats.total_number_of_runs))
            );
            print!("{} ", Color::Yellow.paint(format!("{}", pool_stats)));
            print!(
                "{} ",
                Color::Yellow.paint(format!("iter/s {}", fuzzer_stats.exec_per_s))
            );

            println!();
            let mut stats_fields = vec![CSVField::Integer(time_since_start.as_millis() as isize)];
            stats_fields.extend(fuzzer_stats.to_csv_record());
            stats_fields.extend(pool_stats.to_csv_record());
            self.append_stats_file(&stats_fields)
                .expect("cannot write to stats file");
        }
    }

    // #[coverage(off)]
    // pub fn set_start_instant(&mut self) {
    //     self.initial_instant = Instant::now();
    // }
    #[coverage(off)]
    pub fn set_checkpoint_instant(&mut self) {
        self.checkpoint_instant = Instant::now();
    }
    #[coverage(off)]
    pub fn elapsed_time_since_start(&self) -> Duration {
        self.initial_instant.elapsed()
    }
    #[coverage(off)]
    pub fn elapsed_time_since_last_checkpoint(&self) -> usize {
        self.checkpoint_instant.elapsed().as_micros() as usize
    }

    #[coverage(off)]
    pub fn read_input_corpus(&self) -> Result<Vec<Vec<u8>>> {
        if self.settings.corpus_in.is_none() {
            return Result::Ok(vec![]);
        }
        let corpus = self.settings.corpus_in.as_ref().unwrap().as_path();
        let mut values = vec![];
        self.read_input_corpus_rec(corpus, &mut values)?;
        Ok(values)
    }
    #[coverage(off)]
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

    #[coverage(off)]
    pub fn read_input_file(&self, file: &Path) -> Result<Vec<u8>> {
        let data = fs::read(file)?;
        Ok(data)
    }

    #[coverage(off)]
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
        fs::write(&path, &content)?;
        println!("Failing test case found. Saving at {:?}", path);

        Result::Ok(())
    }

    #[coverage(off)]
    pub fn stop(&mut self) -> ! {
        self.report_event(FuzzerEvent::Stop, None);
        std::process::exit(TerminationStatus::Success as i32);
    }

    #[coverage(off)]
    pub fn write_stats_content(&self, contents: Vec<(PathBuf, Vec<u8>)>) -> Result<()> {
        if let Some(stats_folder) = &self.stats_folder {
            for (path, content) in contents {
                let path = stats_folder.join(path);
                fs::write(path, &content)?;
            }
        }
        Ok(())
    }
}
impl SaveToStatsFolder for World {
    #[coverage(off)]
    fn save_to_stats_folder(&self) -> Vec<(PathBuf, Vec<u8>)> {
        cfg_if::cfg_if! {
            if #[cfg(feature = "serde_json_serializer")] {
                let content = serde_json::to_vec(&self.corpus.iter().collect::<Vec<_>>()).unwrap();
                vec![(PathBuf::new().join("world.json"), content)]
            } else {
                vec![]
            }
        }
    }
}
