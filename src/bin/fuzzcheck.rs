extern crate clap;
use clap::{SubCommand};
use fuzzcheck::command_line::*;
use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::string::String;

static FUZZCHECK_PATH: &str = "https://github.com/loiclec/fuzzcheck-rs";
// static FUZZCHECK_REVISION: &str = "bf7948bb2b1f911197ca66af094ac20021fdd7f9";

/// The default target to pass to cargo, to workaround issue #11.
#[cfg(target_os="macos")]
pub fn default_target() -> &'static str {
    "x86_64-apple-darwin"
}

/// The default target to pass to cargo, to workaround issue #11.
#[cfg(not(target_os="macos"))]
pub fn default_target() -> &'static str {
    "x86_64-unknown-linux-gnu"
}


fn main() {
    let app = setup_app()
        .subcommand(
            SubCommand::with_name("setup")
                .about("Setup Fuzzcheck"));

    let app_m = app.get_matches();
    let target_triple = default_target();

    let args = CommandLineArguments::from_arg_matches(&app_m);

    if app_m.subcommand_name() == Some("setup") {
        setup_command()
    } else {
        match args.command {
            FuzzerCommand::Fuzz => fuzz_command(args, target_triple),
            FuzzerCommand::Minimize => minimize_command(args, target_triple),
            FuzzerCommand::Read => panic!("unimplemented"),
            FuzzerCommand::Shrink => shrink_command(args, target_triple),
        }
    }
}

fn setup_command() {
    Command::new("git")
        .args(vec!["clone", FUZZCHECK_PATH])
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .output()
        .expect("failed to execute process");

    // Command::new("git")
    //     .current_dir("fuzzcheck-rs")
    //     .args(vec!["checkout", FUZZCHECK_REVISION])
    //     .stdout(std::process::Stdio::inherit())
    //     .stderr(std::process::Stdio::inherit())
    //     .output()
    //     .expect("failed to execute process");

    Command::new("cargo")
        .current_dir("fuzzcheck-rs")
        .args(vec!["build", "--release",])
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .output()
        .expect("failed to execute process");
}

fn fuzz_command(arguments: CommandLineArguments, target_triple: &str) {    
    run_command(&arguments, target_triple);
}

fn shrink_command(arguments: CommandLineArguments, target_triple: &str) {
    run_command(&arguments, target_triple);
}

// TODO: rename CommandLineArguments
fn minimize_command(mut arguments: CommandLineArguments, target_triple: &str) -> ! {
    let file_to_minimize = (&arguments.input_file).as_ref().unwrap().clone();

    let artifacts_folder = {
        let mut x = file_to_minimize.parent().unwrap().to_path_buf();
        x.push(file_to_minimize.file_stem().unwrap());
        x = x.with_extension("minimized");
        x
    };
    let _ = std::fs::create_dir(&artifacts_folder);
    arguments.artifacts_folder = Some(artifacts_folder.clone());

    fn simplest_input_file(folder: &Path) -> Option<PathBuf> {
        let files_with_complexity = std::fs::read_dir(folder)
            .unwrap()
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
        let (file, _) = files_with_complexity
            .min_by(|x, y| std::cmp::PartialOrd::partial_cmp(&x.1, &y.1).unwrap_or(Ordering::Equal))?;
        Some(file)
    }

    if let Some(simplest) = simplest_input_file(&artifacts_folder.as_path()) {
        arguments.input_file = Some(simplest);
    }
    arguments.command = FuzzerCommand::Read;


    let o = run_command(&arguments, target_triple);
    assert!(o.status.success() == false);

    // hjhjb.minimized/hshs.parent() != hjhjb.minimized/ -> copy hshs to hjhjb.minimized/hshs
    //let destination = artifacts_folder.join(arguments.input_file.file_name());
    // if arguments.input_file.unwrap().parent() != Some(artifacts_folder.as_path()) {
    //     std::fs::copy(arguments.input_file, artifacts_folder.to_owned() + arguments.input_file);
    // }

    arguments.command = FuzzerCommand::Minimize;

    loop {
        arguments.input_file = simplest_input_file(&artifacts_folder).or(arguments.input_file);

        run_command(&arguments, target_triple);
    }
}

