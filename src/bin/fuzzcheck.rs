extern crate clap;
extern crate serde_json;
use fuzzcheck::command_line::*;
use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::string::String;
use clap::Arg;
static TARGET_FLAG: &str = "target";

fn main() {
    let app = 
        setup_app().arg(Arg::with_name(TARGET_FLAG)
                .long(TARGET_FLAG)
                .takes_value(true)
                .value_name("executable path")
                .display_order(1)
                .validator(|v| {
                    match v.parse::<PathBuf>() {
                    Ok(p) => {
                        let p = p.as_path();
                        if !p.is_file() {
                            Err(String::from("path does not point to an executable file"))
                        } else {
                            Ok(())
                        }
                    }
                    Err(_) => Err(String::from("must be a valid path to an executable file"))
                    }
                })
                .required(true)
                .help("The fuzz target is the executable file containing launching a fuzzcheck fuzzer.")
            );
    let app_m = app.get_matches();
    let args = CommandLineArguments::from_arg_matches(&app_m);
    let target = Path::new(app_m.value_of(TARGET_FLAG).unwrap());

    match args.command {
        FuzzerCommand::Fuzz => fuzz_command(&target, args),
        FuzzerCommand::Minimize => minimize_command(&target, args),
        FuzzerCommand::Read => panic!("unimplemented"),
    }
}

fn fuzz_command(executable: &Path, arguments: CommandLineArguments) {
    println!("{:?}", args_to_string(&arguments));
    Command::new(executable)
        .args(args_to_string(&arguments))
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .output()
        .expect("failed to execute process");
}

// TODO: rename CommandLineArguments
fn minimize_command(executable: &Path, mut arguments: CommandLineArguments) -> ! {
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
                let data = std::fs::read_to_string(&path).ok()?;
                let json = serde_json::from_str::<serde_json::Value>(&data).ok()?;
                let complexity = json["cplx"].as_f64()?;
                Some((path.to_path_buf(), complexity))
            });
        let (file, _) = files_with_complexity
            .min_by(|x, y| std::cmp::PartialOrd::partial_cmp(&x.1, &y.1).unwrap_or(Ordering::Equal))?;
        Some(file)
    }

    if let Some(simplest) = simplest_input_file(&artifacts_folder.as_path()) {
        arguments.input_file = Some(simplest);
    }
    arguments.command = FuzzerCommand::Read;

    println!("{:?}", args_to_string(&arguments));

    let o = Command::new(executable)
        .args(args_to_string(&arguments))
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .output()
        .expect("failed to execute process");

    assert!(o.status.success() == false);

    // hjhjb.minimized/hshs.parent() != hjhjb.minimized/ -> copy hshs to hjhjb.minimized/hshs
    //let destination = artifacts_folder.join(arguments.input_file.file_name());
    // if arguments.input_file.unwrap().parent() != Some(artifacts_folder.as_path()) {
    //     std::fs::copy(arguments.input_file, artifacts_folder.to_owned() + arguments.input_file);
    // }

    arguments.command = FuzzerCommand::Minimize;

    loop {
        arguments.input_file = simplest_input_file(&artifacts_folder).or(arguments.input_file);
        println!("{:?}", args_to_string(&arguments));

        Command::new(executable)
            .args(args_to_string(&arguments))
            .stdout(std::process::Stdio::inherit())
            .output()
            .expect("failed to execute process");
    }
}

fn args_to_string(args: &CommandLineArguments) -> Vec<String> {
    let mut s: Vec<String> = Vec::new();

    let input_file_args = args.input_file.clone().map(|f| vec!["--".to_owned() + INPUT_FILE_FLAG, f.as_path().to_str().unwrap().to_string()]);
    let corpus_in_args = args.corpus_in.clone().map(|f| vec!["--".to_owned() + CORPUS_IN_FLAG, f.as_path().to_str().unwrap().to_string()]);
    let corpus_out_args = args.corpus_out.clone().map(|f| vec!["--".to_owned() + CORPUS_OUT_FLAG, f.as_path().to_str().unwrap().to_string()]);
    let artifacts_args = args.artifacts_folder.clone().map(|f| vec!["--".to_owned() + ARTIFACTS_FLAG, f.as_path().to_str().unwrap().to_string()]);

    match args.command {
        FuzzerCommand::Read => {
            s.push("read".to_owned());
            if let Some(input_file_args) = input_file_args {
                s.append(&mut input_file_args.clone());
            }
            if let Some(artifacts_args) = artifacts_args {
                s.append(&mut artifacts_args.clone());
            }
        },
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
        },
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
            s.push("--".to_owned() + MAX_NBR_RUNS_FLAG);
            s.push(args.max_nbr_of_runs.to_string());
            s.push("--".to_owned() + MAX_INPUT_CPLX_FLAG);
            s.push(args.max_input_cplx.to_string());
            s.push("--".to_owned() + MUT_DEPTH_FLAG);
            s.push(args.mutate_depth.to_string());
        },
    }
    s
}
