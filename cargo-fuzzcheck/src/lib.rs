pub mod project;

use project::*;

use fuzzcheck_common::arg::*;

use std::path::{Path, PathBuf};
use std::{
    fmt::{self, Display},
    process::{self, Stdio},
};

use std::process::Command;

use std::cmp::Ordering;

const TARGET: &str = env!("TARGET");

impl NonInitializedRoot {
    pub fn init_command(&self, fuzzcheck_path_or_version: &str) -> Result<(), CargoFuzzcheckError> {
        let fuzz_folder = &self.path.join("fuzz");
        let fuzz = Fuzz::init(fuzz_folder, &self.name, fuzzcheck_path_or_version);
        fuzz.write(&fuzz_folder)?;
        Ok(())
    }
}

impl Root {
    pub fn clean_command(&self, stdio: &impl Fn() -> Stdio) -> Result<(), CargoFuzzcheckError> {
        Command::new("cargo")
            .current_dir(self.non_instrumented_folder())
            .args(vec!["clean"])
            .stdout(stdio())
            .stderr(stdio())
            .output()?;

        Command::new("cargo")
            .current_dir(self.instrumented_folder())
            .args(vec!["clean"])
            .stdout(stdio())
            .stderr(stdio())
            .output()?;

        Ok(())
    }

    pub fn non_instrumented_compile(
        &self,
        target_name: &str,
        config: &FullConfig,
        stdio: &impl Fn() -> Stdio,
    ) -> Result<process::Child, CargoFuzzcheckError> {
        let mut build_rustflags = vec!["--cfg", "fuzzcheck", "-Ctarget-cpu=native"];
        if config.trace_compares {
            build_rustflags.push("--cfg");
            build_rustflags.push("trace_compares");
        }
        if use_gold_linker() {
            build_rustflags.push("-Clink-arg=-fuse-ld=gold");
        }
        let build_rustflags = build_rustflags.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        self.fuzz
            .non_instrumented
            .cargo_config
            .write_build_rustflags(build_rustflags);
        let fuzzcheck_traits_rlib_path = self
            .instrumented_folder()
            .join("target/")
            .join(TARGET)
            .join("release/deps");

        let fuzzcheck_traits_link_rustc_flags = format!("-L {}", fuzzcheck_traits_rlib_path.display());
        self.fuzz
            .non_instrumented
            .cargo_config
            .write_rustc_flags_for_link("fuzzcheck_traits", fuzzcheck_traits_link_rustc_flags);

        let mut cargo_command = Command::new("cargo");

        cargo_command
            .current_dir(self.non_instrumented_folder())
            .arg("build")
            .arg("--bin")
            .arg(target_name)
            .arg("--manifest-path")
            .arg(self.non_instrumented_folder().join("Cargo.toml"))
            .arg("--release")
            .arg("--target")
            .arg(TARGET)
            .args(config.extra_cargo_flags.clone());

        if !config.non_instrumented_default_features {
            cargo_command.arg("--no-default-features");
        }

        // non-empty
        if !config.non_instrumented_features.is_empty() {
            cargo_command
                .arg("--features")
                .args(config.non_instrumented_features.clone());
        }

        let child = cargo_command.stdout(stdio()).stderr(stdio()).spawn()?;

        Ok(child)
    }

    pub fn build_command(
        &self,
        target_name: &str,
        config: &FullConfig,
        stdio: &impl Fn() -> Stdio,
    ) -> Result<(), CargoFuzzcheckError> {
        let instrumented_command = self.instrumented_compile(config, stdio)?;
        let instrumented_output = instrumented_command.wait_with_output()?;
        if !instrumented_output.status.success() {
            return Err("Could not compile the instrumented part of the fuzz target"
                .to_string()
                .into());
        }

        let non_instrumented_command = self.non_instrumented_compile(target_name, config, stdio)?;
        let non_instrumented_output = non_instrumented_command.wait_with_output()?;
        if !non_instrumented_output.status.success() {
            return Err("Could not compile the non instrumented part of the fuzz target"
                .to_string()
                .into());
        }

        Ok(())
    }

    pub fn fuzz_target_is_built(&self, target_name: &str) -> bool {
        let exec = self
            .non_instrumented_folder()
            .join(format!("target/{}/release/{}", TARGET, target_name));

        exec.is_file()
    }

