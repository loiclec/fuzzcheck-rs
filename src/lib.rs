#![feature(drain_filter)]
#![feature(never_type)]
#![feature(thread_spawn_unchecked)]
#![allow(dead_code)]

extern crate libc;

#[macro_use]
extern crate lazy_static;
extern crate structopt;

mod artifact;
mod code_coverage_sensor;
mod command_line;
mod fuzzer;
mod generators;
mod hooks;
mod input;
mod input_pool;
mod signals_handler;
mod weighted_index;
mod world;
