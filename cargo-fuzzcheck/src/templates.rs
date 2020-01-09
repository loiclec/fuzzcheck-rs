macro_rules! non_instrumented_toml_template {
    ($name: expr, $fuzzcheck_rs_dep: expr, $fuzzcheck_input_dep: expr, $target: expr) => {
        format_args!(
            r##"
[package]
name = "{0}-non-instrumented-fuzz"
version = "0.0.0"
authors = ["Automatically generated"]
publish = false

[package.metadata]
cargo-fuzzcheck = true

# [dependencies.{0}]
# path = "../.."
# Managed by cargo-fuzzcheck

# [dependencies.{0}-instrumented-fuzz]
# path = "../instrumented"
# Managed by cargo-fuzzcheck

[dependencies.fuzzcheck]
{1}

[dependencies.fuzzcheck_input]
{2}

# Prevent this from interfering with workspaces
[workspace]
members = ["."]

[[bin]]
name = "{3}"
path = "fuzz_targets/{3}.rs"
"##,
            $name, $fuzzcheck_rs_dep, $fuzzcheck_input_dep, $target
        )
    };
}
macro_rules! instrumented_toml_template {
    ($name: expr) => {
        format_args!(
            r##"
[package]
name = "{0}-instrumented-fuzz"
version = "0.0.0"
authors = ["Automatically generated"]
publish = false

[package.metadata]
cargo-fuzzcheck = true

[dependencies.{0}]
path = "../.."

# Prevent this from interfering with workspaces
[workspace]
members = ["."]
"##,
            $name
        )
    };
}

macro_rules! build_rs_template {
    ($instrumented_target_folder: expr) => {
        format_args!(
            r##"
fn main() {{
    println!("cargo:rustc-link-search={0}");
    println!("cargo:rerun-if-changed={0}");
}}
"##,
            $instrumented_target_folder.display()
        )
    };
}

macro_rules! gitignore_template {
    () => {
        format_args!(
            r##"
target
fuzz-corpus
artifacts
fuzzcheck-rs
"##
        )
    };
}

macro_rules! instrumented_lib_rs_template {
    ($name: expr) => {
        format_args!(
            r#"
extern crate {0};

pub fn test(input: &Vec<u8>) -> bool {{
    // test goes here
    if 
        input.len() > 14 &&
        input[0] == 0 &&
        input[1] == 167 &&
        input[2] == 200 &&
        input[3] == 103 &&
        input[4] == 56 &&
        input[5] == 78 &&
        input[6] == 2 &&
        input[7] == 254 &&
        input[8] == 0 &&
        input[9] == 167 &&
        input[10] == 200 &&
        input[11] == 103 &&
        input[12] == 56 &&
        input[13] == 78 &&
        input[14] == 103
    {{
        false
    }}
    else {{
        true
    }}
}}
"#,
            $name
        )
    };
}

macro_rules! target_template {
    ($name: expr) => {
        format_args!(
            r#"
extern crate fuzzcheck;
use fuzzcheck::fuzzer;

extern crate fuzzcheck_input;
use fuzzcheck_input::integer::*;
use fuzzcheck_input::vector::*;

extern crate {0};

extern crate {0}_non_instrumented_fuzz;

extern crate {0}_instrumented_fuzz;
use {0}_instrumented_fuzz::test;

fn main() {{
    let _ = fuzzer::launch::<_, FuzzedVector<FuzzedU8>>(test);
}}
"#,
            $name
        )
    };
}
