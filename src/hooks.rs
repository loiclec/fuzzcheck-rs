use crate::code_coverage_sensor::*;
use std::slice;
use std::sync::Once;
use std::collections::HashMap;

extern "C" {
    fn return_address() -> usize;
}

static START: Once = Once::new();

#[export_name = "__sanitizer_cov_trace_pc_guard_init"]
fn trace_pc_guard_init(start: *mut u32, stop: *mut u32) {
    unsafe {
        START.call_once(|| {
            SHARED_SENSOR.as_mut_ptr().write(CodeCoverageSensor {
                num_guards: 0,
                is_recording: false,
                eight_bit_counters: HashMap::new(),
                cmp_features: Vec::new(),
            });
        });
    }
    shared_sensor().handle_pc_guard_init(start, stop);
}

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

#[export_name = "__sanitizer_cov_trace_cmp1"]
fn trace_cmp1(arg1: u8, arg2: u8) {
    let sensor = shared_sensor();
    if !sensor.is_recording {
        return;
    }
    let pc = unsafe { return_address() };
    sensor.handle_trace_cmp(pc, u64::from(arg1), u64::from(arg2));
}

#[export_name = "__sanitizer_cov_trace_cmp2"]
fn trace_cmp2(arg1: u16, arg2: u16) {
    let sensor = shared_sensor();
    if !sensor.is_recording {
        return;
    }
    let pc = unsafe { return_address() };
    sensor.handle_trace_cmp(pc, u64::from(arg1), u64::from(arg2));
}

#[export_name = "__sanitizer_cov_trace_cmp4"]
fn trace_cmp4(arg1: u32, arg2: u32) {
    let sensor = shared_sensor();
    if !sensor.is_recording {
        return;
    }
    let pc = unsafe { return_address() };
    sensor.handle_trace_cmp(pc, u64::from(arg1), u64::from(arg2));
}

#[export_name = "__sanitizer_cov_trace_cmp8"]
fn trace_cmp8(arg1: u64, arg2: u64) {
    let sensor = shared_sensor();
    if !sensor.is_recording {
        return;
    }
    let pc = unsafe { return_address() };
    sensor.handle_trace_cmp(pc, arg1, arg2);
}

#[export_name = "__sanitizer_cov_trace_const_cmp1"]
fn trace_const_cmp1(arg1: u8, arg2: u8) {
    let sensor = shared_sensor();
    if !sensor.is_recording {
        return;
    }
    let pc = unsafe { return_address() };
    sensor.handle_trace_cmp(pc, u64::from(arg1), u64::from(arg2));
}

#[export_name = "__sanitizer_cov_trace_const_cmp2"]
fn trace_const_cmp2(arg1: u16, arg2: u16) {
    let sensor = shared_sensor();
    if !sensor.is_recording {
        return;
    }
    let pc = unsafe { return_address() };
    sensor.handle_trace_cmp(pc, u64::from(arg1), u64::from(arg2));
}

#[export_name = "__sanitizer_cov_trace_const_cmp4"]
fn trace_const_cmp4(arg1: u32, arg2: u32) {
    let sensor = shared_sensor();
    if !sensor.is_recording {
        return;
    }
    let pc = unsafe { return_address() };
    sensor.handle_trace_cmp(pc, u64::from(arg1), u64::from(arg2));
}

#[export_name = "__sanitizer_cov_trace_const_cmp8"]
fn trace_const_cmp8(arg1: u64, arg2: u64) {
    let sensor = shared_sensor();
    if !sensor.is_recording {
        return;
    }
    let pc = unsafe { return_address() };
    sensor.handle_trace_cmp(pc, arg1, arg2);
}

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
