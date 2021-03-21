pub mod init;
pub mod read;
pub mod write;

use decent_toml_rs_alternative as toml;
use read::CargoConfigError;
use toml::TomlValue;
use toml::{FromToml, ToToml};

extern crate fuzzcheck_common;
use fuzzcheck_common::arg::{CommandLineArguments, DefaultArguments, FuzzerCommand};

use std::path::PathBuf;
use std::{fmt::Display, result::Result};

use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;

use crate::TARGET;

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
    pub cargo_config: CargoConfig,
}

#[derive(Debug)]
pub struct Instrumented {
    pub src: SrcLibRs,
    pub cargo_toml: CargoToml,
    pub cargo_config: CargoConfig,
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
    pub toml: HashMap<String, TomlValue>,
}

#[derive(Debug)]
pub struct FuzzTargets {
    pub targets: HashMap<OsString, Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct CargoConfig {
    pub path: PathBuf,
}

impl CargoConfig {
    pub fn get_toml(&self) -> Result<HashMap<String, TomlValue>, CargoConfigError> {
        let bytes = fs::read(&self.path)?;
        let string = String::from_utf8_lossy(&bytes);
        let toml = toml::parse_toml(&string)?;
        Ok(toml)
    }
    pub fn get_build_rustflags(&self) -> Vec<String> {
        match self.get_toml() {
            Ok(content) => {
                if let Some(rustflags) = content.get("build").and_then(|build| build.get("rustflags")) {
                    if let TomlValue::Array(flags) = rustflags {
                        flags
                            .into_iter()
                            .map(|flag| {
                                if let TomlValue::String(flag) = flag {
                                    flag.clone()
                                } else {
                                    panic!("build.rustflags contains an element that is not a string")
                                }
                            })
                            .collect()
                    } else {
                        panic!("build.rustflags should be an array of strings")
                    }
                } else {
                    vec![]
                }
            }
            Err(e) => panic!("Error while reading non_instrumented/.cargo/config.toml: {:?}", e),
        }
    }
    pub fn write_build_rustflags(&self, new_rustflags: Vec<String>) {
        let mut toml_content = self.get_toml().unwrap_or(<_>::default());
        let build = toml_content
            .entry("build".to_string())
            .or_insert(TomlValue::Table(<_>::default()));
        if let TomlValue::Table(build) = build {
            let rustflags = build.entry("rustflags".to_string()).or_insert(TomlValue::Array(vec![]));
            *rustflags = TomlValue::Array(new_rustflags.into_iter().map(TomlValue::String).collect());
        } else {
            panic!("build.rustflags should be an array of strings")
        }
        let new_string_content = toml::print(&toml_content);
        fs::write(&self.path, new_string_content)
            .expect("Could not write new build.rustflags to non_instrumented/.cargo/config.toml");
    }

    pub fn write_rustc_flags_for_link(&self, link: &str, new_rustc_flags: String) {
        let mut toml_content = self.get_toml().unwrap_or(<_>::default());
        let target = toml_content
            .entry("target".to_string())
            .or_insert(TomlValue::Table(<_>::default()));
        if let TomlValue::Table(target) = target {
            let target = target
                .entry(TARGET.to_string())
                .or_insert(TomlValue::Table(<_>::default()));
            if let TomlValue::Table(target) = target {
                let link = target
                    .entry(link.to_string())
                    .or_insert(TomlValue::Table(<_>::default()));
                if let TomlValue::Table(link) = link {
                    let rustc_flags = link
                        .entry("rustc-flags".to_string())
                        .or_insert(TomlValue::Array(vec![]));
                    *rustc_flags = TomlValue::String(new_rustc_flags);
                } else {
                    panic!("target.<triple>.rustcflags should be a string")
                }
            } else {
                panic!("target.<triple>.rustcflags should be a string")
            }
        } else {
            panic!("target.<triple>.rustcflags should be a string")
        }
        let new_string_content = toml::print(&toml_content);
        fs::write(&self.path, new_string_content)
            .expect("Could not write new target.<triple>.rustc-flags to non_instrumented/.cargo/config.toml");
    }
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

