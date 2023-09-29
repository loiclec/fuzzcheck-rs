use std::collections::{HashMap, HashSet};
use std::convert::TryFrom;
use std::path::{Path, PathBuf};

use flate2::Status;
use object::{Object, ObjectSection};

use super::leb128;

type CovMap = HashMap<[u8; 8], Vec<String>>;

extern "C" {
    pub(crate) fn get_start_instrumentation_counters() -> *mut u64;
    pub(crate) fn get_end_instrumentation_counters() -> *mut u64;
    pub(crate) fn get_start_prf_data() -> *const u8;
    pub(crate) fn get_end_prf_data() -> *const u8;
    pub(crate) fn get_start_prf_names() -> *const u8;
    pub(crate) fn get_end_prf_names() -> *const u8;
}

#[coverage(off)]
pub unsafe fn get_counters() -> &'static mut [u64] {
    let start = get_start_instrumentation_counters();
    let end = get_end_instrumentation_counters();
    let len = end.offset_from(start) as usize;
    std::slice::from_raw_parts_mut(start, len)
}
#[coverage(off)]
pub unsafe fn get_prf_data() -> &'static [u8] {
    let start = get_start_prf_data();
    let end = get_end_prf_data();
    let len = end.offset_from(start) as usize;
    std::slice::from_raw_parts(start, len)
}
#[coverage(off)]
pub unsafe fn get_prf_names() -> &'static [u8] {
    let start = get_start_prf_names();
    let end = get_end_prf_names();
    let len = end.offset_from(start) as usize;
    std::slice::from_raw_parts(start, len)
}

pub struct LLVMCovSections {
    pub covfun: Vec<u8>,
    pub covmap: Vec<u8>,
    pub prf_names: Vec<u8>,
}

#[coverage(off)]
pub fn get_llvm_cov_sections(path: &Path) -> Result<LLVMCovSections, ReadCovMapError> {
    let bin_data = std::fs::read(path).map_err(
        #[coverage(off)]
        |_| ReadCovMapError::CannotReadObjectFile {
            path: path.to_path_buf(),
        },
    )?;
    let obj_file = object::File::parse(&*bin_data).map_err(
        #[coverage(off)]
        |_| ReadCovMapError::CannotReadObjectFile {
            path: path.to_path_buf(),
        },
    )?;
    let covmap = obj_file
        .section_by_name("__llvm_covmap")
        .ok_or(ReadCovMapError::CannotFindSection {
            section: CovMapSection::CovMap,
        })?
        .data()
        .unwrap()
        .to_vec();
    let covfun = obj_file
        .section_by_name("__llvm_covfun")
        .ok_or(ReadCovMapError::CannotFindSection {
            section: CovMapSection::CovFun,
        })?
        .data()
        .unwrap()
        .to_vec();
    Ok(LLVMCovSections {
        covfun,
        covmap,
        prf_names: unsafe { get_prf_names() }.to_vec(),
    })
}

#[coverage(off)]
fn read_counter(counter: usize) -> RawCounter {
    let mask_tag = 0b11;
    let zero_kind = 0b0;
    let reference_kind = 0b01;
    let subtraction_expression_kind = 0b10;
    let addition_expression_kind = 0b11;
    let kind_bits = counter & mask_tag;
    let mask_id = !mask_tag;
    let id = (counter & mask_id) as u32 >> 2;

    if kind_bits == zero_kind {
        return RawCounter::Zero;
    }
    if kind_bits == reference_kind {
        RawCounter::Counter { idx: id as usize }
    } else if kind_bits == addition_expression_kind {
        RawCounter::Expression {
            operation_sign: Sign::Positive,
            idx: id as usize,
        }
    } else if kind_bits == subtraction_expression_kind {
        RawCounter::Expression {
            operation_sign: Sign::Negative,
            idx: id as usize,
        }
    } else {
        unreachable!()
    }
}

#[coverage(off)]
fn read_leb_usize(slice: &[u8], idx: &mut usize) -> usize {
    assert!(!slice.is_empty());
    let (result, pos) = leb128::read_u64_leb128(&slice[*idx..]);
    *idx += pos;
    result as usize
}

