use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::CodeCoverageSensor;

#[derive(Serialize, Deserialize)]
pub struct CoverageMap {
    functions: Vec<Function>,
}

#[derive(Serialize, Deserialize)]
pub struct Function {
    name: String,
    file: String,
    counters: Vec<Counter>,
    inferred_counters: Vec<InferredCounter>,
}

#[derive(Serialize, Deserialize)]
pub struct InferredCounter {
    regions: Vec<Region>,
    from_counter_ids: Vec<usize>,
}

#[derive(Serialize, Deserialize)]
pub struct Region {
    lines: (usize, usize),
    cols: (usize, usize),
}

#[derive(Serialize, Deserialize)]
pub struct Counter {
    id: usize,
    regions: Vec<Region>,
}

impl CodeCoverageSensor {
    #[coverage(off)]
    pub(crate) fn coverage_map(&self) -> CoverageMap {
        let mut idx = 0;
        let functions = self
            .coverage
            .iter()
            .map(
                #[coverage(off)]
                |coverage| {
                    let f_record = &coverage.function_record;
                    assert!(f_record.filenames.len() == 1);
                    let name = f_record.name_function.clone();
                    let mut counters_by_file = HashMap::<PathBuf, Vec<Counter>>::new();

                    // need to map (expression_idx) -> counter_idx
                    let mut expression_idx_to_counter_idx = HashMap::new();
                    let mut counter_indices_and_regions = vec![];
                    for (i, (e, regions)) in f_record.expressions.iter().enumerate() {
                        if e.add_terms.len() == 1 && e.sub_terms.is_empty() {
                            counter_indices_and_regions.push((idx, regions));
                            expression_idx_to_counter_idx.insert(i, idx);
                            idx += 1;
                        }
                    }
                    for (i, (e, regions)) in f_record.expressions.iter().enumerate() {
                        if !(e.add_terms.len() == 1 && e.sub_terms.is_empty()) && !e.add_terms.is_empty() {
                            counter_indices_and_regions.push((idx, regions));
                            expression_idx_to_counter_idx.insert(i, idx);
                            idx += 1;
                        }
                    }

                    for (idx, regions) in counter_indices_and_regions {
                        // assume that all regions are within one file
                        let file_idx = f_record
                            .file_id_mapping
                            .filename_indices
                            .iter()
                            .position(
                                #[coverage(off)]
                                |idx| *idx == regions[0].filename_index,
                            )
                            .unwrap();
                        let file = f_record.filenames[file_idx].clone();
                        let counter = Counter {
                            id: idx,
                            regions: regions
                                .iter()
                                .map(
                                    #[coverage(off)]
                                    |region| Region {
                                        lines: (region.line_start, region.line_end),
                                        cols: (region.col_start, region.col_end),
                                    },
                                )
                                .collect(),
                        };
                        counters_by_file.entry(file).or_default().push(counter);
                    }
                    // assume there is only one file
                    let (file, counters) = counters_by_file.into_iter().next().unwrap();
                    let inferred_counters = f_record
                        .inferred_expressions
                        .iter()
                        .map(
                            #[coverage(off)]
                            |(regions, from_expr_idxs)| InferredCounter {
                                regions: regions
                                    .iter()
                                    .map(
                                        #[coverage(off)]
                                        |region| Region {
                                            lines: (region.line_start, region.line_end),
                                            cols: (region.col_start, region.col_end),
                                        },
                                    )
                                    .collect(),
                                from_counter_ids: from_expr_idxs
                                    .iter()
                                    .map(
                                        #[coverage(off)]
                                        |idx| expression_idx_to_counter_idx[idx],
                                    )
                                    .collect(),
                            },
                        )
                        .collect::<Vec<_>>();
                    Function {
                        name,
                        file: file.to_str().unwrap().to_owned(),
                        counters,
                        inferred_counters,
                    }
                },
            )
            .collect();
        CoverageMap { functions }
    }
}
