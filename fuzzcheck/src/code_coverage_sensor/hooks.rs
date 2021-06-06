//! This module implements the hooks defined by Sanitizer Coverage to track the
//! execution of the program.
//!
//! For more information about Sanitizer Coverage,
//! see <https://clang.llvm.org/docs/SanitizerCoverage.html>
//!
//! I have tried to implement the callbacks in a simple way, and to delegate
//! the more complicated logic to the `code_coverage_sensor` module.
//! Most of the time, I try to create an identifier from the arguments
//! passed to the hook, and/or from the address of the hook call, which I name
//! PC (for Program Counter). The PC by itself uniquely identifies the part
//! of the program that was reached just before calling the hook.
//!
//! There are many hooks containing data passed before specific instructions.
//! For these hooks, I also just record the arguments and pass them to the code
//! coverage sensor. Here is the documentation for these hooks from
//! Sanitizer Coverage:
//!
//! ```text
//! // Called before a comparison instruction.
//! // Arg1 and Arg2 are arguments of the comparison.
//! void __sanitizer_cov_trace_cmp1(uint8_t Arg1, uint8_t Arg2);
//! void __sanitizer_cov_trace_cmp2(uint16_t Arg1, uint16_t Arg2);
//! void __sanitizer_cov_trace_cmp4(uint32_t Arg1, uint32_t Arg2);
//! void __sanitizer_cov_trace_cmp8(uint64_t Arg1, uint64_t Arg2);
//!
//! // Called before a comparison instruction if exactly one of the arguments is constant.
//! // Arg1 and Arg2 are arguments of the comparison, Arg1 is a compile-time constant.
//! // These callbacks are emitted by -fsanitize-coverage=trace-cmp since 2017-08-11
//! void __sanitizer_cov_trace_const_cmp1(uint8_t Arg1, uint8_t Arg2);
//! void __sanitizer_cov_trace_const_cmp2(uint16_t Arg1, uint16_t Arg2);
//! void __sanitizer_cov_trace_const_cmp4(uint32_t Arg1, uint32_t Arg2);
//! void __sanitizer_cov_trace_const_cmp8(uint64_t Arg1, uint64_t Arg2);
//!
//! // Called before a switch statement.
//! // Val is the switch operand.
//! // Cases[0] is the number of case constants.
//! // Cases[1] is the size of Val in bits.
//! // Cases[2:] are the case constants.
//! void __sanitizer_cov_trace_switch(uint64_t Val, uint64_t *Cases);
//!
//! // Called before a division statement.
//! // Val is the second argument of division.
//! void __sanitizer_cov_trace_div4(uint32_t Val);
//! void __sanitizer_cov_trace_div8(uint64_t Val);
//!
//! // Called before a GetElemementPtr (GEP) instruction
//! // for every non-constant array index.
//! void __sanitizer_cov_trace_gep(uintptr_t Idx);
//! ```

use super::{CodeCoverageSensor, SHARED_SENSOR};
use std::sync::Once;
use std::{panic::Location, slice};

#[cfg(trace_compares)]
use crate::data_structures::HBitSet;

#[cfg(trace_compares)]
extern "C" {
    #[link_name = "llvm.returnaddress"]
    fn __return_address(l: i32) -> *const u8;
}

#[cfg(trace_compares)]
#[inline]
unsafe fn return_address() -> usize {
    __return_address(0) as usize
}

// TODO: reenable at some point
// #[thread_local]
// #[export_name = "__sancov_lowest_stack"]
static mut LOWEST_STACK: libc::uintptr_t = usize::MAX;

static START: Once = Once::new();

// #[export_name = "__sanitizer_cov_bool_flag_init"]
// fn bool_flags_init(start: *mut bool, stop: *mut bool) {
//     unsafe {
//         if !(start != stop && *start == false) {
//             return;
//         }

//         let dist = stop.offset_from(start).abs() as usize;
//         START.call_once(|| {
//             println!("Number of counters: {}", dist);
//             SHARED_SENSOR.as_mut_ptr().write(CodeCoverageSensor {
//                 eight_bit_counters: slice::from_raw_parts_mut(start, dist),
//                 _lowest_stack: &mut LOWEST_STACK,
//                 lowest_stack: usize::MAX,
//                 #[cfg(trace_compares)]
//                 instr_features: HBitSet::new(),
//             });
//         });
//     }
// }

#[export_name = "__sanitizer_cov_trace_pc_guard_init"]
unsafe fn pcguard_init(start: *mut u32, stop: *mut u32) {
    if start == stop {
        return;
    }

    let dist = stop.offset_from(start).abs() as usize;
    START.call_once(|| {
        let l = Location::caller();
        println!("{:?}", l);
        println!("Number of counters: {}", dist);
        SHARED_SENSOR.as_mut_ptr().write(CodeCoverageSensor {
            eight_bit_counters: slice::from_raw_parts_mut(start, dist),
            _lowest_stack: &mut LOWEST_STACK,
            lowest_stack: usize::MAX,
            #[cfg(trace_compares)]
            instr_features: HBitSet::new(),
        });
    });
}

#[export_name = "__sanitizer_cov_trace_pc_guard"]
unsafe fn trace_pc_guard(guard: *mut u32) {
    *guard += 1;
}

// #[export_name = "__sanitizer_cov_8bit_counters_init"]
// fn counters_init(start: *mut u8, stop: *mut u8) {
//     unsafe {
//         if !(start != stop && *start == 0) {
//             return;
//         }

