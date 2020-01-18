//! Code coverage analysis

mod hooks;

use crate::Feature;

use ahash::AHashSet;
use std::convert::TryFrom;
use std::mem::MaybeUninit;

type PC = usize;

static mut SHARED_SENSOR: MaybeUninit<CodeCoverageSensor> = MaybeUninit::<CodeCoverageSensor>::uninit();

/// Returns a reference to the only `CodeCoverageSensor`
pub fn shared_sensor() -> &'static mut CodeCoverageSensor {
    unsafe { &mut *SHARED_SENSOR.as_mut_ptr() }
}

/// Records the code coverage of the program and converts it into `Feature`s
/// that the `pool` can understand.
pub struct CodeCoverageSensor {
    pub is_recording: bool,
    eight_bit_counters: &'static mut [u8],
    features: AHashSet<Feature>, //  could it be a BTreeSet?
}

impl CodeCoverageSensor {
    /// Handles a `trace_cmp` hook from SanitizerCoverage, by recording it
    /// as a `Feature` of kind `instruction`.
    fn handle_trace_cmp(&mut self, pc: PC, arg1: u64, arg2: u64) {
        let f = Feature::instruction(pc, arg1, arg2);
        self.features.insert(f);
    }
    /// Handles a `trace_indir` hook from SanitizerCoverage, by recording it
    /// as a `Feature` of kind `indirect`.
    fn handle_trace_indir(&mut self, caller: PC, callee: PC) {
        let f = Feature::indir(caller ^ callee);
        self.features.insert(f);
    }

    /// Runs the closure on all recorded features.
    pub(crate) fn iterate_over_collected_features<F>(&mut self, mut handle: F)
    where
        F: FnMut(Feature) -> (),
    {
        const CHUNK_SIZE: usize = 32;
        let length_chunks = self.eight_bit_counters.len() / CHUNK_SIZE;
        let zero: [u8; CHUNK_SIZE] = [0; CHUNK_SIZE];

        for i in 0..length_chunks {
            let start = i * CHUNK_SIZE;
            let end = start + CHUNK_SIZE;

            let slice = <&[u8; CHUNK_SIZE]>::try_from(&self.eight_bit_counters[start..end]).unwrap();
            if slice == &zero {
                continue;
            } else {
                for (j, x) in slice.iter().enumerate() {
                    if *x == 0 {
                        continue;
                    } else {
                        let f = Feature::edge(start + j, *x as u16);
                        handle(f);
                    }
                }
            }
        }

        let start_remainder = length_chunks * CHUNK_SIZE;
        let remainder = &self.eight_bit_counters[start_remainder..];
        for (j, x) in remainder.iter().enumerate() {
            let i = start_remainder + j;
            if *x == 0 {
                continue;
            } else {
                let f = Feature::edge(i, *x as u16);
                handle(f);
            }
        }

        // TODO: could covert features into a Vec and then sort that, will do it
        // for now, but in the future I may need a proper alternative
        let mut op_features: Vec<_> = self.features.iter().copied().collect();
        op_features.sort();

        for f in op_features.iter() {
            handle(*f);
        }
    }

    pub fn clear(&mut self) {
        for x in self.eight_bit_counters.iter_mut() {
            *x = 0;
        }
        self.features.clear();
    }
}