fn run_command(args: &CommandLineArguments, target_triple: &str) -> std::process::Output {
    let mut s: Vec<String> = Vec::new();

    let input_file_args = args.input_file.clone().map(|f| {
        vec![
            "--".to_owned() + INPUT_FILE_FLAG,
            f.as_path().to_str().unwrap().to_string(),
        ]
    });
    let corpus_in_args = args.corpus_in.clone().map(|f| {
        vec![
            "--".to_owned() + CORPUS_IN_FLAG,
            f.as_path().to_str().unwrap().to_string(),
        ]
    });
    let corpus_out_args = args.corpus_out.clone().map(|f| {
        vec![
            "--".to_owned() + CORPUS_OUT_FLAG,
            f.as_path().to_str().unwrap().to_string(),
        ]
    });
    let artifacts_args = args.artifacts_folder.clone().map(|f| {
        vec![
            "--".to_owned() + ARTIFACTS_FLAG,
            f.as_path().to_str().unwrap().to_string(),
        ]
    });

    match args.command {
        FuzzerCommand::Read => {
            s.push("read".to_owned());
            if let Some(input_file_args) = input_file_args {
                s.append(&mut input_file_args.clone());
            }
            if let Some(artifacts_args) = artifacts_args {
                s.append(&mut artifacts_args.clone());
            }
        }
        FuzzerCommand::Minimize => {
            s.push("minimize".to_owned());
            if let Some(input_file_args) = input_file_args {
                s.append(&mut input_file_args.clone());
            }
            if let Some(artifacts_args) = artifacts_args {
                s.append(&mut artifacts_args.clone());
            }
            s.push("--".to_owned() + MUT_DEPTH_FLAG);
            s.push(args.mutate_depth.to_string());
        }
        FuzzerCommand::Shrink => {
            s.push("shrink".to_owned());
            if let Some(corpus_in_args) = corpus_in_args {
                s.append(&mut corpus_in_args.clone());
            }
            if let Some(corpus_out_args) = corpus_out_args {
                s.append(&mut corpus_out_args.clone());
            }
            if args.debug {
                s.push("--debug".to_string());
            }
            s.push("--".to_owned() + CORPUS_SIZE_FLAG);
            s.push(args.corpus_size.to_string());
        }
        FuzzerCommand::Fuzz => {
            s.push("fuzz".to_owned());
            if let Some(corpus_in_args) = corpus_in_args {
                s.append(&mut corpus_in_args.clone());
            }
            if let Some(corpus_out_args) = corpus_out_args {
                s.append(&mut corpus_out_args.clone());
            }
            if let Some(artifacts_args) = artifacts_args {
                s.append(&mut artifacts_args.clone());
            }
            if args.debug {
                s.push("--debug".to_string());
            }
            s.push("--".to_owned() + MAX_NBR_RUNS_FLAG);
            s.push(args.max_nbr_of_runs.to_string());
            s.push("--".to_owned() + MAX_INPUT_CPLX_FLAG);
            s.push(args.max_input_cplx.to_string());
            s.push("--".to_owned() + MUT_DEPTH_FLAG);
            s.push(args.mutate_depth.to_string());
        }
    }
    let cur_dir = std::env::current_dir().expect("");
    let fuzzcheck_lib = cur_dir.join("fuzzcheck-rs/target/release/deps");

    let rustflags: String = format!(
        "--cfg fuzzing \
        -Cpasses=sancov \
        -Cllvm-args=-sanitizer-coverage-level=4 \
        -Cllvm-args=-sanitizer-coverage-trace-pc-guard \
        -Cllvm-args=-sanitizer-coverage-trace-compares \
        -Cllvm-args=-sanitizer-coverage-trace-divs \
        -Cllvm-args=-sanitizer-coverage-trace-geps \
        -Cllvm-args=-sanitizer-coverage-prune-blocks=0 \
        -L {}", fuzzcheck_lib.display()
    );

    Command::new("cargo")
        .env("RUSTFLAGS", rustflags)
        .arg("run")
        .arg("--release")
        .arg("--target").arg(target_triple)
        .arg("--")
        .args(s)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .output()
        .expect("failed to execute process")
}
