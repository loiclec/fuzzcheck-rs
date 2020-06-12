
use std::result::Result;
use std::path::PathBuf;

use std::ffi::OsString;
use std::collections::HashMap;

mod read;
use read::CorporaError;

mod init;

#[derive(Debug)]
struct Root {
    name: String,
    fuzz: Fuzz,
    cargo_toml: CargoToml,
}

#[derive(Debug)]
struct Fuzz {
    non_instrumented: NonInstrumented,
    instrumented: Instrumented,
    corpora: Result<Corpora, CorporaError>,
    gitignore: Option<String>,
}

#[derive(Debug)]
struct NonInstrumented {
    // src: Src,
    fuzz_targets: FuzzTargets,
    build_rs: BuildRs,
    cargo_toml: CargoToml,
}

#[derive(Debug)]
struct Instrumented {
    // src: Src,
    cargo_toml: CargoToml,
}

#[derive(Debug)]
struct Corpora {
    corpora: Vec<PathBuf>
}

#[derive(Debug)]
struct BuildRs {
    content: Vec<u8>
}

#[derive(Debug)]
struct CargoToml {
    toml: toml::Value,
}

#[derive(Debug)]
struct FuzzTargets {
    targets: HashMap<OsString, Vec<u8>>,
}
