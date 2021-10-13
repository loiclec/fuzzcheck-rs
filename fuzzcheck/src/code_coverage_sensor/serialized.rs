use std::{collections::HashMap, ops::RangeInclusive, path::PathBuf};

use super::CodeCoverageSensor;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct CoverageMap {
    functions: Vec<Function>,
}

#[derive(Serialize, Deserialize)]
pub struct Function {
    name: String,
    counters: Vec<CountersByFile>,
}

#[derive(Serialize, Deserialize)]
pub struct CountersByFile {
    file: PathBuf,
    counters: Vec<Counter>,
}

#[derive(Serialize, Deserialize)]
pub struct Counter {
    lines: RangeInclusive<usize>,
    cols: RangeInclusive<usize>,
}

impl CodeCoverageSensor {
    #[no_coverage]
    pub fn coverage_map(&self) -> CoverageMap {
        let functions = self
            .coverage
            .iter()
            .map(
                #[no_coverage]
                |coverage| {
                    let f_record = &coverage.function_record;
                    let name = f_record.name_function.clone();
                    let mut regions_by_file = HashMap::<PathBuf, Vec<Counter>>::new();
                    for (_, region) in &f_record.expressions {
                        let file_idx = f_record
                            .file_id_mapping
                            .filename_indices
                            .iter()
                            .position(
                                #[no_coverage]
                                |idx| *idx == region.filename_index,
                            )
                            .unwrap();
                        let file = f_record.filenames[file_idx].clone();
                        let counter = Counter {
                            lines: region.line_start..=region.line_end,
                            cols: region.col_start..=region.col_end,
                        };
                        regions_by_file.entry(file).or_default().push(counter);
                    }
                    let counters = regions_by_file
                        .into_iter()
                        .map(
                            #[no_coverage]
                            |(file, counters)| CountersByFile { file, counters },
                        )
                        .collect();
                    Function { name, counters }
                },
            )
            .collect();
        CoverageMap { functions }
    }
}
