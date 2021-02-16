//! Code coverage analysis

mod hooks;

use crate::Feature;
use std::convert::TryFrom;

#[cfg(trace_compares)]
use crate::InstrFeatureWithoutTag;

#[cfg(trace_compares)]
use crate::data_structures::HBitSet;

use std::mem::MaybeUninit;

#[cfg(trace_compares)]
type PC = usize;

static mut SHARED_SENSOR: MaybeUninit<CodeCoverageSensor> = MaybeUninit::<CodeCoverageSensor>::uninit();

/// Records the code coverage of the program and converts it into `Feature`s
/// that the `pool` can understand.
struct CodeCoverageSensor {
    eight_bit_counters: &'static mut [u8],
    /// pointer to the __sancov_lowest_stack variable
    _lowest_stack: &'static mut libc::uintptr_t,
    /// value of __sancov_lowest_stack after running an input
    lowest_stack: usize,
    #[cfg(trace_compares)]
    instr_features: HBitSet,
}

pub fn lowest_stack() -> usize {
    unsafe { (*SHARED_SENSOR.as_ptr()).lowest_stack }
}

pub fn start_recording() {
    unsafe {
        let sensor = SHARED_SENSOR.as_mut_ptr();
        (*sensor).lowest_stack = usize::MAX;
        *(*sensor)._lowest_stack = usize::MAX;
    }
}

pub fn stop_recording() {
    unsafe {
        let sensor = SHARED_SENSOR.as_mut_ptr();
        (*sensor).lowest_stack = *(*sensor)._lowest_stack;
    }
}

/// Runs the closure on all recorded features.
pub(crate) fn iterate_over_collected_features<F>(mut handle: F)
where
    F: FnMut(Feature) -> (),
{
    let sensor = unsafe { SHARED_SENSOR.as_mut_ptr() };
    const CHUNK_SIZE: usize = 16;
    let zero: [u8; CHUNK_SIZE] = [0; CHUNK_SIZE];
    let length_chunks = unsafe { (*sensor).eight_bit_counters.len() / CHUNK_SIZE };

    for i in 0..length_chunks {
        let start = i * CHUNK_SIZE;
        let end = start + CHUNK_SIZE;

        let slice =
            unsafe { <&[u8; CHUNK_SIZE]>::try_from((*sensor).eight_bit_counters.get_unchecked(start..end)).unwrap() };

        if slice == &zero {
            continue;
        } else {
            for (j, &x) in slice.iter().enumerate() {
                if x == 0 {
                    continue;
                }
                let f = Feature::edge(start + j, u16::from(x));
                handle(f);
            }
        }
    }

    let start_remainder = length_chunks * CHUNK_SIZE;
    let remainder = unsafe { (*sensor).eight_bit_counters.get_unchecked(start_remainder..) };
    for (j, &x) in remainder.iter().enumerate() {
        if x == 0 {
            continue;
        }
        let i = start_remainder + j;
        let f = Feature::edge(i, u16::from(x));
        handle(f);
    }

    // self.indir_features. ...

    #[cfg(trace_compares)]
    {
        unsafe {
            (*sensor).instr_features.drain(|f| {
                handle(Feature::from_instr(InstrFeatureWithoutTag(f)));
            });
        }
    }
}

#[cfg(trace_compares)]
macro_rules! make_instr_feature_without_tag {
    ($pc:ident, $arg1:ident, $arg2:ident) => {{
        (($pc & 0x3F_FFFF) << Feature::id_offset()) | (($arg1 ^ $arg2).count_ones() as usize)
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
#[inline]
fn handle_trace_cmp_u8(pc: PC, arg1: u8, arg2: u8) {
    let f = make_instr_feature_without_tag!(pc, arg1, arg2);
    let sensor = unsafe { SHARED_SENSOR.as_mut_ptr() };
    unsafe { (*sensor).instr_features.set(f) };
}
#[cfg(trace_compares)]
#[inline]
fn handle_trace_cmp_u16(pc: PC, arg1: u16, arg2: u16) {
    let f = make_instr_feature_without_tag!(pc, arg1, arg2);
    let sensor = unsafe { SHARED_SENSOR.as_mut_ptr() };
    unsafe { (*sensor).instr_features.set(f) };
}
#[cfg(trace_compares)]
#[inline]
fn handle_trace_cmp_u32(pc: PC, arg1: u32, arg2: u32) {
    let f = make_instr_feature_without_tag!(pc, arg1, arg2);
    let sensor = unsafe { SHARED_SENSOR.as_mut_ptr() };
    unsafe { (*sensor).instr_features.set(f) };
}
#[cfg(trace_compares)]
#[inline]
fn handle_trace_cmp_u64(pc: PC, arg1: u64, arg2: u64) {
    let f = make_instr_feature_without_tag!(pc, arg1, arg2);
    let sensor = unsafe { SHARED_SENSOR.as_mut_ptr() };
    unsafe { (*sensor).instr_features.set(f) };
}

pub fn clear() {
    unsafe {
        let sensor = SHARED_SENSOR.as_mut_ptr();

        for x in (*sensor).eight_bit_counters.iter_mut() {
            *x = 0;
        }
        #[cfg(trace_compares)]
        {
            (*sensor).instr_features.drain(|_| {});
        }
        // self.indir_features.clean();
    }
}
