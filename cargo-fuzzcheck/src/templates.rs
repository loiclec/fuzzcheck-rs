macro_rules! non_instrumented_toml_template {
    ($name: expr, $fuzzcheck_rs_dep: expr, $fuzzcheck_input_dep: expr, $fuzzcheck_serializer_dep: expr, $target: expr) => {
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

[dependencies]
serde = {{ version = "1.0" }} #, features = ["derive"] }}
serde_json = "1.0"

[dependencies.fuzzcheck]
{1}

[dependencies.fuzzcheck_mutators]
{2}

[dependencies.fuzzcheck_serializer]
{3}

# Prevent this from interfering with workspaces
[workspace]
members = ["."]

[[bin]]
name = "{4}"
path = "fuzz_targets/{4}.rs"

[profile.release]
debug = false
opt-level = 3
lto = "thin"
codegen-units = 1
panic = 'abort'
overflow-checks = false
incremental = false

[profile.release.package.serde_json]
opt-level = 0
codegen-units = 16

[profile.release.package.serde]
opt-level = 0
codegen-units = 16

[profile.release.package.libc]
opt-level = 0
codegen-units = 16

[profile.release.package.getopts]
opt-level = 0
codegen-units = 16

[profile.release.package.fuzzcheck_arg_parser]
opt-level = 0
codegen-units = 16
"##,
            $name, $fuzzcheck_rs_dep, $fuzzcheck_input_dep, $fuzzcheck_serializer_dep, $target
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

[profile.release]
debug = false
opt-level = 3
lto = "thin"
codegen-units = 1
panic = 'abort'
overflow-checks = true
incremental = false
"##,
            $name
        )
    };
}

macro_rules! build_rs_template {
    ($instrumented_target_folder_0: expr, $instrumented_target_folder_1: expr) => {
        format_args!(
            r##"
fn main() {{
    println!("cargo:rustc-link-search={0}");
    println!("cargo:rustc-link-search={1}");
    println!("cargo:rerun-if-changed={0}");
    println!("cargo:rerun-if-changed={1}");
}}
"##,
            $instrumented_target_folder_0.display(),
            $instrumented_target_folder_1.display()
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

pub fn test(input: &[u8]) -> bool {{
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

extern crate fuzzcheck_mutators;
use fuzzcheck_mutators::integer::*;
use fuzzcheck_mutators::vector::*;

#[macro_use]
extern crate fuzzcheck_serializer;

extern crate {0};

extern crate {0}_non_instrumented_fuzz;

extern crate {0}_instrumented_fuzz;
use {0}_instrumented_fuzz::test;

extern crate serde;
use serde::{{Serialize, Deserialize}};
extern crate serde_json;

define_serde_serializer!();

fn main() {{
    type Mutator = VecMutator<U8Mutator>;
    let mutator = Mutator::default();
    let serializer = SerdeSerializer::<Vec<u8>>::default();
    let _ = fuzzcheck::launch(test, mutator, serializer);
}}
"#,
            $name
        )
    };
}
