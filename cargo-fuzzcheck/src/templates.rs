macro_rules! toml_template {
    ($name: expr) => {
        format_args!(
            r##"
[package]
name = "{0}-fuzz"
version = "0.0.0"
authors = ["Automatically generated"]
publish = false

[package.metadata]
cargo-fuzz = true

[dependencies.{0}]
path = ".."

[dependencies.fuzzcheck_input]
"git" = "https://github.com/loiclec/fuzzcheck-rs"

# Prevent this from interfering with workspaces
[workspace]
members = ["."]
"##,
            $name
        )
    };
}

macro_rules! toml_bin_template {
    ($name: expr) => {
        format_args!(
            r#"
[[bin]]
name = "{0}"
path = "fuzz_targets/{0}.rs"
"#,
            $name
        )
    };
}

macro_rules! gitignore_template {
    () => {
        format_args!(
            r##"
target
corpus
artifacts
fuzzcheck-rs
"##
        )
    };
}

macro_rules! target_template {
    ($name: expr) => {
        format_args!(r#"
extern crate {};

extern crate fuzzcheck;
use fuzzcheck::fuzzer;

extern crate fuzzcheck_input;
use fuzzcheck_input::integer::IntegerGenerator;
use fuzzcheck_input::vector::VectorGenerator;

fn test(input: &Vec<u8>) -> bool {{
    // property test goes here
    if 
        input.len() > 7 &&
        input[0] == 0 &&
        input[1] == 167 &&
        input[2] == 200 &&
        input[3] == 103 &&
        input[4] == 56 &&
        input[5] == 78 &&
        input[6] == 2 &&
        input[7] == 254
    {{
        false
    }}
    else {{
        true
    }}
}}

fn main() {{
    // fuzzed code goes here
    let u8_gen = IntegerGenerator::<u8>::new();
    let vec_gen = VectorGenerator::new(u8_gen);
    let result = fuzzer::launch(test, vec_gen);
    println!("{{:?}}", result);
}}
"#,
            $name
        )
    };
}
