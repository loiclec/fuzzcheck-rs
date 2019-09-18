//! This module implements the hooks used by SanitizerCoverage to track the
//! execution of the program.
//! For more information about SanitizerCoverage,
//! see https://clang.llvm.org/docs/SanitizerCoverage.html
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
//! SanitizerCoverage:
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

use crate::code_coverage_sensor::*;
use std::collections::HashMap;
use std::collections::HashSet;
use std::slice;
use std::sync::Once;

use crate::hasher::FuzzcheckHash;

extern "C" {
    /// Returns the address of the calling function
    fn return_address() -> usize;
}

static START: Once = Once::new();

/// __sanitizer_cov_trace_pc_guard_init
///
/// SanitizerCoverage documentation:
///
/// > This callback is inserted by the compiler as a module constructor
/// > into every DSO. 'start' and 'stop' correspond to the
/// > beginning and end of the section with the guards for the entire
/// > binary (executable or DSO). The callback will be called at least
/// > once per DSO and may be called multiple times with the same parameters.
///
/// Fuzzcheck documentation:
///
/// The implementation of this hook is largely based on SanitizerCoverage’s
/// example. It initializes a unique id to each guard in [start, stop). These
/// ids will be used by the `trace_pc_guard` hook to identify which part
/// of the program has been reached.
#[export_name = "__sanitizer_cov_trace_pc_guard_init"]
fn trace_pc_guard_init(start: *mut u32, stop: *mut u32) {
    unsafe {
        START.call_once(|| {
            SHARED_SENSOR.as_mut_ptr().write(CodeCoverageSensor {
                num_guards: 0,
                is_recording: false,
                eight_bit_counters: HashMap::with_hasher(FuzzcheckHash {}),
                features: HashSet::new(),
            });
        });
    }
    shared_sensor().handle_pc_guard_init(start, stop);
}

/// __sanitizer_cov_trace_pc_guard
///
/// SanitizerCoverage documentation:
///
/// > This callback is inserted by the compiler on every edge in the
/// > control flow (some optimizations apply).
/// > Typically, the compiler will emit the code like this:
/// > ```text
/// > if(*guard)
/// >      __sanitizer_cov_trace_pc_guard(guard);
/// > ```
/// > But for large functions it will emit a simple call:
/// > ```text
/// > __sanitizer_cov_trace_pc_guard(guard);
/// > ```
///
/// Fuzzcheck documentation:
///
/// The counter associated with the given guard is incremented.
/// That allows us to know how many times that portion of the code
/// represented by the guard has been reached.
#[export_name = "__sanitizer_cov_trace_pc_guard"]
fn trace_pc_guard(pc: *mut u32) {
    let sensor = shared_sensor();
    if !sensor.is_recording {
        return;
    }
    let idx = unsafe { *pc as usize };
    let counter = sensor.eight_bit_counters.entry(idx).or_insert(0);

    *counter = counter.wrapping_add(1);
}

/// __sanitizer_cov_trace_pc_indir
///
/// SanitizerCoverage documentation:
///
/// With an additional `...=trace-pc,indirect-calls` flag
/// `__sanitizer_cov_trace_pc_indirect(void *callee)`
///  will be inserted on every indirect call.
///
/// Fuzzcheck documentation:
///
/// We save the address of the caller and of the callee to identify the
/// indirect call and include it in the code coverage analysis.
#[export_name = "__sanitizer_cov_trace_pc_indir"]
fn trace_pc_indir(callee: usize) {
    let sensor = shared_sensor();
    if !sensor.is_recording {
        return;
    }
    let caller = unsafe { return_address() };
    sensor.handle_trace_indir(caller, callee);
}

/// __sanitizer_cov_trace_cmp1
///
/// See general crate documentation about hooks inserted before specific instructions
#[export_name = "__sanitizer_cov_trace_cmp1"]
fn trace_cmp1(arg1: u8, arg2: u8) {
    let sensor = shared_sensor();
    if !sensor.is_recording {
        return;
    }
    let pc = unsafe { return_address() };
    sensor.handle_trace_cmp(pc, u64::from(arg1), u64::from(arg2));
}

/// __sanitizer_cov_trace_cmp2
///
/// See general crate documentation about hooks inserted before specific instructions
#[export_name = "__sanitizer_cov_trace_cmp2"]
fn trace_cmp2(arg1: u16, arg2: u16) {
    let sensor = shared_sensor();
    if !sensor.is_recording {
        return;
    }
    let pc = unsafe { return_address() };
    sensor.handle_trace_cmp(pc, u64::from(arg1), u64::from(arg2));
}