#[coverage(off)]
fn read_i64(slice: &[u8], idx: &mut usize) -> i64 {
    assert!(slice.len() >= 8);
    let subslice = <[u8; 8]>::try_from(&slice[*idx..*idx + 8]).unwrap();
    let x = i64::from_le_bytes(subslice);
    *idx += 8;
    x
}
#[coverage(off)]
fn read_u64(slice: &[u8], idx: &mut usize) -> u64 {
    assert!(slice.len() >= 8);
    let subslice = <[u8; 8]>::try_from(&slice[*idx..*idx + 8]).unwrap();
    let x = u64::from_le_bytes(subslice);
    *idx += 8;
    x
}
#[coverage(off)]
fn read_i32(slice: &[u8], idx: &mut usize) -> i32 {
    assert!(slice.len() >= 4);
    let subslice = <[u8; 4]>::try_from(&slice[*idx..*idx + 4]).unwrap();
    let x = i32::from_le_bytes(subslice);
    *idx += 4;
    x
}
#[coverage(off)]
fn read_i16(slice: &[u8], idx: &mut usize) -> i16 {
    assert!(slice.len() >= 2);
    let subslice = <[u8; 2]>::try_from(&slice[*idx..*idx + 2]).unwrap();
    let x = i16::from_le_bytes(subslice);
    *idx += 2;
    x
}
#[coverage(off)]
fn read_u32(slice: &[u8], idx: &mut usize) -> u32 {
    assert!(slice.len() >= 4);
    let subslice = <[u8; 4]>::try_from(&slice[*idx..*idx + 4]).unwrap();
    let x = u32::from_le_bytes(subslice);
    *idx += 4;
    x
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionIdentifier {
    pub name_md5: i64,
    pub structural_hash: u64,
}

#[derive(Clone, Debug)]
pub struct FunctionRecordHeader {
    pub id: FunctionIdentifier,
    pub hash_translation_unit: [u8; 8],
    length_encoded_data: usize,
}

#[coverage(off)]
fn read_first_function_record_fields(covfun: &[u8], idx: &mut usize) -> FunctionRecordHeader {
    let name_md5 = read_i64(covfun, idx);
    let length_encoded_data = read_i32(covfun, idx) as usize;
    let structural_hash = read_u64(covfun, idx);
    let hash_translation_unit = <[u8; 8]>::try_from(&covfun[*idx..*idx + 8]).unwrap();
    *idx += 8;
    FunctionRecordHeader {
        id: FunctionIdentifier {
            name_md5,
            structural_hash,
        },
        hash_translation_unit,
        length_encoded_data,
    }
}

#[derive(Clone, Debug)]
pub struct FileIDMapping {
    pub filename_indices: Vec<usize>,
}

#[coverage(off)]
fn read_file_id_mapping(covfun: &[u8], idx: &mut usize) -> FileIDMapping {
    assert!(!covfun.is_empty());
    let num_indices = read_leb_usize(covfun, idx);
    let mut filename_indices = Vec::new();
    for _ in 0..num_indices {
        filename_indices.push(read_leb_usize(covfun, idx));
    }
    FileIDMapping { filename_indices }
}

#[coverage(off)]
fn read_coverage_expressions(covfun: &[u8], idx: &mut usize) -> Vec<RawExpression> {
    assert!(!covfun.is_empty());

    let num_expressions = read_leb_usize(covfun, idx);
    let mut result = Vec::with_capacity(num_expressions);

    for _ in 0..num_expressions {
        let lhs = read_counter(read_leb_usize(covfun, idx));
        let rhs = read_counter(read_leb_usize(covfun, idx));
        result.push(RawExpression { lhs, rhs });
    }
    result
}

#[derive(Clone, Debug)]
pub struct MappingRegion {
    pub filename_index: usize,
    pub line_start: usize,
    pub line_end: usize,
    pub col_start: usize,
    pub col_end: usize,
}

#[coverage(off)]
fn read_mapping_regions(
    covfun: &[u8],
    idx: &mut usize,
    filename_indices: &[usize],
) -> Result<Vec<(RawCounter, MappingRegion)>, ReadCovMapError> {
    assert!(!covfun.is_empty());
    let mut result = Vec::new();
    // the reference says we should read this number, but it doesn't actually exist
    // and their example doesn't have it either, so I have included here and commented it out
    // let num_regions_arrays = read_leb_usize(covfun, idx);
    // assert_eq!(num_regions_arrays, filename_indices.len());
    for &filename_index in filename_indices {
        let num_regions = read_leb_usize(covfun, idx);
        let mut prev_line_start = 0;
        for _ in 0..num_regions {
            let raw_header = read_leb_usize(covfun, idx);
            let header = read_counter(raw_header); //read_leb_usize(covfun, idx)); // counter or pseudo-counter
            match header {
                RawCounter::Zero if raw_header != 0 => {
                    // TODO: interpret the pseudo counter
                    return Err(ReadCovMapError::PseudoCountersNotSupportedYet { raw_header });
                }
                _ => {}
            }
            let delta_line_start = read_leb_usize(covfun, idx);
            let col_start = read_leb_usize(covfun, idx);
            let num_lines = read_leb_usize(covfun, idx);
            let col_end = read_leb_usize(covfun, idx);

            let line_start = prev_line_start + delta_line_start;
            let line_end = line_start + num_lines;
            prev_line_start = line_start;
            let file_region = MappingRegion {
                filename_index,
                line_start,
                line_end,
                col_start,
                col_end,
            };

            result.push((header, file_region));
        }
    }

    Ok(result)
}

#[coverage(off)]
pub fn read_covfun(covfun: &[u8]) -> Result<Vec<RawFunctionCounters>, ReadCovMapError> {
    let mut results = Vec::new();
    let mut idx = 0;
    while idx < covfun.len() {
        let function_record_header = read_first_function_record_fields(covfun, &mut idx);
        let idx_before_encoding_data = idx;

        // somehow, length_encoded_data == 0 is possible!
        let (file_id_mapping, expressions, counters) = if function_record_header.length_encoded_data == 0 {
            (
                FileIDMapping {
                    filename_indices: vec![],
                },
                vec![],
                vec![],
            )
        } else {
            let file_id_mapping = read_file_id_mapping(covfun, &mut idx);
            let expressions = read_coverage_expressions(covfun, &mut idx);
            let counters = read_mapping_regions(covfun, &mut idx, &file_id_mapping.filename_indices)?;
            (file_id_mapping, expressions, counters)
        };

        if idx_before_encoding_data + function_record_header.length_encoded_data != idx {
            return Err(ReadCovMapError::InconsistentLengthOfEncodedData {
                section: CovMapSection::CovFun,
            });
        }

        let padding = if idx < covfun.len() && idx % 8 != 0 {
            8 - idx % 8
        } else {
            0
        };
        idx += padding;

        if function_record_header.length_encoded_data == 0 {
            assert_eq!(function_record_header.id.structural_hash, 0);
        }
        if function_record_header.id.structural_hash == 0 {
            // dummy function, ignore
            continue;
        }
        results.push(RawFunctionCounters {
            header: function_record_header,
            file_id_mapping,
            expression_list: expressions,
            counters_list: counters,
        });
    }

    Ok(results)
}

pub struct PrfData {
    pub function_id: FunctionIdentifier,
    number_of_counters: usize,
}

#[coverage(off)]
pub fn read_prf_data(prf_data: &[u8]) -> Result<Vec<PrfData>, ReadCovMapError> {
    // Read the prf_data section.
    //
    // The problem is that there is no clear reference for it, and its format can be updated by newer LLVM versions
    //
    // Look at this commit: https://github.com/llvm/llvm-project/commit/24c615fa6b6b7910c8743f9044226499adfac4e6
    // (as well as the commit it references) for a few of the files involved in generating the prf_data section, which
    // can then be used to implement this function
    //
    // In particular, look at InstrProfData.inc

    let mut counts = Vec::new();
    let mut idx = 0;

    while idx < prf_data.len() {
        let name_md5 = read_i64(prf_data, &mut idx);
        let structural_hash = read_u64(prf_data, &mut idx);
        let function_id = FunctionIdentifier {
            name_md5,
            structural_hash,
        };
        let _relative_counter_ptr = read_u64(prf_data, &mut idx);
        let _function_ptr = read_u64(prf_data, &mut idx);
        let _values = read_u64(prf_data, &mut idx); // values are only used for PGO, not coverage instrumentation

        // u32 counters
        let nbr_counters = read_u32(prf_data, &mut idx);

        if structural_hash == 0 {
            // it is a dummy function, so it doesn't have counters
            // 1 counter seems to be the minimum for some reason
            assert!(nbr_counters <= 1);
        }
        // u16 but aligned
        let _num_value_site = read_i16(prf_data, &mut idx); // this is used for PGO only, I think
        idx += 2; // alignment

        // This is no longer a valid check with LLVM 14.0, I think?
        // Maybe due to:
        //     https://github.com/llvm/llvm-project/commit/a1532ed27582038e2d9588108ba0fe8237f01844
        //     https://github.com/llvm/llvm-project/commit/24c615fa6b6b7910c8743f9044226499adfac4e6
        // if let Some((prv_counter_pointer, prv_nbr_counters)) = prv_counter_pointer_and_nbr_counters {
        //     if prv_counter_pointer + 8 * prv_nbr_counters != counter_ptr {
        //         return Err(
        //             ReadCovMapError::InconsistentCounterPointersAndLengths {
        //                 prev_pointer: prv_counter_pointer as usize,
        //                 length: prv_nbr_counters as usize,
        //                 cur_pointer: counter_ptr as usize,
        //             }
        //         );
        //     }
        // }
        // prv_counter_pointer_and_nbr_counters = Some((counter_ptr, nbr_counters as u64));

        counts.push(PrfData {
            function_id,
            number_of_counters: nbr_counters as usize,
        });
    }

    Ok(counts)
}

#[coverage(off)]
fn read_func_names(slice: &[u8], names: &mut Vec<String>) -> Result<(), ReadCovMapError> {
    let slices = slice.split(
        #[coverage(off)]
        |&x| x == 0x01,
    );
    for slice in slices {
        let string = String::from_utf8(slice.to_vec()).map_err(
            #[coverage(off)]
            |_| ReadCovMapError::CannotParseUTF8 {
                section: CovMapSection::PrfNames,
            },
        )?;
        names.push(string);
    }
    Ok(())
}

#[coverage(off)]
pub fn read_prf_names(slice: &[u8], idx: &mut usize) -> Result<Vec<String>, ReadCovMapError> {
    let mut names = Vec::new();
    while *idx < slice.len() {
        let length_uncompressed = read_leb_usize(slice, idx);
        let length_compressed = read_leb_usize(slice, idx);
        if length_compressed == 0 {
            read_func_names(&slice[*idx..*idx + length_uncompressed], &mut names)?;
            *idx += length_uncompressed;
        } else {
            let mut decompressed = vec![0; length_uncompressed];
            let mut decompress = flate2::Decompress::new(true);
            let decompress_result = decompress.decompress(
                &slice[*idx..*idx + length_compressed],
                &mut decompressed,
                flate2::FlushDecompress::Finish,
            );
            if !matches!(decompress_result, Ok(flate2::Status::StreamEnd)) {
                return Err(ReadCovMapError::FailedToDecompress {
                    section: CovMapSection::PrfNames,
                    decompress_result,
                });
            }

            *idx += length_compressed;
            read_func_names(&decompressed, &mut names)?;
        }
    }
    Ok(names)
}
#[derive(Debug)]
pub struct PartialFunctionRecord {
    pub name_function: String,
    pub filenames: Vec<PathBuf>,
    pub counters: RawFunctionCounters,
}

#[derive(Clone, Debug)]
pub struct FunctionRecord {
    pub header: FunctionRecordHeader,
    pub file_id_mapping: FileIDMapping,
    pub expressions: Vec<(ExpandedExpression, Vec<MappingRegion>)>,
    pub inferred_expressions: Vec<(Vec<MappingRegion>, Vec<usize>)>,
    pub name_function: String,
    pub filenames: Vec<PathBuf>,
}

#[coverage(off)]
pub fn filter_covfun(
    records: Vec<RawFunctionCounters>,
    prf_names: HashMap<i64, String>,
    covmap: &CovMap,
    keep: impl Fn(&Path, &str) -> bool,
) -> Vec<PartialFunctionRecord> {
    records
        .into_iter()
        .filter_map(
            #[coverage(off)]
            |function_counters| {
                let name_function = prf_names[&function_counters.header.id.name_md5].clone();
                let name_function = rustc_demangle::demangle(&name_function).to_string();

                let filenames = &covmap[&function_counters.header.hash_translation_unit];
                let mut filepaths = Vec::new();
                for idx in function_counters.file_id_mapping.filename_indices.iter() {
                    let filename = &filenames[*idx];
                    let filepath = Path::new(filename).to_path_buf();
                    if !keep(&filepath, name_function.as_str()) {
                        return None;
                    }
                    filepaths.push(filepath);
                }

                Some(PartialFunctionRecord {
                    name_function,
                    filenames: filepaths,
                    counters: function_counters,
                })
            },
        )
        .collect()
}

#[coverage(off)]
pub fn process_function_records(records: Vec<PartialFunctionRecord>) -> Vec<FunctionRecord> {
    let mut all_expressions = Vec::new();
    for function_record in records {
        let mut expressions: Vec<(ExpandedExpression, Vec<MappingRegion>)> = vec![];
        // map from expanded expression to an index in `expressions`
        let mut expressions_map: HashMap<ExpandedExpression, usize> = HashMap::new();
        let mut all_subexpressions = HashSet::new();

        // NOTE: the order with which the `expressions` variable is built matters
        for (raw_counter, mapping_region) in function_record.counters.counters_list.iter() {
            let mut expanded = ExpandedExpression::default();
            expanded.push_counter(
                raw_counter,
                Sign::Positive,
                &function_record.counters,
                &mut all_subexpressions,
            );
            expanded.sort(); // sort them to canonicalise their representation
            all_subexpressions.insert(expanded.clone());
            if let Some(idx) = expressions_map.get(&expanded) {
                expressions[*idx].1.push(mapping_region.clone());
            } else {
                expressions_map.insert(expanded.clone(), expressions.len());
                expressions.push((expanded.clone(), vec![mapping_region.clone()]));
            }
        }

        let mut to_delete: HashMap<usize, HashSet<ExpandedExpression>> = HashMap::new();

        'outer: for (i, (e1, _)) in expressions.iter().enumerate() {
            if to_delete.contains_key(&i) {
                continue 'outer;
            };
            if e1.add_terms.is_empty() {
                continue 'outer;
            }
            'inner: for (j, (e2, _)) in expressions.iter().enumerate() {
                if i == j {
                    continue 'inner;
                }
                // the below means that every add_term in e1 is also included in e2
                // in other words, the add_terms of e2 are a superset of those of e1
                for c1 in &e1.add_terms {
                    if !e2.add_terms.contains(c1) {
                        continue 'inner;
                    }
                }
                // the below means that every sub_term in e2 is also included in e1
                // in other words, the sub_terms of e1 are a superset of those of e2

                for c2 in &e2.sub_terms {
                    if !e1.sub_terms.contains(c2) {
                        continue 'inner;
                    }
                }

                // for example
                // e1: [1, 2, 3] [4, 5]
                // e2: [1, 2, 3, 6] [4]

                // so if e1 > 0, it follows that e2 > 0
                // so reaching e1 is a sufficient condition to reaching e2

                // we keep track of all sufficient conditions for all deleted expressions
                let mut sufficient_expressions = vec![e1.clone()];
                // also take into consideration the difference of sub terms between e1 and e2
                // e.g. e1 = [c1] [c3, diff_sub_terms]
                //      e2 = [c1, diff_add_terms] [c3]
                // whenever e1 > 0, then e2 > 0
                // diff_sub_terms = [diff_sub_terms]
                // whenever e1 = 0 && diff_sub_terms > 0, then we know e2 > 0 as well
                // otherwise, if e1 = 0 && diff_sub_terms > 0 and e2 = 0, we have:
                // e1 = c1 - c3 - diff_sub_terms = 0 => c1 = c3 + diff_sub_terms
                // e2 = c1 + diff_add_terms - c3 = 0 => diff_add_terms + diff_sub_terms = 0
                // but diff_sub_terms > 0 and diff_add_terms >= 0 ! ---> contradiction
                let mut diff_sub_terms = ExpandedExpression::default();
                for &sub_term in &e1.sub_terms {
                    if !e2.sub_terms.contains(&sub_term) {
                        diff_sub_terms.add_terms.push(sub_term);
                    }
                }
                'q: for (e_suff, _) in expressions.iter() {
                    if e_suff.add_terms.is_empty() {
                        continue 'q;
                    }
                    if !e_suff.sub_terms.is_empty() {
                        continue 'q;
                    }
                    // all terms in e_suff.add_terms must be in diff_sub_terms.add_terms
                    // at least one term in diff_sub_terms.add_terms must be in e_suff.add_terms
                    // e.g.
                    // diff_sub_terms : [c4]
                    // e_suff : [c4]
                    // or
                    // diff_sub_terms: [c4, c5]
                    // e_suff: [c5]
                    for c1 in &e_suff.add_terms {
                        if !diff_sub_terms.add_terms.contains(c1) {
                            continue 'q;
                        }
                    }
                    sufficient_expressions.push(e_suff.clone());
                }
                to_delete.entry(j).or_default().extend(sufficient_expressions);
            }
        }

        let mut to_delete = to_delete.into_iter().collect::<Vec<_>>();
        to_delete.sort_by(
            #[coverage(off)]
            |a, b| b.0.cmp(&a.0),
        );
        let mut deleted = vec![];
        for (e_idx, sufficient_expressions) in to_delete {
            deleted.push((expressions.remove(e_idx).1, sufficient_expressions));
        }

        let mut expression_to_index = HashMap::new();
        for (i, (e, _)) in expressions.iter().enumerate() {
            expression_to_index.insert(e, i);
        }
        let inferred_expressions = deleted
            .into_iter()
            .filter(
                #[coverage(off)]
                |(regions, _)| !regions.is_empty(),
            )
            .map(
                #[coverage(off)]
                |(regions, suff_expressions)| {
                    let suff_expressions = suff_expressions
                        .into_iter()
                        .filter_map(
                            #[coverage(off)]
                            |e| expression_to_index.get(&e).copied(),
                        )
                        .collect::<Vec<_>>();
                    (regions, suff_expressions)
                },
            )
            .collect::<Vec<_>>();

        all_expressions.push(FunctionRecord {
            header: function_record.counters.header,
            file_id_mapping: function_record.counters.file_id_mapping,
            expressions,
            inferred_expressions,
            name_function: function_record.name_function,
            filenames: function_record.filenames,
        });
    }
    all_expressions
}

