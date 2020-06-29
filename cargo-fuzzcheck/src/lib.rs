pub mod project;

use init::{DEFAULT_COVERAGE_LEVEL, DEFAULT_LTO, DEFAULT_TRACE_COMPARES};
use project::*;

use fuzzcheck_arg_parser::*;

use std::fmt::{self, Display};
use std::path::{Path, PathBuf};

use std::process::Command;

use std::cmp::Ordering;

impl NonInitializedRoot {
    pub fn init_command(&self, fuzzcheck_path_or_version: &str) -> Result<(), CargoFuzzcheckError> {
        let fuzz_folder = &self.path.join("fuzz");
        let fuzz = Fuzz::init(fuzz_folder, &self.name, fuzzcheck_path_or_version);
        fuzz.write(&fuzz_folder)?;
        Ok(())
    }
}

impl Root {
    pub fn clean_command(&self) -> Result<(), CargoFuzzcheckError> {
        Command::new("cargo")
            .current_dir(self.non_instrumented_folder())
            .args(vec!["clean"])
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .output()?;

        Command::new("cargo")
            .current_dir(self.instrumented_folder())
            .args(vec!["clean"])
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .output()?;

        Ok(())
    }

    pub fn run_command(
        &self,
        args: &CommandLineArguments,
        target_name: &str,
    ) -> Result<std::process::Output, CargoFuzzcheckError> {
        let config = self.fuzz.config_toml.resolved_config(target_name);

        let s = self.command_line_arguments_string(&config, args, target_name);

        self.instrumented_compile(target_name)?;


        let mut rustflags: String = "--cfg fuzzcheck -Ctarget-cpu=native".to_string();
        
        if config.trace_compares.unwrap_or(DEFAULT_TRACE_COMPARES) {
            rustflags.push_str(" --cfg trace_compares");
        }

        {
            let instrumented_folder = self.instrumented_folder();

            let instrumented_target_folder_0 = instrumented_folder.join("target/release/deps");
            let instrumented_target_folder_1 =
                instrumented_folder.join(format!("target/{}/release/deps", default_target()));

            rustflags.push_str(&format!(" -L all={} -L all={}", instrumented_target_folder_0.display(), instrumented_target_folder_1.display()));
        }

        if use_gold_linker() {
            rustflags.push_str(" -Clink-arg=-fuse-ld=gold");
        }

        let mut cargo_command = Command::new("cargo");

        cargo_command
            .env("RUSTFLAGS", rustflags)
            .arg("run")
            .arg("--bin")
            .arg(target_name)
            .arg("--manifest-path")
            .arg(self.non_instrumented_folder().join("Cargo.toml"))
            .arg("--release")
            .arg("--target")
            .arg(default_target())
            .args(config.extra_cargo_flags.unwrap_or(vec![]));

        if matches!(config.non_instrumented_default_features, Some(false)) {
            cargo_command.arg("--no-default-features");
        }
        if let Some(features) = config.non_instrumented_features { // non-empty
            if !features.is_empty() {
                cargo_command
                .arg("--features")
                .args(features);
            }
        }
        // TODO: features!!
        cargo_command
            .arg("--")
            .args(s)
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .output()
            .map_err(|e| e.into())
    }

    pub fn launch_executable(&self, args: &CommandLineArguments, target_name: &str) -> Result<(), CargoFuzzcheckError> {
        let config = self.fuzz.config_toml.resolved_config(target_name);

        let s = self.command_line_arguments_string(&config, args, target_name);

        let exec = self
            .non_instrumented_folder()
            .join(format!("target/{}/release/{}", default_target(), target_name));

        Command::new(exec)
            .args(s)
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .output()?;

        Ok(())
    }

