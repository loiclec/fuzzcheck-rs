//! Code coverage analysis

mod leb128;
mod llvm_coverage;
#[cfg(feature = "serde_json_serializer")]
mod serialized;

use crate::traits::{SaveToStatsFolder, Sensor};
use std::convert::TryFrom;
use std::path::Path;
use std::{collections::HashMap, path::PathBuf};

use self::llvm_coverage::{get_counters, get_prf_data, read_covmap, Coverage, LLVMCovSections};

/// A sensor that automatically records the code coverage of the program through an array of counters.
///
/// This is the default sensor used by fuzzcheck. It can filter the recorded code coverage so that
/// only some files influence the fuzzer.
///
/// By default, coverage is recorded only for the files whose given paths are relative to the current directory.
/// This is a heuristic to observe only the crate being tested. However, this behaviour can be changed.
/// When creating a new `CodeCoverageSensor`, you can pass a function that determines whether coverage is
/// recorded for a file with a given path.
///
/// ```no_run
/// use fuzzcheck::sensors_and_pools::CodeCoverageSensor;
/// let sensor = CodeCoverageSensor::new(|path| path.is_relative() == true);
/// ```
pub struct CodeCoverageSensor {
    pub(crate) coverage: Vec<Coverage>,
    needs_clearing: Vec<usize>,
    /// The number of code regions observed by the sensor
    pub count_instrumented: usize,
}

impl CodeCoverageSensor {
    #[no_coverage]
    pub fn observing_only_files_from_current_dir() -> Self {
        Self::new(
            #[no_coverage]
            |f| f.is_relative(),
        )
    }
    #[no_coverage]
    pub fn new<K>(keep: K) -> Self
    where
        K: Fn(&Path) -> bool,
    {
        let exec = std::env::current_exe().expect("could not read current executable");
        let LLVMCovSections {
            covfun,
            covmap,
            prf_names,
        } = llvm_coverage::get_llvm_cov_sections(&exec).expect("could not find all relevant LLVM coverage sections");
        let prf_data = unsafe { get_prf_data() };
        let covmap = read_covmap(&covmap, &mut 0).expect("failed to parse LLVM covmap");
        let covfun = llvm_coverage::read_covfun(&covfun, &mut 0).expect("failed to parse LLVM covfun");

        let prf_names = llvm_coverage::read_prf_names(&prf_names, &mut 0).expect("failed to parse LLVM prf_names");
        let mut map = HashMap::new();
        for prf_name in prf_names {
            let name_md5 = md5::compute(prf_name.as_bytes());
            let name_md5 = i64::from_le_bytes(<[u8; 8]>::try_from(&name_md5[0..8]).unwrap());
            map.insert(name_md5, prf_name);
        }

        let covfun = llvm_coverage::process_function_records(covfun, map, &covmap);
        let prf_data = llvm_coverage::read_prf_data(prf_data, &mut 0).expect("failed to parse LLVM prf_data");

        let mut coverage = unsafe { Coverage::new(covfun, prf_data, get_counters()) }
            .expect("failed to properly link the different LLVM coverage sections");
        coverage.drain_filter(
            #[no_coverage]
            |coverage| {
                coverage.single_counters.is_empty()
                    || (coverage.single_counters.len() + coverage.expression_counters.len() < 1)
            },
        );
        Coverage::filter_function_by_files(&mut coverage, keep);

        let mut count_instrumented = 0;
        for coverage in coverage.iter() {
            count_instrumented += coverage.single_counters.len() + coverage.expression_counters.len();
        }
        let needs_clearing = (0..coverage.len()).collect();
        CodeCoverageSensor {
            coverage,
            needs_clearing,
            count_instrumented,
        }
    }

    #[no_coverage]
    unsafe fn clear(&mut self) {
        for &coverage_idx in &self.needs_clearing {
            let coverage = &self.coverage[coverage_idx];
            let slice = std::slice::from_raw_parts_mut(coverage.start_counters, coverage.counters_len);
            for c in slice.iter_mut() {
                *c = 0;
            }
        }
        self.needs_clearing.clear();
    }
}

impl Sensor for CodeCoverageSensor {
    type Observations = Vec<(usize, u64)>;

    #[no_coverage]
    fn start_recording(&mut self) {
        unsafe {
            self.clear();
        }
    }
    #[no_coverage]
    fn stop_recording(&mut self) {}

    #[no_coverage]
    fn get_observations(&mut self) -> Self::Observations {
        self.needs_clearing.clear();
        let mut observations = Vec::with_capacity(self.count_instrumented);
        unsafe {
            let CodeCoverageSensor { coverage, .. } = self;
            let mut index = 0;
            let mut old_observations_len = 0;
            for (i, coverage) in coverage.iter().enumerate() {
                for &single in coverage.single_counters.iter() {
                    if *single != 0 {
                        observations.push((index, *single));
                    }
                    index += 1;
                }
                for expr in &coverage.expression_counters {
                    let computed = expr.compute();
                    if computed != 0 {
                        observations.push((index, computed));
                    }
                    index += 1;
                }
                if observations.len() != old_observations_len {
                    self.needs_clearing.push(i);
                    old_observations_len = observations.len();
                }
            }
        }
        observations
    }
}
impl SaveToStatsFolder for CodeCoverageSensor {
    #[no_coverage]
    fn save_to_stats_folder(&self) -> Vec<(PathBuf, Vec<u8>)> {
        cfg_if::cfg_if! {
            if #[cfg(feature = "serde_json_serializer")] {
                let coverage_map = self.coverage_map();
                let content = serde_json::to_vec(&coverage_map).unwrap();
                vec![(PathBuf::new().join("coverage_sensor.json"), content)]
            } else {
                vec![]
            }
        }
    }
}
