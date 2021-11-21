use fuzzcheck_common::arg::*;
use std::cmp::Ordering;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process;
use std::process::{Command, Stdio};
const TARGET: &str = env!("TARGET");
const BUILD_FOLDER: &str = "target/fuzzcheck";

pub enum CompiledTarget {
    Lib,
    Bin(String),
    Test(String),
}
impl CompiledTarget {
    fn to_args(&self) -> Vec<String> {
        match self {
            CompiledTarget::Lib => vec!["--lib".to_owned()],
            CompiledTarget::Bin(name) => vec!["--bin".to_owned(), name.clone()],
            CompiledTarget::Test(name) => vec!["--test".to_owned(), name.clone()],
        }
    }
}

pub fn launch_executable(
    target_name: &str,
    args: &Arguments,
    name_package: Option<String>,
    compiled_target: &CompiledTarget,
    cargo_args: &[String],
    stdio: impl Fn() -> Stdio,
) -> std::io::Result<process::Child> {
    let args = string_from_args(args);
    let child = if let Some(name_package) = &name_package {
        let compiled = Command::new("cargo")
            .env("CARGO_BUILD_PIPELINING", "false")
            .env("RUSTFLAGS", "--cfg fuzzing --cfg test -Ccodegen-units=1")
            .arg("rustc")
            .args(compiled_target.to_args())
            .args(cargo_args)
            .args(["--target", TARGET])
            .arg("--release")
            .args(["--target-dir", BUILD_FOLDER])
            .arg("--")
            .arg("--test")
            .args(["-Zinstrument-coverage=except-unused-functions", "-Zno-profiler-runtime"])
            .stdout(stdio())
            .stderr(stdio())
            .spawn()?
            .wait_with_output()?;
        assert!(compiled.status.success());
        let dep_folder = PathBuf::new()
            .join(BUILD_FOLDER)
            .join(TARGET)
            .join("release")
            .join("deps");
        let files_inside_dep_folder = std::fs::read_dir(dep_folder)?;
        let mut executables = vec![];
        for file in files_inside_dep_folder {
            if let Ok(file) = file {
                let metadata = file.metadata().unwrap();
                let mode = metadata.mode();
                if metadata.is_file() && mode & 0o111 != 0 {
                    if file
                        .file_name()
                        .into_string()
                        .unwrap()
                        .as_str()
                        .starts_with(name_package)
                    {
                        let time_created = metadata.ctime();
                        executables.push((time_created, file.path()));
                    }
                }
            }
        }
        if executables.is_empty() {
            panic!("did not find executable");
        } else {
            let most_recent_exec = executables
                .iter()
                .max_by_key(|(time, _)| time)
                .map(|(_, path)| path)
                .unwrap();
            Command::new(most_recent_exec)
                .env("FUZZCHECK_ARGS", args)
                .arg("--nocapture")
                .arg("--exact")
                .arg(target_name)
                .args(["--test-threads", "1"])
                .stdout(stdio())
                .stderr(stdio())
                .spawn()?
        }
    } else {
        Command::new("cargo")
            .env("FUZZCHECK_ARGS", args)
            .env(
                "RUSTFLAGS",
                "-Zinstrument-coverage=except-unused-functions -Zno-profiler-runtime --cfg fuzzing -Ccodegen-units=1",
            )
            .arg("test")
            .args(compiled_target.to_args())
            .args(cargo_args)
            .args(["--target", TARGET])
            .arg("--release")
            .args(["--target-dir", BUILD_FOLDER])
            .arg("--")
            .arg("--nocapture")
            .arg("--exact")
            .arg(target_name)
            .args(["--test-threads", "1"])
            .stdout(stdio())
            .stderr(stdio())
            .spawn()?
    };

    Ok(child)
}

pub fn input_minify_command(
    target_name: &str,
    args: &Arguments,
    compiled_target: &CompiledTarget,
    cargo_args: &[String],
    stdio: &impl Fn() -> Stdio,
) -> std::io::Result<()> {
    let mut config = args.clone();
    let file_to_minify = if let FuzzerCommand::MinifyInput { input_file } = config.command {
        input_file
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
    config.artifacts_folder = Some(artifacts_folder.clone());
    config.stop_after_first_failure = true;

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

    let mut simplest = simplest_input_file(artifacts_folder.as_path()).unwrap_or(file_to_minify);
    config.command = FuzzerCommand::Read {
        input_file: simplest.clone(),
    };

    let child = launch_executable(target_name, &config, None, compiled_target, cargo_args, stdio)?;
    let o = child.wait_with_output()?;

    assert!(!o.status.success());

    loop {
        simplest = simplest_input_file(&artifacts_folder).unwrap_or_else(|| simplest.clone());
        config.command = FuzzerCommand::MinifyInput {
            input_file: simplest.clone(),
        };
        let mut c = launch_executable(target_name, &config, None, compiled_target, cargo_args, Stdio::inherit)?;
        c.wait()?;
    }
}

pub fn string_from_args(args: &Arguments) -> String {
    let mut s = String::new();

    let input_file = match &args.command {
        FuzzerCommand::Fuzz => {
            s.push_str(COMMAND_FUZZ);
            s.push_str(" ");
            None
        }
        FuzzerCommand::Read { input_file } => {
            s.push_str(COMMAND_READ);
            s.push_str(" ");
            Some(input_file.clone())
        }
        FuzzerCommand::MinifyInput { input_file } => {
            s.push_str(COMMAND_MINIFY_INPUT);
            s.push_str(" ");
            Some(input_file.clone())
        }
    };
    if let Some(input_file) = input_file {
        s.push_str(&format!("--{} {} ", INPUT_FILE_FLAG, input_file.display()));
    }

    let corpus_in_args = args
        .corpus_in
        .as_ref()
        .map(|f| format!("--{} {} ", IN_CORPUS_FLAG, f.display()))
        .unwrap_or_else(|| format!("--{} ", NO_IN_CORPUS_FLAG));

    s.push_str(&corpus_in_args);
    s.push_str(" ");

    let corpus_out_args = args
        .corpus_out
        .as_ref()
        .map(|f| format!("--{} {} ", OUT_CORPUS_FLAG, f.display()))
        .unwrap_or_else(|| format!("--{} ", NO_OUT_CORPUS_FLAG));

    s.push_str(&corpus_out_args);
    s.push_str(" ");

    let artifacts_args = args
        .artifacts_folder
        .as_ref()
        .map(|f| format!("--{} {} ", ARTIFACTS_FLAG, f.display()))
        .unwrap_or_else(|| format!("--{} ", NO_ARTIFACTS_FLAG));
    s.push_str(&artifacts_args);
    s.push_str(" ");

    let stats_args = args
        .stats_folder
        .as_ref()
        .map(|f| format!("--{} {} ", STATS_FLAG, f.display()))
        .unwrap_or_else(|| format!("--{} ", NO_STATS_FLAG));
    s.push_str(&stats_args);
    s.push_str(" ");

    s.push_str(&format!("--{} {} ", MAX_INPUT_CPLX_FLAG, args.max_input_cplx as usize));
    s.push_str(&format!("--{} {} ", MAX_DURATION_FLAG, args.maximum_duration.as_secs()));
    s.push_str(&format!("--{} {} ", MAX_ITERATIONS_FLAG, args.maximum_iterations));
    if args.stop_after_first_failure {
        s.push_str(&format!("--{} ", STOP_AFTER_FIRST_FAILURE_FLAG));
    }
    s
}
