//! Code coverage analysis

mod hooks;

use crate::Feature;

#[cfg(trace_compares)]
use crate::InstrFeatureWithoutTag;

#[cfg(trace_compares)]
use crate::data_structures::HBitSet;

use std::convert::TryFrom;
use std::mem::MaybeUninit;

#[cfg(trace_compares)]
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
    /// pointer to the __sancov_lowest_stack variable
    _lowest_stack: &'static mut libc::uintptr_t,
    /// value of __sancov_lowest_stack after running an input
    pub lowest_stack: usize,
    #[cfg(trace_compares)]
    instr_features: HBitSet,
}

impl CodeCoverageSensor {
    pub fn start_recording(&mut self) {
        self.is_recording = true;
        self.lowest_stack = usize::MAX;
        *self._lowest_stack = usize::MAX;
    }
    pub fn stop_recording(&mut self) {
        self.is_recording = false;
        self.lowest_stack = *self._lowest_stack;
    }
}

#[cfg(trace_compares)]
macro_rules! make_instr_feature_without_tag {
    ($pc:ident, $arg1:ident, $arg2:ident) => {{
        (($pc & 0x2F_FFFF) << Feature::id_offset()) | (($arg1 ^ $arg2).count_ones() as usize)
    }};
}

// TODO: indir disabled for now
// impl CodeCoverageSensor {
//     #[inline]
//     fn handle_trace_indir(&mut self, caller: PC, callee: PC) {
//         // let f = Feature::indir(caller, callee);
//         // self.indir_features.insert(f);
//     }
// }

#[cfg(trace_compares)]
impl CodeCoverageSensor {
    /// Handles a `trace_cmp` hook from Sanitizer Coverage, by recording it
    /// as a `Feature` of kind `instruction`.
    #[inline]
    fn handle_trace_cmp_u8(&mut self, pc: PC, arg1: u8, arg2: u8) {
        let f = make_instr_feature_without_tag!(pc, arg1, arg2);
        self.instr_features.set(f);
    }
    #[inline]
    fn handle_trace_cmp_u16(&mut self, pc: PC, arg1: u16, arg2: u16) {
        let f = make_instr_feature_without_tag!(pc, arg1, arg2);
        self.instr_features.set(f);
    }
    #[inline]
    fn handle_trace_cmp_u32(&mut self, pc: PC, arg1: u32, arg2: u32) {
        let f = make_instr_feature_without_tag!(pc, arg1, arg2);
        self.instr_features.set(f);
    }
    #[inline]
    fn handle_trace_cmp_u64(&mut self, pc: PC, arg1: u64, arg2: u64) {
        let f = make_instr_feature_without_tag!(pc, arg1, arg2);
        self.instr_features.set(f);
    }
}

impl CodeCoverageSensor {
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
            }
            for (j, x) in slice.iter().enumerate() {
                if *x == 0 {
                    continue;
                }
                let f = Feature::edge(start + j, u16::from(*x));
                handle(f);
            }
        }

        let start_remainder = length_chunks * CHUNK_SIZE;
        let remainder = &self.eight_bit_counters[start_remainder..];
        for (j, x) in remainder.iter().enumerate() {
            let i = start_remainder + j;
            if *x == 0 {
                continue;
            }
            let f = Feature::edge(i, u16::from(*x));
            handle(f);
        }

        // self.indir_features. ...

        #[cfg(trace_compares)]
        {
            self.instr_features.drain(|f| {
                handle(Feature::from_instr(InstrFeatureWithoutTag(f)));
            });
        }
    }

    pub fn clear(&mut self) {
        for x in self.eight_bit_counters.iter_mut() {
            *x = 0;
        }
        #[cfg(trace_compares)]
        {
            self.instr_features.drain(|_| {});
        }
        // self.indir_features.clean();
    }
}