#[derive(Debug)]
pub enum CovMapSection {
    CovFun,
    CovMap,
    // PrfData,
    PrfNames,
}

#[derive(Debug)]
pub enum ReadCovMapError {
    InconsistentLengthOfEncodedData {
        section: CovMapSection,
    },
    CannotReadObjectFile {
        path: PathBuf,
    },
    CannotFindSection {
        section: CovMapSection,
    },
    PseudoCountersNotSupportedYet {
        raw_header: usize,
    },
    FailedToDecompress {
        section: CovMapSection,
        decompress_result: Result<Status, flate2::DecompressError>,
    },
    NumberOfFilenamesDoesNotMatch {
        actual: usize,
        expected: usize,
    },
    InvalidVersion(i32),
    CannotParseUTF8 {
        section: CovMapSection,
    },
}

#[coverage(off)]
/// Reads the contents of the LLVM coverage map, returning an error if this is
/// not possible.
pub fn read_covmap(covmap: &[u8], idx: &mut usize) -> Result<CovMap, ReadCovMapError> {
    let mut translation_unit_map = HashMap::new();
    while *idx < covmap.len() {
        let _always_0 = read_i32(covmap, idx);
        let length_encoded_data = read_i32(covmap, idx) as usize;
        let _always_0 = read_i32(covmap, idx);
        let version = read_i32(covmap, idx);
        if (3..=5).contains(&version) == false {
            return Err(ReadCovMapError::InvalidVersion(version));
        }

        let encoded_data = &covmap[*idx..*idx + length_encoded_data];
        let filenames = read_list_filenames(encoded_data, &mut 0)?;
        let hash_encoded_data = md5::compute(encoded_data);
        let hash_encoded_data = <[u8; 8]>::try_from(&hash_encoded_data[0..8]).unwrap();

        translation_unit_map.insert(hash_encoded_data, filenames);

        *idx += length_encoded_data;
        let padding = if *idx < covmap.len() && *idx % 8 != 0 {
            8 - *idx % 8
        } else {
            0
        };
        *idx += padding;
    }
    Ok(translation_unit_map)
}