    fn instrumented_compile(&self, target_name: &str) -> Result<(), CargoFuzzcheckError> {
        let mut rustflags: String = "--cfg fuzzcheck \
                                     -Ctarget-cpu=native \
                                     -Cmetadata=fuzzing \
                                     -Cpasses=sancov"
            .into();

        let config = self.fuzz.config_toml.resolved_config(target_name);

        if config.lto.unwrap_or(DEFAULT_LTO) {
            rustflags.push_str(" -Clinker-plugin-lto=1");
        }

        rustflags.push_str(&format!(
            " -Cllvm-args=-sanitizer-coverage-level={}",
            config.coverage_level.unwrap_or(DEFAULT_COVERAGE_LEVEL)
        ));

        if config.trace_compares.unwrap_or(DEFAULT_TRACE_COMPARES) {
            rustflags.push_str(" --cfg trace_compares");
            rustflags.push_str(" -Cllvm-args=-sanitizer-coverage-trace-compares");
        }
        rustflags.push_str(" -Cllvm-args=-sanitizer-coverage-inline-8bit-counters");

        /*
        if config.trace_compares.unwrap_or(DEFAULT_STACK_DEPTH) {
            rustflags.push_str(" -Cllvm-args=-sanitizer-coverage-stack-depth");
        }
        */

        if use_gold_linker() {
            rustflags.push_str(" -Clink-arg=-fuse-ld=gold");
        }

        let mut cargo_command = Command::new("cargo");

        cargo_command
            .env("RUSTFLAGS", rustflags)
            .arg("build")
            .arg("--manifest-path")
            .arg(self.instrumented_folder().join("Cargo.toml"))
            .arg("--release")
            .arg("--target")
            .arg(default_target())
            .args(config.extra_cargo_flags.unwrap_or(vec![]));

        if matches!(config.instrumented_default_features, Some(false)) {
            cargo_command.arg("--no-default-features");
            println!("no default features!");
        }
        if let Some(features) = config.instrumented_features { 
            if !features.is_empty() {
                cargo_command
                .arg("--features")
                .args(features);
            }
        }

        let output = cargo_command
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .output()?;

        if output.status.success() {
            Ok(())
        } else {
            Err("Could not compile the instrumented part of the fuzz target"
                .to_string()
                .into())
        }
    }

    pub fn input_minify_command(
        &self,
        arguments: &CommandLineArguments,
        target_name: &str,
    ) -> Result<(), CargoFuzzcheckError> {
        let mut arguments = arguments.clone();

        let file_to_minify = (&arguments.input_file).as_ref().unwrap().clone();

        let artifacts_folder = {
            let mut x = file_to_minify.parent().unwrap().to_path_buf();
            x.push(file_to_minify.file_stem().unwrap());
            x = x.with_extension("minified");
            x
        };

        let _ = std::fs::create_dir(&artifacts_folder);
        arguments.artifacts_folder = Some(artifacts_folder.clone());

        fn simplest_input_file(folder: &Path) -> Option<PathBuf> {
            let files_with_complexity = std::fs::read_dir(folder)
                .ok()?
                .filter_map(|path| -> Option<(PathBuf, f64)> {
                    let path = path.ok()?.path();
                    let name_components: Vec<&str> = path.file_stem()?.to_str()?.splitn(2, "--").collect();
                    if name_components.len() == 2 {
                        let cplx = name_components[0].parse::<f64>().ok()?;
                        Some((path.to_path_buf(), cplx))
                    } else {
                        None
                    }
                });

            files_with_complexity
                .min_by(|x, y| std::cmp::PartialOrd::partial_cmp(&x.1, &y.1).unwrap_or(Ordering::Equal))
                .map(|x| x.0)
        }

        if let Some(simplest) = simplest_input_file(&artifacts_folder.as_path()) {
            arguments.input_file = Some(simplest);
        }
        arguments.command = FuzzerCommand::Read;

        let o = self.run_command(&arguments, target_name)?;

        assert!(!o.status.success());

        // hjhjb.minifyd/hshs.parent() != hjhjb.minifyd/ -> copy hshs to hjhjb.minifyd/hshs
        //let destination = artifacts_folder.join(arguments.input_file.file_name());
        // if arguments.input_file.unwrap().parent() != Some(artifacts_folder.as_path()) {
        //     std::fs::copy(arguments.input_file, artifacts_folder.to_owned() + arguments.input_file);
        // }

        arguments.command = FuzzerCommand::MinifyInput;

        loop {
            arguments.input_file = simplest_input_file(&artifacts_folder).or(arguments.input_file);

            self.launch_executable(&arguments, target_name)?;
        }
    }