    pub fn launch_executable(
        &self,
        target_name: &str,
        config: &FullConfig,
        stdio: impl Fn() -> Stdio,
    ) -> Result<process::Child, CargoFuzzcheckError> {
        let exec = self
            .non_instrumented_folder()
            .join(format!("target/{}/release/{}", TARGET, target_name));

        let args = strings_from_config(config);

        let child = Command::new(exec).args(args).stdout(stdio()).stderr(stdio()).spawn()?;

        Ok(child)
    }

    fn instrumented_prepare_compile(&self, config: &FullConfig) -> Result<(), CargoFuzzcheckError> {
        let mut rustflags: Vec<&str> = vec![
            "--cfg",
            "fuzzcheck",
            "-Ctarget-cpu=native",
            "-Cmetadata=fuzzing",
            "-Cpasses=sancov",
        ];

        if config.lto {
            rustflags.push("-Clinker-plugin-lto=1");
        }
        let coverage_level = format!("-Cllvm-args=-sanitizer-coverage-level={}", config.coverage_level);
        rustflags.push(&coverage_level);

        if config.trace_compares {
            rustflags.push("--cfg");
            rustflags.push("trace_compares");
            rustflags.push("-Cllvm-args=-sanitizer-coverage-trace-compares");
        }
        rustflags.push("-Cllvm-args=-sanitizer-coverage-trace-pc-guard");

        if config.stack_depth {
            rustflags.push("-Cllvm-args=-sanitizer-coverage-stack-depth");
        }

        if use_gold_linker() {
            rustflags.push("-Clink-arg=-fuse-ld=gold");
        }

        self.fuzz
            .instrumented
            .cargo_config
            .write_build_rustflags(rustflags.into_iter().map(|x| x.to_string()).collect());

        Ok(())
    }

    fn instrumented_compile(
        &self,
        config: &FullConfig,
        stdio: impl Fn() -> Stdio,
    ) -> Result<process::Child, CargoFuzzcheckError> {
        self.instrumented_prepare_compile(config)?;

        let mut cargo_command = Command::new("cargo");
        cargo_command
            .current_dir(self.instrumented_folder())
            .arg("build")
            .arg("--manifest-path")
            .arg(self.instrumented_folder().join("Cargo.toml"))
            .arg("--release")
            .arg("--target")
            .arg(TARGET)
            .args(config.extra_cargo_flags.clone());

        if !config.instrumented_default_features {
            cargo_command.arg("--no-default-features");
        }
        if !config.instrumented_features.is_empty() {
            cargo_command
                .arg("--features")
                .args(config.instrumented_features.clone());
        }

        let child = cargo_command.stdout(stdio()).stderr(stdio()).spawn()?;

        Ok(child)
    }

    pub fn instrumented_open_docs(
        &self,
        config: &FullConfig,
        stdio: impl Fn() -> Stdio,
    ) -> Result<(), CargoFuzzcheckError> {
        self.instrumented_prepare_compile(config)?;

        let mut cargo_command = Command::new("cargo");
        let output = cargo_command
            .current_dir(self.instrumented_folder())
            .arg("doc")
            .arg("--open")
            .arg("--manifest-path")
            .arg(self.instrumented_folder().join("Cargo.toml"))
            .arg("--release")
            .arg("--target")
            .arg(TARGET)
            .args(config.extra_cargo_flags.clone())
            .stdout(stdio())
            .stderr(stdio())
            .output()?;

        if output.status.success() {
            Ok(())
        } else {
            Err(CargoFuzzcheckError::Str(
                "Could not generate docs for the instrumented crate".to_string(),
            ))
        }
    }

