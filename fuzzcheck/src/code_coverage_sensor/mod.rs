//! Code coverage analysis

mod leb128;
mod llvm_coverage;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::path::Path;

use crate::code_coverage_sensor::llvm_coverage::Coverage;
use crate::Feature;

use self::llvm_coverage::{get_counters, get_prf_data, read_covmap, LLVMCovSections};

/// Records the code coverage of the program and converts it into `Feature`s
/// that the `pool` can understand.
pub struct CodeCoverageSensor {
    pub coverage_counters: Vec<Coverage>,
}

impl CodeCoverageSensor {
    #[no_coverage]
    pub(crate) fn new<E, K>(exclude: E, keep: K) -> Self
    where
        E: Fn(&Path) -> bool,
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
        Coverage::filter_function_by_files(&mut coverage, exclude, keep);

        CodeCoverageSensor {
            coverage_counters: coverage,
        }
    }
    #[no_coverage]
    pub(crate) unsafe fn start_recording(&self) {}
    #[no_coverage]
    pub(crate) unsafe fn stop_recording(&self) {}
    #[no_coverage]
    pub(crate) unsafe fn iterate_over_collected_features<F>(&mut self, mut handle: F)
    where
        F: FnMut(Feature),
    {
        let CodeCoverageSensor { coverage_counters } = self;
        Coverage::iterate_over_coverage_points(&coverage_counters, |(index, count)| {
            handle(Feature::new(index, count));
        });
    }
    #[no_coverage]
    pub(crate) unsafe fn clear(&mut self) {
        for coverage in &self.coverage_counters {
            let slice =
                std::slice::from_raw_parts_mut(coverage.physical_counters_start, coverage.physical_counters_len);
            for c in slice {
                *c = 0;
            }
        }
    }
}
