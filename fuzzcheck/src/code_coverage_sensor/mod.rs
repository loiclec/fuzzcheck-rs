//! Code coverage analysis

mod leb128;
mod llvm_coverage;
use std::path::Path;

use crate::Feature;

use self::llvm_coverage::{get_counters, get_prf_data, read_covmap, AllCoverage, LLVMCovSections};

/// Records the code coverage of the program and converts it into `Feature`s
/// that the `pool` can understand.
pub struct CodeCoverageSensor {
    pub coverage_counters: AllCoverage,
}

impl CodeCoverageSensor {
    #[no_coverage]
    pub(crate) fn new<E, K>(exclude: E, keep: K) -> Self
    where
        E: Fn(&Path) -> bool,
        K: Fn(&Path) -> bool,
    {
        let exec = std::env::current_exe().expect("could not read current executable");
        let LLVMCovSections { covfun, covmap } = llvm_coverage::get_llvm_cov_sections(&exec);
        let prf_data = unsafe { get_prf_data() };
        let covmap = read_covmap(&covmap, &mut 0);
        let covfun = llvm_coverage::read_covfun(&covfun, &mut 0);
        let covfun = llvm_coverage::process_function_records(covfun);
        let prf_data = llvm_coverage::read_prf_data(&prf_data, &mut 0);
        let mut coverage = AllCoverage::new(covmap, covfun, prf_data);

        coverage.filter_function_by_files(exclude, keep);
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
        coverage_counters.iterate_over_coverage_points(get_counters(), |(index, count)| {
            handle(Feature::new(index, count));
        });
    }
    #[no_coverage]
    pub(crate) unsafe fn clear(&mut self) {
        let slice = get_counters();
        for c in &self.coverage_counters.counters {
            for i in c.counters_range.clone() {
                *slice.get_unchecked_mut(i) = 0;
            }
        }
    }
}