    pub fn default_arguments(&self, target_name: &str) -> DefaultArguments {
        let mut defaults = DefaultArguments::default();

        let defaults_corpus = self.corpora_folder().join(target_name);
        let defaults_artifacts = self.artifacts_folder().join(target_name);

        defaults.in_corpus = defaults_corpus.clone();
        defaults.out_corpus = defaults_corpus;
        defaults.artifacts = defaults_artifacts;

        defaults
    }
    pub fn full_config(&self, target_name: &str, args: &CommandLineArguments) -> FullConfig {
        let mut config = FullConfig::default();
        config.override_with_config(self.fuzz.config_toml.resolved_config(target_name));
        config.override_with_arguments_or_defaults(args);
        config
    }
}

#[derive(Debug, FromToml, ToToml)]
pub struct Config {
    pub coverage_level: Option<CoverageLevel>,
    pub trace_compares: Option<bool>,
    pub stack_depth: Option<bool>,

    pub instrumented_default_features: Option<bool>,
    pub non_instrumented_default_features: Option<bool>,
    pub instrumented_features: Option<Vec<String>>,
    pub non_instrumented_features: Option<Vec<String>>,

    pub lto: Option<bool>,
    pub extra_cargo_flags: Option<Vec<String>>,

    pub corpus_size: Option<usize>,
    pub max_cplx: Option<f64>,
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
    fn default() -> Self {
        let full = FullConfig::default();
        let args = DefaultArguments::default();
        Self {
            coverage_level: Some(full.coverage_level),
            trace_compares: Some(full.trace_compares),
            stack_depth: Some(full.stack_depth),
            instrumented_default_features: Some(full.instrumented_default_features),
            non_instrumented_default_features: Some(full.non_instrumented_default_features),
            instrumented_features: Some(full.instrumented_features),
            non_instrumented_features: Some(full.non_instrumented_features),
            lto: Some(full.lto),
            extra_cargo_flags: Some(full.extra_cargo_flags),
            corpus_size: Some(args.corpus_size),
            max_cplx: Some(full.max_cplx),
            max_nbr_of_runs: Some(full.max_nbr_of_runs),
            timeout: Some(full.timeout),
            in_corpus: None, // no value here because it is target-specific
            out_corpus: None,
            artifacts: None,
            no_in_corpus: Some(false),
            no_out_corpus: Some(false),
            no_artifacts: Some(false),
        }
    }
    fn empty() -> Self {
        Self {
            coverage_level: None,
            trace_compares: None,
            stack_depth: None,

            instrumented_default_features: None,
            non_instrumented_default_features: None,
            instrumented_features: None,
            non_instrumented_features: None,

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
}

#[derive(Clone)]
pub enum FullFuzzerCommand {
    Fuzz,
    Read { input_file: PathBuf },
    MinifyInput { input_file: PathBuf },
    MinifyCorpus { corpus_size: usize },
}

#[derive(Debug, Clone, Copy)]
pub enum CoverageLevel {
    One,
    Two,
    Three,
    Four,
}
impl Into<usize> for CoverageLevel {
    fn into(self) -> usize {
        match self {
            CoverageLevel::One => 1,
            CoverageLevel::Two => 2,
            CoverageLevel::Three => 3,
            CoverageLevel::Four => 4,
        }
    }
}
impl Display for CoverageLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", <Self as Into<usize>>::into(*self))
    }
}
impl FromToml for CoverageLevel {
    fn from_toml(from: Option<&TomlValue>) -> Option<Self> {
        from.map(|value| match value {
            TomlValue::Integer(1) => Some(Self::One),
            TomlValue::Integer(2) => Some(Self::Two),
            TomlValue::Integer(3) => Some(Self::Three),
            TomlValue::Integer(4) => Some(Self::Four),
            _ => None,
        })
        .flatten()
    }
}
impl ToToml for CoverageLevel {
    fn to_toml(&self) -> Option<TomlValue> {
        Some(TomlValue::Integer(<Self as Into<usize>>::into(*self) as i64))
    }
}

#[derive(Clone)]
pub struct FullConfig {
    pub command: FullFuzzerCommand,