#[coverage(off)]
pub fn read_list_filenames(slice: &[u8], idx: &mut usize) -> Result<Vec<String>, ReadCovMapError> {
    let nbr_filenames = read_leb_usize(slice, idx);
    let length_uncompressed = read_leb_usize(slice, idx);
    let length_compressed = read_leb_usize(slice, idx);

    #[coverage(off)]
    fn read_filenames(slice: &[u8], idx: &mut usize) -> Result<Vec<String>, ReadCovMapError> {
        let mut filenames = Vec::new();
        while *idx < slice.len() {
            let len = read_leb_usize(slice, idx);
            let filename = String::from_utf8(slice[*idx..*idx + len].to_vec()).map_err(
                #[coverage(off)]
                |_| ReadCovMapError::CannotParseUTF8 {
                    section: CovMapSection::CovMap,
                },
            )?;
            filenames.push(filename);
            *idx += len;
        }
        Ok(filenames)
    }

    let filenames = if length_compressed == 0 {
        read_filenames(slice, idx)?
    } else {
        let mut decompressed = vec![0; length_uncompressed];
        let mut decompress = flate2::Decompress::new(true);
        let decompress_result = decompress.decompress(
            &slice[*idx..*idx + length_compressed],
            &mut decompressed,
            flate2::FlushDecompress::Finish,
        );
        if !matches!(decompress_result, Ok(flate2::Status::StreamEnd)) {
            return Err(ReadCovMapError::FailedToDecompress {
                section: CovMapSection::CovMap,
                decompress_result,
            });
        }

        *idx += length_compressed;
        let mut decompressed_idx = 0;
        read_filenames(&decompressed, &mut decompressed_idx)?
    };

    if filenames.len() != nbr_filenames {
        return Err(ReadCovMapError::NumberOfFilenamesDoesNotMatch {
            actual: filenames.len(),
            expected: nbr_filenames,
        });
    }

    Ok(filenames)
}

