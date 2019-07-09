use crate::fuzzer::code_coverage_sensor::*;
use std::sync::Once;

static START: Once = Once::new();

#[export_name="__sanitizer_cov_trace_pc_guard_init"]
fn trace_pc_guard_init(start: *mut u32, stop: *mut u32) {	
	unsafe {
		START.call_once(|| {
			SHARED_SENSOR.as_mut_ptr().write(
				CodeCoverageSensor {
					num_guards: 0,
					is_recording: false,
					eight_bit_counters: Vec::with_capacity(0),
					cmp_features: Vec::new()
				}
			);
		});
	}
	shared_sensor().handle_pc_guard_init(start, stop);
}

#[export_name="__sanitizer_cov_trace_pc_guard"]
fn trace_pc_guard(pc: *mut u32) {
	let sensor = shared_sensor();
	if sensor.is_recording == false { return }
	// TODO: check
	let idx = unsafe { *pc as usize };
	// TODO: overflow check
	sensor.eight_bit_counters[idx] += 1;
}

#[export_name="__sanitizer_cov_trace_cmp1"]
fn trace_cmp1(arg1: i8, arg2: i8) {
	println!("compare {:?} {:?} ", arg1, arg2);
}

#[export_name="__sanitizer_cov_trace_cmp2"]
fn trace_cmp2(arg1: i16, arg2: i16) {
	println!("compare {:?} {:?}", arg1, arg2);
}

#[export_name="__sanitizer_cov_trace_cmp4"]
fn trace_cmp4(arg1: i32, arg2: i32) {
	println!("compare {:?} {:?}", arg1, arg2);
}

#[export_name="__sanitizer_cov_trace_cmp8"]
fn trace_cmp8(arg1: i64, arg2: i64) {
	println!("compare {:?} {:?}", arg1, arg2);
}

#[export_name="__sanitizer_cov_trace_const_cmp1"]
fn trace_const_cmp1(arg1: i8, arg2: i8) {
	println!("compare {:?} {:?}", arg1, arg2);
}

#[export_name="__sanitizer_cov_trace_const_cmp2"]
fn trace_const_cmp2(arg1: i16, arg2: i16) {
	println!("compare {:?} {:?}", arg1, arg2);
}

#[export_name="__sanitizer_cov_trace_const_cmp4"]
fn trace_const_cmp4(arg1: i32, arg2: i32) {
	println!("compare {:?} {:?}", arg1, arg2);
}

#[export_name="__sanitizer_cov_trace_const_cmp8"]
fn trace_const_cmp8(arg1: i64, arg2: i64) {
	println!("compare {:?} {:?}", arg1, arg2);
}

#[export_name="__sanitizer_cov_trace_switch"]
fn trace_switch(arg1: i64, arg2: *mut i64) {
	println!("compare {:?} {:?}", arg1, arg2);
}

