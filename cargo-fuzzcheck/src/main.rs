extern crate cargo_fuzzcheck;
use cargo_fuzzcheck::*;
use fuzzcheck_common::arg::*;
use std::error::Error;
use std::path::PathBuf;
use std::process;
use std::string::String;

const CARGO_ARGS_FLAG: &str = "cargo-args";

fn main() -> Result<(), Box<dyn Error>> {
    let mut parser = options_parser();
    parser.opt(
        "",
        CARGO_ARGS_FLAG,
        "additional arguments to pass to cargo",
        "",
        getopts::HasArg::Yes,
        getopts::Occur::Optional,
    );

    let env_args: Vec<String> = std::env::args().collect();

    if env_args.len() <= 1 {
        return Err(Box::new(ArgumentsError::NoArgumentsGiven(parser)));
    }

    let start_idx = if env_args[1] == "fuzzcheck" { 2 } else { 1 };

    if env_args.len() <= start_idx {
        return Err(Box::new(ArgumentsError::NoArgumentsGiven(parser)));
    }

    let target_name = &env_args[start_idx];

    if is_help_string(&target_name) {
        println!("{}", help(&parser));
        return Ok(());
    }

    let string_args = env_args[start_idx + 1..]
        .into_iter()
        .map(|s| s.as_str())
        .collect::<Vec<_>>();

    let parsed = parser.parse(string_args.clone()).map_err(ArgumentsError::Parsing)?;

    let cargo_args: Option<String> = parsed.opt_get(CARGO_ARGS_FLAG)?;

    let cargo_args = cargo_args
        .map(|x| x.split_ascii_whitespace().map(|s| s.to_string()).collect::<Vec<_>>())
        .unwrap_or(vec![]);

    if string_args.is_empty() {
        return Err(Box::new(ArgumentsError::Validation(
            format!(
                "Both a fuzz target and a command must be given to cargo fuzzcheck. You specified the fuzz target as \"{}\", but no command was given.",
                target_name
            )
        )));
    }

    if is_help_string(&string_args[0]) {
        println!("{}", help(&parser));
        return Ok(());
    }

    let mut args = match Arguments::from_parser(&parser, &string_args) {
        Ok(r) => r,
        Err(ArgumentsError::WantsHelp) => {
            println!("{}", help(&parser));
            return Ok(());
        }
        Err(e) => {
            return Err(Box::new(e));
        }
    };
    let matches = parser.parse(&string_args).unwrap();

    match args.command {
        FuzzerCommand::Fuzz => {
            if args.corpus_in.is_none() && matches.opt_present(NO_IN_CORPUS_FLAG) == false {
                args.corpus_in = Some(PathBuf::new().join(format!("fuzz/{}/corpus", target_name)));
            }
            if args.corpus_out.is_none() && matches.opt_present(NO_OUT_CORPUS_FLAG) == false {
                args.corpus_out = Some(PathBuf::new().join(format!("fuzz/{}/corpus", target_name)));
            }
            if args.artifacts_folder.is_none() && matches.opt_present(NO_ARTIFACTS_FLAG) == false {
                args.artifacts_folder = Some(PathBuf::new().join(format!("fuzz/{}/artifacts", target_name)));
            }
            if args.stats_folder.is_none() && matches.opt_present(NO_STATS_FLAG) == false {
                args.stats_folder = Some(PathBuf::new().join(format!("fuzz/{}/stats", target_name)));
            }
            let exec = launch_executable(target_name, &args, &cargo_args, &process::Stdio::inherit)?;
            exec.wait_with_output()?;
        }
        FuzzerCommand::MinifyInput { .. } => {
            input_minify_command(target_name, &args, &cargo_args, &process::Stdio::inherit)?;
        }
        FuzzerCommand::Read { .. } => {
            let exec = launch_executable(target_name, &args, &cargo_args, &process::Stdio::inherit)?;
            exec.wait_with_output()?;
        }
        FuzzerCommand::MinifyCorpus { .. } => {
            let exec = launch_executable(target_name, &args, &cargo_args, &process::Stdio::inherit)?;
            exec.wait_with_output()?;
        }
    }
    Ok(())
}

fn is_help_string(s: &str) -> bool {
    matches!(s, "--help" | "-help" | "-h" | "help")
}