//         let dist = stop.offset_from(start).abs() as usize;
//         START.call_once(|| {
//             println!("Number of counters: {}", dist);
//             SHARED_SENSOR.as_mut_ptr().write(CodeCoverageSensor {
//                 eight_bit_counters: slice::from_raw_parts_mut(start, dist),
//                 _lowest_stack: &mut LOWEST_STACK,
//                 lowest_stack: usize::MAX,
//                 #[cfg(trace_compares)]
//                 instr_features: HBitSet::new(),
//             });
//         });
//     }
// }

/// `__sanitizer_cov_trace_pc_indir`
///
/// Sanitizer Coverage documentation:
///
/// With an additional `...=trace-pc,indirect-calls` flag
/// `__sanitizer_cov_trace_pc_indirect(void *callee)`
///  will be inserted on every indirect call.
///
/// Fuzzcheck documentation:
///
/// We save the address of the caller and of the callee to identify the
/// indirect call and include it in the code coverage analysis.
// #[export_name = "__sanitizer_cov_trace_pc_indir"]
// fn trace_pc_indir(_callee: usize) {
//     // TODO: feature disabled for now
//     // let caller = unsafe { return_address() };
//     // sensor.handle_trace_indir(caller, callee);
// }

/// `__sanitizer_cov_trace_cmp1`
///
/// See general crate documentation about hooks inserted before specific instructions
#[cfg(trace_compares)]
#[export_name = "__sanitizer_cov_trace_cmp1"]
fn trace_cmp1(arg1: u8, arg2: u8) {
    let pc = unsafe { return_address() };
    super::handle_trace_cmp_u8(pc, arg1, arg2);
}

/// `__sanitizer_cov_trace_cmp2`
///
/// See general crate documentation about hooks inserted before specific instructions
#[cfg(trace_compares)]
#[export_name = "__sanitizer_cov_trace_cmp2"]
fn trace_cmp2(arg1: u16, arg2: u16) {
    let pc = unsafe { return_address() };
    super::handle_trace_cmp_u16(pc, arg1, arg2);
}

/// `__sanitizer_cov_trace_cmp4`
///
/// See general crate documentation about hooks inserted before specific instructions
#[cfg(trace_compares)]
#[export_name = "__sanitizer_cov_trace_cmp4"]
fn trace_cmp4(arg1: u32, arg2: u32) {
    let pc = unsafe { return_address() };
    super::handle_trace_cmp_u32(pc, arg1, arg2);
}

/// `__sanitizer_cov_trace_cmp8`
///
/// See general crate documentation about hooks inserted before specific instructions
#[cfg(trace_compares)]
#[export_name = "__sanitizer_cov_trace_cmp8"]
fn trace_cmp8(arg1: u64, arg2: u64) {
    let pc = unsafe { return_address() };
    super::handle_trace_cmp_u64(pc, arg1, arg2);
}

/// `__sanitizer_cov_trace_const_cmp1`
///
/// See general crate documentation about hooks inserted before specific instructions
#[cfg(trace_compares)]
#[export_name = "__sanitizer_cov_trace_const_cmp1"]
fn trace_const_cmp1(arg1: u8, arg2: u8) {
    let pc = unsafe { return_address() };

    super::handle_trace_cmp_u8(pc, arg1, arg2);
}

/// `__sanitizer_cov_trace_const_cmp2`
///
/// See general crate documentation about hooks inserted before specific instructions
#[cfg(trace_compares)]
#[export_name = "__sanitizer_cov_trace_const_cmp2"]
fn trace_const_cmp2(arg1: u16, arg2: u16) {
    let pc = unsafe { return_address() };
    super::handle_trace_cmp_u16(pc, arg1, arg2);
}

/// `__sanitizer_cov_trace_const_cmp4`
///
/// See general crate documentation about hooks inserted before specific instructions
#[cfg(trace_compares)]
#[export_name = "__sanitizer_cov_trace_const_cmp4"]
fn trace_const_cmp4(arg1: u32, arg2: u32) {
    let pc = unsafe { return_address() };
    super::handle_trace_cmp_u32(pc, arg1, arg2);
}

/// `__sanitizer_cov_trace_const_cmp8`
///
/// See general crate documentation about hooks inserted before specific instructions
#[cfg(trace_compares)]
#[export_name = "__sanitizer_cov_trace_const_cmp8"]
fn trace_const_cmp8(arg1: u64, arg2: u64) {
    let pc = unsafe { return_address() };
    super::handle_trace_cmp_u64(pc, arg1, arg2);
}

/// `__sanitizer_cov_trace_switch`
///
/// Sanitizer Coverage documentation:
///
/// > Called before a switch statement.
/// > Val is the switch operand.
/// > * `Cases[0]` is the number of case constants.
/// > * `Cases[1]` is the size of Val in bits.
/// > * `Cases[2:]` are the case constants.
///
/// Fuzzcheck documentation:
/// TODO
#[cfg(trace_compares)]
#[export_name = "__sanitizer_cov_trace_switch"]
fn trace_switch(val: u64, arg2: *mut u64) {
    let pc = unsafe { return_address() };

    let n = unsafe { *arg2 as usize };
    let mut cases = unsafe { slice::from_raw_parts_mut(arg2, n + 2).iter().take(1) };

    // val_size_in_bits
    let _ = cases.next();

    // TODO: understand this. actually, understand this whole method
    // if cases[n-1] < 256 && val < 256 { return }

    let (i, token) = cases
        .take_while(|&&x| x <= val) // TODO: not sure this is correct
        .fold((0 as usize, 0 as u64), |x, next| (x.0 + 1, val ^ *next));

    super::handle_trace_cmp_u64(pc + i, token, 0);
}
