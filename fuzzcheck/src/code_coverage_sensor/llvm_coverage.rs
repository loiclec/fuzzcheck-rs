use super::leb128;
use object::Object;
use object::ObjectSection;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::path::{Path, PathBuf};

type CovMap = HashMap<[u8; 8], Vec<String>>;

pub struct LLVMCovSections {
    pub covfun: Vec<u8>,
    pub covmap: Vec<u8>,
    pub prf_names: Vec<u8>,
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
    let prf_names = obj_file
        .section_by_name("__llvm_prf_names")
        .unwrap()
        .data()
        .unwrap()
        .to_vec();
    LLVMCovSections {
        covfun,
        covmap,
        prf_names,
    }
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
fn read_u64(slice: &[u8], idx: &mut usize) -> u64 {
    assert!(slice.len() >= 8);
    let subslice = <[u8; 8]>::try_from(&slice[*idx..*idx + 8]).unwrap();
    let x = u64::from_le_bytes(subslice);
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
#[no_coverage]
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

#[no_coverage]
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

#[derive(Clone, Debug)]
pub struct MappingRegion {
    pub filename_index: usize,
    pub line_start: usize,
    pub line_end: usize,
    pub col_start: usize,
    pub col_end: usize,
}

#[no_coverage]
fn read_mapping_regions(
    covfun: &[u8],
    idx: &mut usize,
    filename_indices: &[usize],
) -> Vec<(RawCounter, MappingRegion)> {
    assert!(!covfun.is_empty());

    let mut result = Vec::new();

    for &filename_index in filename_indices {
        let num_regions = read_leb_usize(covfun, idx);
        let mut prev_line_end = 0;
        for _ in 0..num_regions {
            let raw_header = read_leb_usize(covfun, idx);
            let header = read_counter(raw_header); //read_leb_usize(covfun, idx)); // counter or pseudo-counter
            match header {
                RawCounter::Zero => assert_eq!(raw_header, 0),
                _ => {}
            }
            let delta_line_start = read_leb_usize(covfun, idx);
            let col_start = read_leb_usize(covfun, idx);
            let num_lines = read_leb_usize(covfun, idx);
            let col_end = read_leb_usize(covfun, idx);

            let line_start = prev_line_end + delta_line_start;
            let line_end = line_start + num_lines;
            prev_line_end = line_end;
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
        let counters = read_mapping_regions(covfun, idx, &file_id_mapping.filename_indices);

        assert!(idx_before_encoding_data + function_record_header.length_encoded_data == *idx);

        let padding = if *idx < covfun.len() && *idx % 8 != 0 {
            8 - *idx % 8
        } else {
            0
        };
        *idx += padding;

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

    results
}

pub struct PrfData {
    pub function_id: FunctionIdentifier,
    number_of_counters: usize,
}

#[no_coverage]
pub fn read_prf_data(prf_data: &[u8], idx: &mut usize) -> Vec<PrfData> {
    let mut counts = Vec::new();
    let mut prv_counter_pointer_and_nbr_counters = None;

    while *idx < prf_data.len() {
        // NameRef, see InstrProdData.inc line 72
        // IndexedInstrProf::ComputeHash(getPGOFuncNameVarInitializer(Inc->getName())
        let name_md5 = read_i64(prf_data, idx);
        // FuncHash, see InstrProdData.inc line 75
        // Inc->getHash()->getZExtValue()
        let structural_hash = read_u64(prf_data, idx);
        assert!(
            structural_hash != 0,
            "found profile data for a dummy function (i.e. with a structural hash == 0)"
        );
        let function_id = FunctionIdentifier {
            name_md5,
            structural_hash,
        };

        let counter_ptr = read_u64(prf_data, idx);
        let _function_ptr = read_u64(prf_data, idx);
        let _values = read_u64(prf_data, idx);

        // u32 counters
        let nbr_counters = read_u32(prf_data, idx);

        if let Some((prv_counter_pointer, nbr_counters)) = prv_counter_pointer_and_nbr_counters {
            assert_eq!(prv_counter_pointer + 8 * nbr_counters as u64, counter_ptr);
        }
        prv_counter_pointer_and_nbr_counters = Some((counter_ptr, nbr_counters));

        counts.push(PrfData {
            function_id,
            number_of_counters: nbr_counters as usize,
        });
        // u16 but aligned
        let something_i_dont_know_what = read_i32(prf_data, idx);
        assert_eq!(
            something_i_dont_know_what, 0,
            "Found last field of __llvm_prf_data different than 0.
            This may not be a problem, but the program is aborted by caution.
            Could you please file an issue to https://github.com/loiclec/fuzzcheck-rs ?"
        );
    }

    counts
}

#[no_coverage]
fn read_func_names(slice: &[u8], names: &mut Vec<String>) {
    let slices = slice.split(|&x| x == 0x01);
    for slice in slices {
        let string = String::from_utf8(slice.to_vec()).expect("could not parse function name in __llvm_prf_names");
        names.push(string);
    }
}

#[no_coverage]
pub fn read_prf_names(slice: &[u8], idx: &mut usize) -> Vec<String> {
    let mut names = Vec::new();
    while *idx < slice.len() {
        let length_uncompressed = read_leb_usize(slice, idx);
        let length_compressed = read_leb_usize(slice, idx);
        if length_compressed == 0 {
            read_func_names(&slice[*idx..*idx + length_uncompressed], &mut names);
            *idx += length_uncompressed;
        } else {
            let mut decompressed = Vec::with_capacity(length_uncompressed);
            let mut decompress = flate2::Decompress::new(false);
            decompress
                .decompress(
                    &slice[*idx..*idx + length_compressed],
                    &mut decompressed,
                    flate2::FlushDecompress::Finish,
                )
                .expect("failed to decompress content of prf_names");
            *idx += length_compressed;
            read_func_names(&decompressed, &mut names);
        }
    }
    names
}

#[derive(Clone, Debug)]
pub struct FunctionRecord {
    pub header: FunctionRecordHeader,
    pub file_id_mapping: FileIDMapping,
    pub expressions: Vec<(ExpandedExpression, MappingRegion)>,
    pub name_function: String,
    pub filenames: Vec<PathBuf>,
}

#[no_coverage]
pub fn process_function_records(
    records: Vec<RawFunctionCounters>,
    prf_names: HashMap<i64, String>,
    covmap: &CovMap,
) -> Vec<FunctionRecord> {
    let mut all_expressions = Vec::new();
    for function_counters in records {
        let mut expressions = HashMap::<ExpandedExpression, MappingRegion>::new();
        for (raw_counter, mapping_region) in function_counters.counters_list.iter() {
            let mut expanded = ExpandedExpression::default();
            expanded.push_counter(raw_counter, Sign::Positive, &function_counters);
            expanded.sort(); // sort them so that their hash is the same if they are equal
            expressions.insert(expanded, mapping_region.clone());
        }
        let name_function = (&prf_names[&function_counters.header.id.name_md5]).clone();
        let expressions = expressions.into_iter().collect::<Vec<_>>();

        let filenames = &covmap[&function_counters.header.hash_translation_unit];
        let mut filepaths = Vec::new();
        for idx in function_counters.file_id_mapping.filename_indices.iter() {
            let filename = &filenames[*idx];
            let filepath = Path::new(filename).to_path_buf();
            filepaths.push(filepath);
        }

        all_expressions.push(FunctionRecord {
            header: function_counters.header,
            file_id_mapping: function_counters.file_id_mapping,
            expressions,
            name_function,
            filenames: filepaths,
        });
    }
    all_expressions
}

#[derive(Debug)]
pub enum ReadCovMapError {
    CompressedLengthTooLong,
    InvalidVersion(i32),
}

#[no_coverage]
/// Reads the contents of the LLVM coverage map, returning an error if this is
/// not possible.
pub fn read_covmap(covmap: &[u8], idx: &mut usize) -> Result<CovMap, ReadCovMapError> {
    let mut translation_unit_map = HashMap::new();
    while *idx < covmap.len() {
        let _always_0 = read_i32(covmap, idx);
        let length_encoded_data = read_i32(covmap, idx) as usize;
        let _always_0 = read_i32(covmap, idx);
        let version = read_i32(covmap, idx);
        if version != 3 {
            return Err(ReadCovMapError::InvalidVersion(version));
        }
        assert_eq!(version, 3); // version 4 actually, but encoded as 3

        let encoded_data = &covmap[*idx..*idx + length_encoded_data];
        let filenames = read_list_filenames(encoded_data, &mut 0);
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

#[no_coverage]
pub fn read_list_filenames(slice: &[u8], idx: &mut usize) -> Vec<String> {
    let nbr_filenames = read_leb_usize(slice, idx);
    let length_uncompressed = read_leb_usize(slice, idx);
    let length_compressed = read_leb_usize(slice, idx);

    fn read_filenames(slice: &[u8], idx: &mut usize) -> Vec<String> {
        let mut filenames = Vec::new();
        while *idx < slice.len() {
            let len = read_leb_usize(slice, idx);
            filenames.push(String::from_utf8(slice[*idx..*idx + len].to_vec()).unwrap());
            *idx += len;
        }
        filenames
    }

    let filenames = if length_compressed == 0 {
        read_filenames(slice, idx)
    } else {
        let mut decompressed = Vec::with_capacity(length_uncompressed);
        let mut decompress = flate2::Decompress::new(false);
        decompress
            .decompress(
                &slice[*idx..*idx + length_compressed],
                &mut decompressed,
                flate2::FlushDecompress::Finish,
            )
            .expect("failed to decompress list of filenames in __llvm_covmap");
        *idx += length_compressed;
        let mut decompressed_idx = 0;
        read_filenames(&decompressed, &mut decompressed_idx)
    };
    assert_eq!(
        filenames.len(),
        nbr_filenames,
        "parsed a different number of filenames than what was expected"
    );
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
    pub counters_list: Vec<(RawCounter, MappingRegion)>,
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
        self.add_terms.sort_unstable();
        self.sub_terms.sort_unstable();
    }

    #[no_coverage]
    pub unsafe fn count(&self, counters: &[u64]) -> u64 {
        self.add_terms.iter().map(|t| counters[*t]).sum::<u64>()
            - self.sub_terms.iter().map(|t| counters[*t]).sum::<u64>()
    }
}

impl FunctionRecord {
    #[no_coverage]
    pub fn filter_trivial_expressions(&mut self) -> Vec<(usize, MappingRegion)> {
        // filtered represent all the (unique) expressions that consist of only one positive term
        // that term is necessarily a reference to one of the “physical” counters in __llvm_prf_cnts
        let filtered = self
            .expressions
            .drain_filter(|(e, _)| e.add_terms.len() <= 1 && e.sub_terms.is_empty())
            .filter_map(|(e, mapping_region)| e.add_terms.get(0).map(|c| (*c, mapping_region)))
            .collect::<Vec<_>>();
        filtered
    }
}

#[derive(Debug)]
pub struct Coverage {
    pub physical_counters_start: *mut u64,
    pub physical_counters_len: usize,
    pub function_record: FunctionRecord,
    pub len: usize,
}

impl Coverage {
    pub fn new(
        function_records: Vec<FunctionRecord>,
        prf_datas: Vec<PrfData>,
        all_counters: &'static mut [u64],
    ) -> Vec<Self> {
        let mut start_idx = 0;
        prf_datas
            .iter()
            .map(|prf_data| {
                let range = start_idx..start_idx + prf_data.number_of_counters;
                start_idx = range.end;
                let f_r = function_records
                    .iter()
                    .find(|fr| fr.header.id == prf_data.function_id)
                    .expect("can't find function record associated with a prf_data");

                // keep the trivial expressions in
                // it's inefficient, but it doesn't really matter, I'll write an optimized version later

                let len = range.len() + f_r.expressions.len();
                let slice = &mut all_counters[range];
                Coverage {
                    physical_counters_start: slice.as_mut_ptr(),
                    physical_counters_len: slice.len(),
                    function_record: f_r.clone(),
                    len,
                }
            })
            .collect()
    }
    pub(crate) fn filter_function_by_files<F, G>(all_self: &mut Vec<Self>, exclude_f: F, keep_f: G)
    where
        F: Fn(&Path) -> bool,
        G: Fn(&Path) -> bool,
    {
        all_self.drain_filter(|coverage| {
            let mut excluded = false;
            for filepath in &coverage.function_record.filenames {
                if keep_f(filepath) {
                    return false;
                }
                if exclude_f(filepath) {
                    // if not keep, then bye bye
                    excluded = true;
                }
            }
            // no filenames were kept or excluded
            excluded
        });
    }
    pub fn iterate_over_coverage_points<F>(coverage: &[Self], mut f: F)
    where
        F: FnMut((usize, u64)),
    {
        unsafe {
            let mut index = 0;
            for coverage in coverage.iter() {
                let slice =
                    std::slice::from_raw_parts(coverage.physical_counters_start, coverage.physical_counters_len);
                if slice[0] == 0 {
                    index += coverage.len;
                    continue;
                }
                for (e, _) in coverage.function_record.expressions.iter() {
                    let count = e.count(slice);
                    if count != 0 {
                        f((index, count));
                    }
                    index += 1;
                }
            }
        }
    }
}