    fn command_line_arguments_string(
        &self,
        config: &Config,
        args: &CommandLineArguments,
        target_name: &str,
    ) -> Vec<String> {
        let mut s: Vec<String> = Vec::new();

        let mut defaults = DEFAULT_ARGUMENTS.clone();

        let defaults_corpus = self
            .corpora_folder()
            .join(target_name)
            .as_path()
            .to_str()
            .unwrap()
            .to_owned();
        let defaults_artifacts = self
            .artifacts_folder()
            .join(target_name)
            .as_path()
            .to_str()
            .unwrap()
            .to_owned();

        defaults.in_corpus = &defaults_corpus;
        defaults.out_corpus = &defaults_corpus;
        defaults.artifacts = &defaults_artifacts;

        let args = config.resolve_arguments(args).resolved(defaults);

        let input_file_args = args
            .input_file
            .clone()
            .map(|f| vec!["--".to_owned() + INPUT_FILE_FLAG, path_str(f)]);

        let corpus_in_args = args
            .corpus_in
            .map(|f| vec!["--".to_owned() + IN_CORPUS_FLAG, path_str(f)])
            .unwrap_or_else(|| vec!["--".to_owned() + NO_IN_CORPUS_FLAG]);

        let corpus_out_args = args
            .corpus_out
            .map(|f| vec!["--".to_owned() + OUT_CORPUS_FLAG, path_str(f)])
            .unwrap_or_else(|| vec!["--".to_owned() + NO_OUT_CORPUS_FLAG]);

        let artifacts_args = args
            .artifacts_folder
            .map(|f| vec!["--".to_owned() + ARTIFACTS_FLAG, path_str(f)])
            .unwrap_or_else(|| vec!["--".to_owned() + NO_ARTIFACTS_FLAG]);

        match args.command {
            FuzzerCommand::Read => s.push(COMMAND_READ.to_owned()),
            FuzzerCommand::MinifyInput => s.push(COMMAND_MINIFY_INPUT.to_owned()),
            FuzzerCommand::MinifyCorpus => s.push(COMMAND_MINIFY_CORPUS.to_owned()),
            FuzzerCommand::Fuzz => s.push(COMMAND_FUZZ.to_owned()),
        };

        if let Some(input_file_args) = input_file_args {
            s.append(&mut input_file_args.clone());
        }
        s.append(&mut vec![
            "--".to_owned() + CORPUS_SIZE_FLAG,
            args.corpus_size.to_string(),
        ]);

        s.append(&mut corpus_in_args.clone());
        s.append(&mut corpus_out_args.clone());
        s.append(&mut artifacts_args.clone());
        s.append(&mut vec![
            "--".to_owned() + MAX_INPUT_CPLX_FLAG,
            args.max_input_cplx.to_string(),
        ]);

        s.append(&mut vec![
            "--".to_owned() + MAX_NBR_RUNS_FLAG,
            args.max_nbr_of_runs.to_string(),
        ]);

        s.append(&mut vec!["--".to_owned() + TIMEOUT_FLAG, args.timeout.to_string()]);

        s
    }
}

fn use_gold_linker() -> bool {
    match Command::new("which") // check if the gold linker is available
        .args(&["ld.gold"])
        .status()
    {
        Err(_) => false,
        Ok(status) => match status.code() {
            Some(0) => true,
            _ => false,
        },
    }
}

#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
pub fn default_target() -> &'static str {
    "x86_64-apple-darwin"
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub fn default_target() -> &'static str {
    "x86_64-unknown-linux-gnu"
}

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
pub fn default_target() -> &'static str {
    "aarch64-unknown-linux-gnu"
}


#[derive(Debug)]
pub enum CargoFuzzcheckError {
    Io(std::io::Error),
    Str(String),
    NonInitializedRoot(project::read::NonInitializedRootError),
    Root(project::read::RootError),
}
impl Display for CargoFuzzcheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CargoFuzzcheckError::Io(e) => write!(f, "{}", e),
            CargoFuzzcheckError::Str(e) => write!(f, "{}", e),
            CargoFuzzcheckError::NonInitializedRoot(e) => write!(f, "{:?}", e),
            CargoFuzzcheckError::Root(e) => write!(f, "{:?}", e),
        }
    }
}
impl From<std::io::Error> for CargoFuzzcheckError {
    fn from(e: std::io::Error) -> Self {
        CargoFuzzcheckError::Io(e)
    }
}
impl From<project::read::NonInitializedRootError> for CargoFuzzcheckError {
    fn from(e: project::read::NonInitializedRootError) -> Self {
        CargoFuzzcheckError::NonInitializedRoot(e)
    }
}
impl From<project::read::RootError> for CargoFuzzcheckError {
    fn from(e: project::read::RootError) -> Self {
        CargoFuzzcheckError::Root(e)
    }
}
impl From<String> for CargoFuzzcheckError {
    fn from(e: String) -> Self {
        CargoFuzzcheckError::Str(e)
    }
}

fn path_str(p: PathBuf) -> String {
    p.as_path().to_str().unwrap().to_owned()
}
