pub mod init;
pub mod read;
pub mod write;

extern crate fuzzcheck_arg_parser;
use fuzzcheck_arg_parser::CommandLineArguments;
use fuzzcheck_arg_parser::DEFAULT_ARGUMENTS;

extern crate serde;
use serde::{Deserialize, Serialize};

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
    pub config_toml: ConfigToml,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub coverage_level: Option<u8>,
    pub trace_compares: Option<bool>,
    pub stack_depth: Option<bool>,

    pub lto: Option<bool>,
    pub extra_cargo_flags: Option<Vec<String>>,

    pub corpus_size: Option<usize>,
    pub max_cplx: Option<usize>,
    pub max_nbr_of_runs: Option<usize>,
    pub timeout: Option<usize>,

    pub in_corpus: Option<PathBuf>,
    pub out_corpus: Option<PathBuf>,
    pub artifacts: Option<PathBuf>,

    pub no_in_corpus: Option<bool>,
    pub no_out_corpus: Option<bool>,
    pub no_artifacts: Option<bool>,
}

impl Config {
    fn empty() -> Self {
        Self {
            coverage_level: None,
            trace_compares: None,
            stack_depth: None,
            lto: None,
            extra_cargo_flags: None,

            corpus_size: None,
            max_cplx: None,
            max_nbr_of_runs: None,
            timeout: None,

            in_corpus: None,
            out_corpus: None,
            artifacts: None,

            no_in_corpus: None,
            no_out_corpus: None,
            no_artifacts: None,
        }
    }

    fn is_valid(&self) -> bool {
        matches!(self.coverage_level, Some(1..=4) | None)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigToml {
    pub default: Config,

    #[serde(flatten)]
    pub targets: HashMap<String, Config>,
}
impl ConfigToml {
    fn empty() -> Self {
        Self {
            default: Config::empty(),
            targets: HashMap::new(),
        }
    }
    pub fn is_valid(&self) -> bool {
        self.default.is_valid() && self.targets.values().all(|x| x.is_valid())
    }
    pub fn resolved_config(&self, target_name: &str) -> Config {
        let target_config = self.targets.get(target_name);

        Config {
            coverage_level: target_config
                .and_then(|c| c.coverage_level.as_ref())
                .or(self.default.coverage_level.as_ref())
                .cloned(),
            trace_compares: target_config
                .and_then(|c| c.trace_compares.as_ref())
                .or(self.default.trace_compares.as_ref())
                .cloned(),
            stack_depth: target_config
                .and_then(|c| c.stack_depth.as_ref())
                .or(self.default.stack_depth.as_ref())
                .cloned(),
            lto: target_config
                .and_then(|c| c.lto.as_ref())
                .or(self.default.lto.as_ref())
                .cloned(),
            extra_cargo_flags: target_config
                .and_then(|c| c.extra_cargo_flags.as_ref())
                .or(self.default.extra_cargo_flags.as_ref())
                .cloned(),
            corpus_size: target_config
                .and_then(|c| c.corpus_size.as_ref())
                .or(self.default.corpus_size.as_ref())
                .cloned(),
            max_nbr_of_runs: target_config
                .and_then(|c| c.max_nbr_of_runs.as_ref())
                .or(self.default.max_nbr_of_runs.as_ref())
                .cloned(),
            max_cplx: target_config
                .and_then(|c| c.max_cplx.as_ref())
                .or(self.default.max_cplx.as_ref())
                .cloned(),
            timeout: target_config
                .and_then(|c| c.timeout.as_ref())
                .or(self.default.timeout.as_ref())
                .cloned(),
            in_corpus: target_config
                .and_then(|c| c.in_corpus.as_ref())
                .or(self.default.in_corpus.as_ref())
                .cloned(),
            out_corpus: target_config
                .and_then(|c| c.out_corpus.as_ref())
                .or(self.default.out_corpus.as_ref())
                .cloned(),
            artifacts: target_config
                .and_then(|c| c.artifacts.as_ref())
                .or(self.default.artifacts.as_ref())
                .cloned(),
            no_in_corpus: target_config
                .and_then(|c| c.no_in_corpus.as_ref())
                .or(self.default.no_in_corpus.as_ref())
                .cloned(),
            no_out_corpus: target_config
                .and_then(|c| c.no_out_corpus.as_ref())
                .or(self.default.no_out_corpus.as_ref())
                .cloned(),
            no_artifacts: target_config
                .and_then(|c| c.no_artifacts.as_ref())
                .or(self.default.no_artifacts.as_ref())
                .cloned(),
        }
    }
}

impl Config {
    pub fn resolve_arguments(&self, args: &CommandLineArguments) -> CommandLineArguments {
        CommandLineArguments {
            command: args.command,
            max_input_cplx: args.max_input_cplx.or(self.max_cplx.map(|x| x as f64)),
            timeout: args.timeout.or(self.timeout),
            corpus_size: args.corpus_size.or(self.corpus_size),
            max_nbr_of_runs: args.max_nbr_of_runs.or(self.max_nbr_of_runs),
            input_file: args.input_file.clone(),
            corpus_in: args.corpus_in.clone().or(self.in_corpus.clone()),
            corpus_out: args.corpus_out.clone().or(self.out_corpus.clone()),
            artifacts_folder: args.artifacts_folder.clone().or(self.artifacts.clone()),
            no_in_corpus: args.no_in_corpus.or(self.no_in_corpus.map(|_| ())),
            no_out_corpus: args.no_out_corpus.or(self.no_out_corpus.map(|_| ())),
            no_artifacts: args.no_artifacts.or(self.no_artifacts.map(|_| ())),
        }
    }
}
