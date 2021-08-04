use super::leb128;
use std::collections::HashMap;
use std::{collections::HashSet, convert::TryFrom};

type CovMap = HashMap<[u8; 8], Vec<String>>;
use object::Object;
use object::ObjectSection;
use std::path::Path;

pub struct LLVMCovSections {
    pub covfun: Vec<u8>,
    pub covmap: Vec<u8>,
}

#[no_coverage]
pub fn get_llvm_cov_sections(path: &Path) -> LLVMCovSections {
    let bin_data = std::fs::read(path).unwrap();
    let obj_file = object::File::parse(&*bin_data).unwrap();
    let covmap = obj_file
        .section_by_name("__llvm_covmap")
        .unwrap()
        .data()
        .unwrap()
        .to_vec();
    let covfun = obj_file
        .section_by_name("__llvm_covfun")
        .unwrap()
        .data()
        .unwrap()
        .to_vec();
    LLVMCovSections { covfun, covmap }
}

#[no_coverage]
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

#[no_coverage]
fn read_leb_usize(slice: &[u8], idx: &mut usize) -> usize {
    assert!(!slice.is_empty());
    let (result, pos) = leb128::read_u64_leb128(&slice[*idx..]);
    *idx += pos;
    result as usize
}

#[no_coverage]
fn read_i64(slice: &[u8], idx: &mut usize) -> i64 {
    assert!(slice.len() >= 8);
    let subslice = <[u8; 8]>::try_from(&slice[*idx..*idx + 8]).unwrap();
    let x = i64::from_le_bytes(subslice);
    *idx += 8;
    x
}
#[no_coverage]
fn read_i32(slice: &[u8], idx: &mut usize) -> i32 {
    assert!(slice.len() >= 4);
    let subslice = <[u8; 4]>::try_from(&slice[*idx..*idx + 4]).unwrap();
    let x = i32::from_le_bytes(subslice);
    *idx += 4;
    x
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionIdentifier {
    pub name_md5: i64,
    pub structural_hash: i64,
}

#[derive(Clone, Debug)]
pub struct FunctionRecordHeader {
    pub id: FunctionIdentifier,
    pub hash_translation_unit: [u8; 8],
    length_encoded_data: usize,
}

#[no_coverage]
fn read_first_function_record_fields(covfun: &[u8], idx: &mut usize) -> FunctionRecordHeader {
    let name_md5 = read_i64(covfun, idx);
    let length_encoded_data = read_i32(covfun, idx) as usize;
    let structural_hash = read_i64(covfun, idx);
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

#[no_coverage]
fn read_file_id_mapping(covfun: &[u8], idx: &mut usize) -> FileIDMapping {
    assert!(!covfun.is_empty());
    let num_indices = read_leb_usize(covfun, idx);
    let mut filename_indices = Vec::new();
    for _ in 0..num_indices {
        filename_indices.push(read_leb_usize(covfun, idx));
    }
    FileIDMapping { filename_indices }
}

#[no_coverage]
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

// pub struct MappingRegion {
//     pub counter: RawCounter,
//     pub filename: String,
// }

#[no_coverage]
fn read_mapping_regions(covfun: &[u8], idx: &mut usize, files: usize) -> Vec<RawCounter> {
    assert!(!covfun.is_empty());

    let mut result = Vec::new();

    for _ in 0..files {
        let num_regions = read_leb_usize(covfun, idx);
        for _ in 0..num_regions {
            let raw_header = read_leb_usize(covfun, idx);
            let header = read_counter(raw_header); //read_leb_usize(covfun, idx)); // counter or pseudo-counter
            match header {
                RawCounter::Zero => assert_eq!(raw_header, 0),
                _ => {}
            }
            result.push(header);
            let _delta_line_start = read_leb_usize(covfun, idx);
            let _column_start = read_leb_usize(covfun, idx);
            let _num_lines = read_leb_usize(covfun, idx);
            let _column_end = read_leb_usize(covfun, idx);
        }
    }

    result
}

#[no_coverage]
pub fn read_covfun(covfun: &[u8], idx: &mut usize) -> Vec<RawFunctionCounters> {
    let mut results = Vec::new();
    while *idx < covfun.len() {
        let function_record_header = read_first_function_record_fields(covfun, idx);
        let idx_before_encoding_data = *idx;
        let file_id_mapping = read_file_id_mapping(covfun, idx);
        let expressions = read_coverage_expressions(covfun, idx);
        let counters = read_mapping_regions(covfun, idx, file_id_mapping.filename_indices.len());

        assert!(idx_before_encoding_data + function_record_header.length_encoded_data == *idx);

        results.push(RawFunctionCounters {
            header: function_record_header,
            file_id_mapping,
            expression_list: expressions,
            counters_list: counters,
        });

        let padding = if *idx < covfun.len() && *idx % 8 != 0 {
            8 - *idx % 8
        } else {
            0
        };
        *idx += padding;
    }

    results
}

pub struct PrfData {
    function_id: FunctionIdentifier,
    number_of_counters: usize,
}

#[no_coverage]
pub fn read_prf_data(prf_data: &[u8], idx: &mut usize) -> Vec<PrfData> {
    // I haven't found a reference for prf_data, so we'll guess ...
    let mut counts = Vec::new();
    while *idx < prf_data.len() {
        let name_md5 = read_i64(prf_data, idx);
        let structural_hash = read_i64(prf_data, idx);
        let function_id = FunctionIdentifier {
            name_md5,
            structural_hash,
        };

        for _ in 0..6 {
            let _something_i_dont_know_what = read_i32(prf_data, idx);
        }
        let nbr_counters = read_i32(prf_data, idx);
        counts.push(PrfData {
            function_id,
            number_of_counters: nbr_counters as usize,
        });
        let _something_i_dont_know_what = read_i32(prf_data, idx);
    }

    counts
}

#[derive(Clone, Debug)]
pub struct FunctionRecord {
    header: FunctionRecordHeader,
    file_id_mapping: FileIDMapping,
    expressions: Vec<ExpandedExpression>,
}

#[no_coverage]
pub fn process_function_records(records: Vec<RawFunctionCounters>) -> Vec<FunctionRecord> {
    let mut all_expressions = Vec::new();
    for function_counters in records {
        let mut expressions = HashSet::<ExpandedExpression>::new();
        for raw_counter in function_counters.counters_list.iter() {
            let mut expanded = ExpandedExpression::default();
            expanded.push_counter(&raw_counter, Sign::Positive, &function_counters);
            expanded.sort();
            expressions.insert(expanded);
        }
        let expressions = expressions.into_iter().collect::<Vec<_>>();
        all_expressions.push(FunctionRecord {
            header: function_counters.header,
            file_id_mapping: function_counters.file_id_mapping,
            expressions,
        });
    }
    all_expressions
}

#[no_coverage]
pub fn read_covmap(covmap: &[u8], idx: &mut usize) -> CovMap {
    let mut translation_unit_map = HashMap::new();
    while *idx < covmap.len() {
        let _always_0 = read_i32(covmap, idx);
        let length_encoded_data = read_i32(covmap, idx) as usize;
        let _always_0 = read_i32(covmap, idx);
        let version = read_i32(covmap, idx);
        assert_eq!(version, 3); // version 4 actually, but encoded as 3
                                //let _something_undocumented = read_i32(covmap, idx);

        let encoded_data = &covmap[*idx..*idx + length_encoded_data];
        let filenames = unsafe { read_list_filenames(encoded_data, &mut 0) };
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
    translation_unit_map
}

#[no_coverage]
pub unsafe fn read_list_filenames(slice: &[u8], idx: &mut usize) -> Vec<String> {
    let _nbr_filenames = read_leb_usize(slice, idx);
    let _length_uncompressed = read_leb_usize(slice, idx);
    let length_compressed = read_leb_usize(slice, idx);
    assert_eq!(length_compressed, 0);

    let mut filenames = Vec::new();
    while *idx < slice.len() {
        let len = read_leb_usize(slice, idx);
        filenames.push(String::from_utf8(slice[*idx..*idx + len].to_vec()).unwrap());
        *idx += len;
    }
    filenames
}

extern "C" {
    #[no_coverage]
    pub(crate) fn get_start_instrumentation_counters() -> *mut u64;
    #[no_coverage]
    pub(crate) fn get_end_instrumentation_counters() -> *mut u64;
    #[no_coverage]
    pub(crate) fn get_start_prf_data() -> *const u8;
    #[no_coverage]
    pub(crate) fn get_end_prf_data() -> *const u8;

}

#[no_coverage]
pub unsafe fn get_counters() -> &'static mut [u64] {
    let start = get_start_instrumentation_counters();
    let end = get_end_instrumentation_counters();
    let len = end.offset_from(start) as usize;
    std::slice::from_raw_parts_mut(start, len)
}
#[no_coverage]
pub unsafe fn get_prf_data() -> &'static [u8] {
    let start = get_start_prf_data();
    let end = get_end_prf_data();
    let len = end.offset_from(start) as usize;
    std::slice::from_raw_parts(start, len)
}
use std::ops::Range;

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

#[derive(Debug, Clone, Copy)]
pub enum Sign {
    Negative,
    Positive,
}

#[derive(Debug)]
pub struct RawFunctionCounters {
    pub header: FunctionRecordHeader,
    pub file_id_mapping: FileIDMapping,
    pub expression_list: Vec<RawExpression>,
    pub counters_list: Vec<RawCounter>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ExpandedExpression {
    pub add_terms: Vec<usize>, // Counter indices
    pub sub_terms: Vec<usize>, // Counter indices
}

impl Sign {
    #[no_coverage]
    fn negated(self) -> Self {
        match self {
            Sign::Negative => Sign::Positive,
            Sign::Positive => Sign::Negative,
        }
    }
    #[no_coverage]
    fn and(self, other: Self) -> Self {
        match other {
            Sign::Negative => self.negated(),
            Sign::Positive => self,
        }
    }
}

impl ExpandedExpression {
    #[no_coverage]
    fn push_leaf_counter(&mut self, term: usize, expr_operation_sign: Sign) {
        let (recipient, counterpart) = match expr_operation_sign {
            Sign::Negative => (&mut self.sub_terms, &mut self.add_terms),
            Sign::Positive => (&mut self.add_terms, &mut self.sub_terms),
        };
        if let Some(index_in_counterpart) = counterpart.iter().position(|&x| x == term) {
            counterpart.remove(index_in_counterpart);
        } else {
            recipient.push(term);
        }
    }
    #[no_coverage]
    pub fn push_counter(&mut self, c: &RawCounter, sign: Sign, ctx: &RawFunctionCounters) {
        match c {
            RawCounter::Zero => {}
            RawCounter::Counter { idx } => self.push_leaf_counter(*idx, sign),
            RawCounter::Expression { operation_sign, idx } => {
                let e = &ctx.expression_list[*idx];
                let lhs = &e.lhs;
                self.push_counter(lhs, sign, ctx);
                let rhs = &e.rhs;
                self.push_counter(rhs, sign.and(*operation_sign), ctx);
            }
        }
    }
    #[no_coverage]
    pub fn sort(&mut self) {
        self.add_terms.sort();
        self.sub_terms.sort();
    }

    #[no_coverage]
    pub unsafe fn count(&self, counters: &[u64]) -> u64 {
        self.add_terms.iter().map(|t| counters[*t]).sum::<u64>()
            - self.sub_terms.iter().map(|t| counters[*t]).sum::<u64>()
    }
}

#[derive(Debug)]
pub struct FunctionCoverage {
    pub function_record: FunctionRecord,
    pub counters_range: Range<usize>,
    pub len: usize,
}
impl FunctionCoverage {
    #[no_coverage]
    pub fn filter_trivial_expressions(&mut self) {
        self.function_record
            .expressions
            .drain_filter(|e| e.add_terms.len() <= 1 && e.sub_terms.is_empty());
    }
}
#[derive(Debug)]
pub struct AllCoverage {
    pub covmap: CovMap,
    pub counters: Vec<FunctionCoverage>,
}
impl AllCoverage {
    #[no_coverage]
    pub fn new(covmap: CovMap, function_records: Vec<FunctionRecord>, prf_datas: Vec<PrfData>) -> Self {
        let mut counters = Vec::new();
        let mut cur_idx = 0;
        for prf_data in prf_datas {
            let range = cur_idx..cur_idx + prf_data.number_of_counters;
            // now retrieve the list of expressions and adjust their counter indices
            let mut function_record = function_records
                .iter()
                .find(|&f_r| f_r.header.id == prf_data.function_id)
                .map(|f_r| f_r)
                .unwrap()
                .clone();

            let coverage_len = range.len() + function_record.expressions.len();
            for e in function_record.expressions.iter_mut() {
                for term in e.add_terms.iter_mut() {
                    *term += cur_idx;
                }
                for term in e.sub_terms.iter_mut() {
                    *term += cur_idx;
                }
            }
            let mut coverage = FunctionCoverage {
                function_record,
                counters_range: range,
                len: coverage_len,
            };
            coverage.filter_trivial_expressions();
            coverage.len = coverage.counters_range.len() + coverage.function_record.expressions.len();

            counters.push(coverage);
            cur_idx += prf_data.number_of_counters;
        }
        Self { covmap, counters }
    }

    #[no_coverage]
    pub fn iterate_over_coverage_points<F>(&mut self, counters: &[u64], mut f: F)
    where
        F: FnMut((usize, u64)),
    {
        unsafe {
            let mut index = 0;
            for coverage in self.counters.iter() {
                if counters[coverage.counters_range.start] == 0 {
                    index += coverage.len;
                    continue;
                }
                for c in counters[coverage.counters_range.clone()].iter() {
                    if *c != 0 {
                        f((index, *c));
                    }
                    index += 1;
                }
                for e in coverage.function_record.expressions.iter() {
                    let count = e.count(counters);
                    if count != 0 {
                        f((index, count));
                    }
                    index += 1;
                }
            }
        }
    }
}

impl AllCoverage {
    pub(crate) fn filter_function_by_files<F, G>(&mut self, exclude_f: F, keep_f: G)
    where
        F: Fn(&str) -> bool,
        G: Fn(&str) -> bool,
    {
        let AllCoverage { covmap, counters } = self;
        counters.drain_filter(|coverage| {
            let mut excluded = false;
            let filenames = &covmap[&coverage.function_record.header.hash_translation_unit];
            for idx in coverage.function_record.file_id_mapping.filename_indices.iter() {
                let filename = &filenames[*idx];
                if keep_f(filename) {
                    return false;
                }
                if exclude_f(filename) {
                    // if not keep, then bye bye
                    excluded = true;
                }
            }
            // no filenames were kept or excluded
            return excluded;
        });
    }
}