/// __sanitizer_cov_trace_cmp4
///
/// See general crate documentation about hooks inserted before specific instructions
#[export_name = "__sanitizer_cov_trace_cmp4"]
fn trace_cmp4(arg1: u32, arg2: u32) {
    let sensor = shared_sensor();
    if !sensor.is_recording {
        return;
    }
    let pc = unsafe { return_address() };
    sensor.handle_trace_cmp(pc, u64::from(arg1), u64::from(arg2));
}

/// __sanitizer_cov_trace_cmp8
///
/// See general crate documentation about hooks inserted before specific instructions
#[export_name = "__sanitizer_cov_trace_cmp8"]
fn trace_cmp8(arg1: u64, arg2: u64) {
    let sensor = shared_sensor();
    if !sensor.is_recording {
        return;
    }
    let pc = unsafe { return_address() };
    sensor.handle_trace_cmp(pc, arg1, arg2);
}

/// __sanitizer_cov_trace_const_cmp1
///
/// See general crate documentation about hooks inserted before specific instructions
#[export_name = "__sanitizer_cov_trace_const_cmp1"]
fn trace_const_cmp1(arg1: u8, arg2: u8) {
    let sensor = shared_sensor();
    if !sensor.is_recording {
        return;
    }
    let pc = unsafe { return_address() };

    sensor.handle_trace_cmp(pc, u64::from(arg1), u64::from(arg2));
}

/// __sanitizer_cov_trace_const_cmp2
///
/// See general crate documentation about hooks inserted before specific instructions
#[export_name = "__sanitizer_cov_trace_const_cmp2"]
fn trace_const_cmp2(arg1: u16, arg2: u16) {
    let sensor = shared_sensor();
    if !sensor.is_recording {
        return;
    }
    let pc = unsafe { return_address() };
    sensor.handle_trace_cmp(pc, u64::from(arg1), u64::from(arg2));
}

/// __sanitizer_cov_trace_const_cmp4
///
/// See general crate documentation about hooks inserted before specific instructions
#[export_name = "__sanitizer_cov_trace_const_cmp4"]
fn trace_const_cmp4(arg1: u32, arg2: u32) {
    let sensor = shared_sensor();
    if !sensor.is_recording {
        return;
    }
    let pc = unsafe { return_address() };
    sensor.handle_trace_cmp(pc, u64::from(arg1), u64::from(arg2));
}

/// __sanitizer_cov_trace_const_cmp8
///
/// See general crate documentation about hooks inserted before specific instructions
#[export_name = "__sanitizer_cov_trace_const_cmp8"]
fn trace_const_cmp8(arg1: u64, arg2: u64) {
    let sensor = shared_sensor();
    if !sensor.is_recording {
        return;
    }
    let pc = unsafe { return_address() };
    sensor.handle_trace_cmp(pc, arg1, arg2);
}

/// __sanitizer_cov_trace_switch
///
/// SanitizerCoverage documentation:
///
/// > Called before a switch statement.
/// > Val is the switch operand.
/// > * Cases[0] is the number of case constants.
/// > * Cases[1] is the size of Val in bits.
/// > * Cases[2:] are the case constants.
///
/// Fuzzcheck documentation:
/// TODO
#[export_name = "__sanitizer_cov_trace_switch"]
fn trace_switch(val: u64, arg2: *mut u64) {
    let sensor = shared_sensor();
    if !sensor.is_recording {
        return;
    }
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

    sensor.handle_trace_cmp(pc + i, token, 0);
}

/// __sanitizer_cov_trace_div4
///
/// See general crate documentation about hooks inserted before specific instructions
#[export_name = "__sanitizer_cov_trace_div4"]
fn trace_div4(val: u32) {
    let sensor = shared_sensor();
    if !sensor.is_recording {
        return;
    }
    let pc = unsafe { return_address() };
    sensor.handle_trace_cmp(pc, u64::from(val), 0);
}

/// __sanitizer_cov_trace_div8
///
/// See general crate documentation about hooks inserted before specific instructions
#[export_name = "__sanitizer_cov_trace_div8"]
fn trace_div8(val: u64) {
    let sensor = shared_sensor();
    if !sensor.is_recording {
        return;
    }
    let pc = unsafe { return_address() };
    sensor.handle_trace_cmp(pc, val, 0);
}

/// __sanitizer_cov_trace_gep
///
/// See general crate documentation about hooks inserted before specific instructions
#[export_name = "__sanitizer_cov_trace_gep"]
fn trace_gep(idx: libc::uintptr_t) {
    let sensor = shared_sensor();
    if !sensor.is_recording {
        return;
    }
    let pc = unsafe { return_address() };
    sensor.handle_trace_cmp(pc, idx as u64, 0);
}
