use fuzzcheck_common::arg::*;
use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use std::process;
use std::process::{Command, Stdio};

const TARGET: &str = env!("TARGET");
const BUILD_FOLDER: &str = "target/fuzzcheck";

pub fn launch_executable(
    target_name: &str,
    args: &Arguments,
    cargo_args: &[String],
    stdio: impl Fn() -> Stdio,
) -> std::io::Result<process::Child> {
    let args = string_from_args(args);
    let child = Command::new("cargo")
        .env("FUZZCHECK_ARGS", args)
        .env(
            "RUSTFLAGS",
            "-Zinstrument-coverage=except-unused-functions -Zno-profiler-runtime --cfg fuzzing -Ctarget-cpu=native -Ccodegen-units=1",
        )
        .arg("test")
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
        .spawn()?;

    Ok(child)
}

pub fn input_minify_command(
    target_name: &str,
    args: &Arguments,
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

    let child = launch_executable(target_name, &config, cargo_args, stdio)?;
    let o = child.wait_with_output()?;

    assert!(!o.status.success());

    // hjhjb.minifyd/hshs.parent() != hjhjb.minifyd/ -> copy hshs to hjhjb.minifyd/hshs
    //let destination = artifacts_folder.join(arguments.input_file.file_name());
    // if arguments.input_file.unwrap().parent() != Some(artifacts_folder.as_path()) {
    //     std::fs::copy(arguments.input_file, artifacts_folder.to_owned() + arguments.input_file);
    // }

    loop {
        simplest = simplest_input_file(&artifacts_folder).unwrap_or_else(|| simplest.clone());
        config.command = FuzzerCommand::MinifyInput {
            input_file: simplest.clone(),
        };
        let mut c = launch_executable(target_name, &config, cargo_args, Stdio::inherit)?;
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
        FuzzerCommand::MinifyCorpus { corpus_size } => {
            s.push_str(COMMAND_MINIFY_CORPUS);
            s.push_str(" ");
            s.push_str(&format!("--{} {} ", CORPUS_SIZE_FLAG, corpus_size));
            None
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

    s.push_str(&format!("--{} {} ", MAX_INPUT_CPLX_FLAG, args.max_input_cplx as usize));
    s.push_str(&format!("--{} {} ", MAX_DURATION_FLAG, args.maximum_duration.as_secs()));
    s.push_str(&format!("--{} {} ", MAX_ITERATIONS_FLAG, args.maximum_iterations ));
    if args.stop_after_first_failure {
        s.push_str(&format!("--{} ", STOP_AFTER_FIRST_FAILURE_FLAG));
    }

    if let Some(socket_address) = args.socket_address {
        s.push_str(&format!("--{} {} ", SOCK_ADDR_FLAG, socket_address,));
    }

    s
}
