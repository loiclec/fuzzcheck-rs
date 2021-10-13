//! Code coverage analysis

mod leb128;
mod llvm_coverage;
mod serialized;

use crate::traits::Sensor;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::path::Path;

use self::llvm_coverage::{get_counters, get_prf_data, read_covmap, Coverage, LLVMCovSections};

/// Records the code coverage of the program and converts it into `Feature`s
/// that the `pool` can understand.
pub struct CodeCoverageSensor {
    pub coverage: Vec<Coverage>,
    // pub index_ranges: Vec<RangeInclusive<usize>>,
    pub count_instrumented: usize,
}

impl CodeCoverageSensor {
    #[no_coverage]
    pub fn observing_only_files_from_current_dir() -> Self {
        Self::new(|_| true, |f| f.is_relative())
    }
    #[no_coverage]
    pub fn new<E, K>(exclude: E, keep: K) -> Self
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
        // coverage.drain_filter(|coverage| coverage.single_counters.len() + coverage.expression_counters.len() <= 1);
        Coverage::filter_function_by_files(&mut coverage, exclude, keep);

        let mut count_instrumented = 0;
        for coverage in coverage.iter() {
            count_instrumented += coverage.single_counters.len() + coverage.expression_counters.len();
        }
        CodeCoverageSensor {
            coverage,
            count_instrumented,
        }
    }

    #[no_coverage]
    unsafe fn clear(&mut self) {
        for coverage in &mut self.coverage {
            let slice = std::slice::from_raw_parts_mut(coverage.start_counters, coverage.counters_len);
            for c in slice.iter_mut() {
                *c = 0;
            }
        }
    }
}
impl Sensor for CodeCoverageSensor {
    type ObservationHandler<'a> = &'a mut dyn FnMut((usize, u64));

    #[no_coverage]
    fn start_recording(&mut self) {
        unsafe {
            self.clear();
        }
    }
    #[no_coverage]
    fn stop_recording(&mut self) {}

    #[no_coverage]
    fn iterate_over_observations(&mut self, handler: Self::ObservationHandler<'_>) {
        unsafe {
            let CodeCoverageSensor { coverage, .. } = self;
            let mut index = 0;
            for coverage in coverage {
                let single = *coverage.single_counters.get_unchecked(0);
                if *single == 0 {
                    // that happens kind of a lot? not sure it is worth simplifying
                    index += coverage.single_counters.len() + coverage.expression_counters.len();
                    continue;
                } else {
                    handler((index, *single));
                }
                index += 1;
                for &single in coverage.single_counters.iter().skip(1) {
                    if *single != 0 {
                        handler((index, *single));
                    }
                    index += 1;
                }
                for expr in &coverage.expression_counters {
                    let computed = expr.compute();
                    if computed != 0 {
                        handler((index, computed));
                    }
                    index += 1;
                }
            }
        }
    }
}