    pub fn input_minify_command(
        &self,
        target_name: &str,
        config: &FullConfig,
        stdio: &impl Fn() -> Stdio,
    ) -> Result<(), CargoFuzzcheckError> {
        let mut config = config.clone();
        let file_to_minify = if let FullFuzzerCommand::MinifyInput { input_file } = config.command {
            input_file.clone()
        } else {
            panic!()
        };

        let artifacts_folder = {
            let mut x = file_to_minify.parent().unwrap().to_path_buf();
            x.push(file_to_minify.file_stem().unwrap());
            x = x.with_extension("minified");
            x
        };

        let _ = std::fs::create_dir(&artifacts_folder);
        config.artifacts = Some(artifacts_folder.clone());

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

        let mut simplest = simplest_input_file(&artifacts_folder.as_path()).unwrap_or(file_to_minify);
        config.command = FullFuzzerCommand::Read {
            input_file: simplest.clone(),
        };

        self.build_command(target_name, &config, stdio)?;
        let child = self.launch_executable(target_name, &config, stdio)?;
        let o = child.wait_with_output()?;

        assert!(!o.status.success());

        // hjhjb.minifyd/hshs.parent() != hjhjb.minifyd/ -> copy hshs to hjhjb.minifyd/hshs
        //let destination = artifacts_folder.join(arguments.input_file.file_name());
        // if arguments.input_file.unwrap().parent() != Some(artifacts_folder.as_path()) {
        //     std::fs::copy(arguments.input_file, artifacts_folder.to_owned() + arguments.input_file);
        // }

        loop {
            simplest = simplest_input_file(&artifacts_folder).unwrap_or(simplest.clone());
            config.command = FullFuzzerCommand::MinifyInput {
                input_file: simplest.clone(),
            };
            let mut c = self.launch_executable(target_name, &config, || Stdio::inherit())?;
            c.wait()?;
        }
    }
}

pub fn strings_from_config(config: &FullConfig) -> Vec<String> {
    let mut s: Vec<String> = Vec::new();

    let corpus_in_args = config
        .in_corpus
        .as_ref()
        .map(|f| vec!["--".to_owned() + IN_CORPUS_FLAG, path_str_ref(f)])
        .unwrap_or_else(|| vec!["--".to_owned() + NO_IN_CORPUS_FLAG]);

    let corpus_out_args = config
        .out_corpus
        .as_ref()
        .map(|f| vec!["--".to_owned() + OUT_CORPUS_FLAG, path_str_ref(f)])
        .unwrap_or_else(|| vec!["--".to_owned() + NO_OUT_CORPUS_FLAG]);

    let artifacts_args = config
        .artifacts
        .as_ref()
        .map(|f| vec!["--".to_owned() + ARTIFACTS_FLAG, path_str_ref(f)])
        .unwrap_or_else(|| vec!["--".to_owned() + NO_ARTIFACTS_FLAG]);

    let input_file_args = match &config.command {
        FullFuzzerCommand::Fuzz => {
            s.push(COMMAND_FUZZ.to_owned());
            None
        }
        FullFuzzerCommand::Read { input_file } => {
            s.push(COMMAND_READ.to_owned());
            Some(input_file.clone())
        }
        FullFuzzerCommand::MinifyInput { input_file } => {
            s.push(COMMAND_MINIFY_INPUT.to_owned());
            Some(input_file.clone())
        }
        FullFuzzerCommand::MinifyCorpus { corpus_size } => {
            s.push(COMMAND_MINIFY_CORPUS.to_owned());
            s.append(&mut vec!["--".to_owned() + CORPUS_SIZE_FLAG, corpus_size.to_string()]);
            None
        }
    }
    .map(|f| vec!["--".to_owned() + INPUT_FILE_FLAG, path_str(f)]);

    if let Some(input_file_args) = input_file_args {
        s.append(&mut input_file_args.clone());
    }

    s.append(&mut corpus_in_args.clone());
    s.append(&mut corpus_out_args.clone());
    s.append(&mut artifacts_args.clone());
    s.append(&mut vec![
        "--".to_owned() + MAX_INPUT_CPLX_FLAG,
        config.max_cplx.to_string(),
    ]);

    s.append(&mut vec![
        "--".to_owned() + MAX_NBR_RUNS_FLAG,
        config.max_nbr_of_runs.to_string(),
    ]);

    s.append(&mut vec!["--".to_owned() + TIMEOUT_FLAG, config.timeout.to_string()]);

    if let Some(socket_address) = config.socket_address {
        s.append(&mut vec![
            "--".to_owned() + SOCK_ADDR_FLAG,
            format!("{}", socket_address),
        ]);
    }

    s
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
fn path_str_ref(p: &PathBuf) -> String {
    p.as_path().to_str().unwrap().to_owned()
}