// an expression read in function records (__llvm_covfun section)
#[derive(Debug)]
pub struct RawExpression {
    pub lhs: RawCounter,
    pub rhs: RawCounter,
}

// a counter read in function records (__llvm_covfun section)
#[derive(Debug)]
pub enum RawCounter {
    Zero,
    Counter { idx: usize },
    Expression { operation_sign: Sign, idx: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Sign {
    Negative,
    Positive,
}

#[derive(Debug)]
pub struct RawFunctionCounters {
    pub header: FunctionRecordHeader,
    pub file_id_mapping: FileIDMapping,
    pub expression_list: Vec<RawExpression>,
    pub counters_list: Vec<(RawCounter, MappingRegion)>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ExpandedExpression {
    pub add_terms: Vec<usize>, // Counter indices
    pub sub_terms: Vec<usize>, // Counter indices
}

impl Sign {
    #[coverage(off)]
    fn negated(self) -> Self {
        match self {
            Sign::Negative => Sign::Positive,
            Sign::Positive => Sign::Negative,
        }
    }
    #[coverage(off)]
    fn and(self, other: Self) -> Self {
        match other {
            Sign::Negative => self.negated(),
            Sign::Positive => self,
        }
    }
}

impl ExpandedExpression {
    #[coverage(off)]
    fn push_leaf_counter(&mut self, term: usize, expr_operation_sign: Sign) {
        let (recipient, counterpart) = match expr_operation_sign {
            Sign::Negative => (&mut self.sub_terms, &mut self.add_terms),
            Sign::Positive => (&mut self.add_terms, &mut self.sub_terms),
        };
        if let Some(index_in_counterpart) = counterpart.iter().position(
            #[coverage(off)]
            |&x| x == term,
        ) {
            counterpart.remove(index_in_counterpart);
        } else {
            recipient.push(term);
        }
    }
    #[coverage(off)]
    pub fn push_counter(
        &mut self,
        c: &RawCounter,
        sign: Sign,
        ctx: &RawFunctionCounters,
        expressions: &mut HashSet<ExpandedExpression>,
    ) {
        match c {
            RawCounter::Zero => {}
            RawCounter::Counter { idx } => {
                self.push_leaf_counter(*idx, sign);
                let mut e = ExpandedExpression::default();
                e.add_terms.push(*idx);
                expressions.insert(e);
            }
            RawCounter::Expression { operation_sign, idx } => {
                let e = &ctx.expression_list[*idx];
                let lhs = &e.lhs;
                self.push_counter(lhs, sign, ctx, expressions);
                let rhs = &e.rhs;
                self.push_counter(rhs, sign.and(*operation_sign), ctx, expressions);
            }
        }
    }
    #[coverage(off)]
    pub fn sort(&mut self) {
        self.add_terms.sort_unstable();
        self.sub_terms.sort_unstable();
    }
}

pub struct OptimisedExpandedExpression {
    add_terms: Vec<*const u64>,
    sub_terms: Vec<*const u64>,
}
impl OptimisedExpandedExpression {
    #[coverage(off)]
    pub fn compute(&self) -> u64 {
        unsafe {
            let mut result = 0;
            for &add_term in self.add_terms.iter() {
                result += *add_term;
            }
            for &sub_term in self.sub_terms.iter() {
                result -= *sub_term;
            }
            result
        }
    }
}

impl ExpandedExpression {
    #[coverage(off)]
    fn optimised(&self, counters: &[u64]) -> OptimisedExpandedExpression {
        let mut add_terms = Vec::new();
        let mut sub_terms = Vec::new();
        for &add_term in &self.add_terms {
            add_terms.push(&counters[add_term] as *const _);
        }
        for &sub_term in &self.sub_terms {
            sub_terms.push(&counters[sub_term] as *const _);
        }
        add_terms.sort_unstable();
        sub_terms.sort_unstable();
        OptimisedExpandedExpression { add_terms, sub_terms }
    }
}

pub struct Coverage {
    pub function_record: FunctionRecord,
    pub start_counters: *mut u64,
    pub counters_len: usize,
    pub single_counters: Vec<*mut u64>,
    pub expression_counters: Vec<OptimisedExpandedExpression>,
}

impl Coverage {
    #[coverage(off)]
    pub fn new(
        function_records: Vec<FunctionRecord>,
        prf_datas: Vec<PrfData>,
        all_counters: &'static mut [u64],
    ) -> Result<Vec<Coverage>, ReadCovMapError> {
        let mut start_idx = 0;
        prf_datas
            .iter()
            .filter_map(
                #[coverage(off)]
                |prf_data| {
                    let prf_data: &PrfData = prf_data;
                    if prf_data.function_id.structural_hash == 0 {
                        return None;
                    }
                    let range = start_idx..start_idx + prf_data.number_of_counters;
                    start_idx = range.end;
                    let f_r = function_records.iter().find(
                        #[coverage(off)]
                        |fr| fr.header.id == prf_data.function_id,
                    )?;

                    let slice = &mut all_counters[range];
                    let mut single_counters = Vec::new();
                    let mut expression_counters = Vec::new();

                    for (e, _) in f_r.expressions.iter() {
                        if e.add_terms.is_empty() && e.sub_terms.is_empty() {
                            continue;
                        } else if e.add_terms.len() == 1 && e.sub_terms.is_empty() {
                            single_counters.push(&mut slice[e.add_terms[0]] as *mut _);
                        } else if !e.add_terms.is_empty() {
                            expression_counters.push(e.optimised(slice));
                        } else {
                            panic!(
                                "An expression contains only sub terms\nAdd terms: {:?}\nSub terms: {:?}",
                                e.add_terms, e.sub_terms
                            );
                        }
                    }
                    Some(Ok(Coverage {
                        function_record: f_r.clone(),
                        start_counters: slice.as_mut_ptr(),
                        counters_len: slice.len(),
                        single_counters,
                        expression_counters,
                    }))
                },
            )
            .collect()
    }
}
