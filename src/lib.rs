#![feature(drain_filter)]
#![feature(never_type)]
#![feature(thread_spawn_unchecked)]
#![feature(ptr_offset_from)]
#![feature(option_flattening)]
#![allow(dead_code)]

#[macro_use]
extern crate lazy_static;

mod code_coverage_sensor;
pub mod command_line;
pub mod fuzzer;
mod hooks;
pub mod input;
mod input_pool;
mod signals_handler;
mod weighted_index;
pub mod world;
