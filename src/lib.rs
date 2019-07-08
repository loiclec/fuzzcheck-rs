
#[export_name="__sanitizer_cov_trace_pc_guard_init"]
fn trace_pc_guard_init(_start: *mut i32, _stop: *const i32) {
}

#[export_name="__sanitizer_cov_trace_pc_guard"]
fn trace_pc_guard(_pc: *mut i32) {
}

#[export_name="__sanitizer_cov_trace_cmp1"]
fn trace_cmp1(arg1: i8, arg2: i8) {
	println!("compare {:?} {:?}", arg1, arg2);
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