    pub coverage_level: CoverageLevel,
    pub trace_compares: bool,
    pub stack_depth: bool,

    pub instrumented_default_features: bool,
    pub non_instrumented_default_features: bool,
    pub instrumented_features: Vec<String>,
    pub non_instrumented_features: Vec<String>,

    pub lto: bool,
    pub extra_cargo_flags: Vec<String>,

    pub max_cplx: f64,
    pub max_nbr_of_runs: usize,
    pub timeout: usize,

    pub in_corpus: Option<PathBuf>,
    pub out_corpus: Option<PathBuf>,
    pub artifacts: Option<PathBuf>,

    pub socket_address: Option<std::net::SocketAddr>,
}

impl Default for FullConfig {
    fn default() -> FullConfig {
        let default_arguments = DefaultArguments::default();
        FullConfig {
            command: FullFuzzerCommand::Fuzz,
            coverage_level: CoverageLevel::Four,
            trace_compares: false,
            stack_depth: false,
            instrumented_default_features: true,
            non_instrumented_default_features: true,
            instrumented_features: vec![],
            non_instrumented_features: vec![],
            lto: true,
            extra_cargo_flags: vec![],
            max_cplx: default_arguments.max_input_cplx,
            max_nbr_of_runs: default_arguments.max_nbr_of_runs,
            timeout: default_arguments.timeout,
            in_corpus: Some(default_arguments.in_corpus),
            out_corpus: Some(default_arguments.out_corpus),
            artifacts: Some(default_arguments.artifacts),
            socket_address: None,
        }
    }
}
impl FullConfig {
    fn override_with_config(&mut self, config: Config) {
        let Config {
            coverage_level,
            trace_compares,
            stack_depth,
            instrumented_default_features,
            non_instrumented_default_features,
            instrumented_features,
            non_instrumented_features,
            lto,
            extra_cargo_flags,
            corpus_size,
            max_cplx,
            max_nbr_of_runs,
            timeout,
            in_corpus,
            out_corpus,
            artifacts,
            no_in_corpus,
            no_out_corpus,
            no_artifacts,
        } = config;

        if let Some(coverage_level) = coverage_level {
            self.coverage_level = coverage_level;
        }
        if let Some(trace_compares) = trace_compares {
            self.trace_compares = trace_compares;
        }
        if let Some(stack_depth) = stack_depth {
            self.stack_depth = stack_depth;
        }
        if let Some(instrumented_default_features) = instrumented_default_features {
            self.instrumented_default_features = instrumented_default_features;
        }
        if let Some(non_instrumented_default_features) = non_instrumented_default_features {
            self.non_instrumented_default_features = non_instrumented_default_features;
        }
        if let Some(instrumented_features) = instrumented_features {
            self.instrumented_features = instrumented_features;
        }
        if let Some(non_instrumented_features) = non_instrumented_features {
            self.non_instrumented_features = non_instrumented_features;
        }
        if let Some(lto) = lto {
            self.lto = lto;
        }
        if let Some(extra_cargo_flags) = extra_cargo_flags {
            self.extra_cargo_flags = extra_cargo_flags;
        }
        if let Some(corpus_size) = corpus_size {
            if let FullFuzzerCommand::MinifyCorpus { .. } = self.command {
                self.command = FullFuzzerCommand::MinifyCorpus { corpus_size };
            }
        }
        if let Some(max_cplx) = max_cplx {
            self.max_cplx = max_cplx;
        }
        if let Some(max_nbr_of_runs) = max_nbr_of_runs {
            self.max_nbr_of_runs = max_nbr_of_runs;
        }
        if let Some(timeout) = timeout {
            self.timeout = timeout;
        }
        if let Some(in_corpus) = in_corpus {
            self.in_corpus = Some(in_corpus);
        }
        if let Some(out_corpus) = out_corpus {
            self.out_corpus = Some(out_corpus);
        }
        if let Some(artifacts) = artifacts {
            self.artifacts = Some(artifacts);
        }
        if let Some(true) = no_in_corpus {
            self.in_corpus = None;
        }
        if let Some(true) = no_out_corpus {
            self.out_corpus = None;
        }
        if let Some(true) = no_artifacts {
            self.artifacts = None;
        }
    }
    fn override_with_arguments_or_defaults(&mut self, args: &CommandLineArguments) {
        let defaults = DefaultArguments::default();

        let CommandLineArguments {
            command,
            max_nbr_of_runs,
            max_input_cplx,
            timeout,
            corpus_size,
            input_file,
            corpus_in,
            corpus_out,
            artifacts_folder,
            no_in_corpus,
            no_out_corpus,
            no_artifacts,
            socket_address,
        } = args;

        self.command = match command {
            FuzzerCommand::MinifyInput => FullFuzzerCommand::MinifyInput {
                input_file: input_file
                    .clone()
                    .expect("An input file must be given when minifying an input'"),
            },
            FuzzerCommand::Fuzz => FullFuzzerCommand::Fuzz,
            FuzzerCommand::Read => FullFuzzerCommand::Read {
                input_file: input_file
                    .clone()
                    .expect("An input file must be given when when reading an input"),
            },
            FuzzerCommand::MinifyCorpus => FullFuzzerCommand::MinifyCorpus {
                corpus_size: corpus_size.unwrap_or(defaults.corpus_size),
            },
        };

        if let Some(corpus_size) = corpus_size {
            if let FullFuzzerCommand::MinifyCorpus { .. } = self.command {
                self.command = FullFuzzerCommand::MinifyCorpus {
                    corpus_size: *corpus_size,
                };
            }
        }
        if let Some(max_cplx) = max_input_cplx {
            self.max_cplx = *max_cplx;
        }
        if let Some(max_nbr_of_runs) = max_nbr_of_runs {
            self.max_nbr_of_runs = *max_nbr_of_runs;
        }
        if let Some(timeout) = timeout {
            self.timeout = *timeout;
        }
        if let Some(in_corpus) = corpus_in {
            self.in_corpus = Some(in_corpus.clone());
        } else if matches!(self.command, FullFuzzerCommand::MinifyCorpus { .. }) {
            self.in_corpus = Some(defaults.in_corpus);
        }
        if let Some(out_corpus) = corpus_out {
            self.out_corpus = Some(out_corpus.clone());
        } else if matches!(self.command, FullFuzzerCommand::MinifyCorpus { .. }) {
            self.out_corpus = Some(defaults.out_corpus);
        }
        if let Some(artifacts) = artifacts_folder {
            self.artifacts = Some(artifacts.clone());
        }
        if let Some(()) = no_in_corpus {
            if !matches!(self.command, FullFuzzerCommand::MinifyCorpus { .. }) {
                self.in_corpus = None;
            }
        }
        if let Some(()) = no_out_corpus {
            if !matches!(self.command, FullFuzzerCommand::MinifyCorpus { .. }) {
                self.out_corpus = None;
            }
        }
        if let Some(()) = no_artifacts {
            self.artifacts = None;
        }
        if let Some(socket_address) = socket_address {
            self.socket_address = Some(socket_address.clone());
        }
    }
}

#[derive(Debug, FromToml, ToToml)]
pub struct ConfigToml {
    pub default: Config,
    pub targets: HashMap<String, Config>,
}
impl ConfigToml {
    fn empty() -> Self {
        Self {
            default: Config::empty(),
            targets: HashMap::new(),
        }
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

            instrumented_default_features: target_config
                .and_then(|c| c.instrumented_default_features.as_ref())
                .or(self.default.instrumented_default_features.as_ref())
                .cloned(),
            non_instrumented_default_features: target_config
                .and_then(|c| c.non_instrumented_default_features.as_ref())
                .or(self.default.non_instrumented_default_features.as_ref())
                .cloned(),
            instrumented_features: target_config
                .and_then(|c| c.instrumented_features.as_ref())
                .or(self.default.instrumented_features.as_ref())
                .cloned(),
            non_instrumented_features: target_config
                .and_then(|c| c.non_instrumented_features.as_ref())
                .or(self.default.non_instrumented_features.as_ref())
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
