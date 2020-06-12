pub mod init;
pub mod read;
pub mod write;

use std::path::PathBuf;
use std::result::Result;

use std::collections::HashMap;
use std::ffi::OsString;

#[derive(Debug)]
pub struct NonInitializedRoot {
    pub path: PathBuf,
    pub name: String,
    pub cargo_toml: CargoToml,
}

#[derive(Debug)]
pub struct Root {
    pub path: PathBuf,
    pub name: String,
    pub fuzz: Fuzz,
    pub cargo_toml: CargoToml,
}

#[derive(Debug)]
pub struct Fuzz {
    pub non_instrumented: NonInstrumented,
    pub instrumented: Instrumented,
    pub corpora: Result<Corpora, read::CorporaError>,
    pub artifacts: Result<Artifacts, read::ArtifactsError>,
    pub gitignore: Option<String>,
}

#[derive(Debug)]
pub struct NonInstrumented {
    pub src: SrcLibRs,
    pub fuzz_targets: FuzzTargets,
    pub build_rs: BuildRs,
    pub cargo_toml: CargoToml,
}

#[derive(Debug)]
pub struct Instrumented {
    pub src: SrcLibRs,
    pub cargo_toml: CargoToml,
}

#[derive(Debug)]
pub struct SrcLibRs {
    pub content: Vec<u8>,
}

#[derive(Debug)]
pub struct Corpora {
    pub corpora: Vec<PathBuf>,
}

#[derive(Debug)]
pub struct Artifacts {
    pub artifacts: Vec<PathBuf>,
}

#[derive(Debug)]
pub struct BuildRs {
    pub content: Vec<u8>,
}

#[derive(Debug)]
pub struct CargoToml {
    pub toml: toml::Value,
}

#[derive(Debug)]
pub struct FuzzTargets {
    pub targets: HashMap<OsString, Vec<u8>>,
}

impl Root {
    pub fn fuzz_folder(&self) -> PathBuf {
        self.path.join("fuzz")
    }
    pub fn non_instrumented_folder(&self) -> PathBuf {
        self.fuzz_folder().join("non_instrumented")
    }
    pub fn instrumented_folder(&self) -> PathBuf {
        self.fuzz_folder().join("instrumented")
    }
    pub fn corpora_folder(&self) -> PathBuf {
        self.fuzz_folder().join("corpora")
    }
    pub fn artifacts_folder(&self) -> PathBuf {
        self.fuzz_folder().join("artifacts")
    }
}
