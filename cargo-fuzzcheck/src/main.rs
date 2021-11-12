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

    parser.optflag("", "lib", "Test only this package's library unit tests");
    parser.optopt("", "bin", "Test only the specified binary", "<NAME>");
    parser.optopt("", "test", "Test only the specified test target", "<NAME>");

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
        return Err(Box::new(ArgumentsError::NoArgumentsGiven(help(&parser))));
    }

    let start_idx = if env_args[1] == "fuzzcheck" { 2 } else { 1 };

    if env_args.len() <= start_idx {
        return Err(Box::new(ArgumentsError::NoArgumentsGiven(help(&parser))));
    }

    let string_args = env_args[start_idx..]
        .into_iter()
        .map(|s| s.as_str())
        .collect::<Vec<_>>();

    let matches = parser.parse(string_args.clone()).map_err(ArgumentsError::Parsing)?;

    let target_name = &matches.free[0];

    let cargo_args: Option<String> = matches.opt_get(CARGO_ARGS_FLAG)?;

    let cargo_args = cargo_args
        .map(|x| x.split_ascii_whitespace().map(|s| s.to_string()).collect::<Vec<_>>())
        .unwrap_or(vec![]);

    let mut args = match Arguments::from_matches(&matches, true) {
        Ok(r) => r,
        Err(ArgumentsError::WantsHelp) => {
            println!("{}", help(&parser));
            return Ok(());
        }
        Err(e) => {
            return Err(Box::new(e));
        }
    };

    let lib = matches.opt_present("lib");
    let bin = matches.opt_present("bin");
    let test = matches.opt_present("test");
    let count_defined = [lib, bin, test]
        .into_iter()
        .fold(0, |acc, next| acc + if next { 1 } else { 0 });
    if count_defined != 1 {
        return Err(Box::new(ArgumentsError::Validation(
            "Exactly one of --lib, --test <NAME>, or --bin <NAME> must be given.".to_string(),
        )));
    }
    let compiled_target = if lib {
        CompiledTarget::Lib
    } else if test {
        let test_name = matches.opt_get::<String>("test").unwrap().unwrap();
        CompiledTarget::Test(test_name)
    } else if bin {
        let bin_name = matches.opt_get::<String>("bin").unwrap().unwrap();
        CompiledTarget::Bin(bin_name)
    } else {
        unreachable!();
    };

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
            let exec = launch_executable(
                target_name,
                &args,
                &compiled_target,
                &cargo_args,
                &process::Stdio::inherit,
            )?;
            exec.wait_with_output()?;
        }
        FuzzerCommand::MinifyInput { .. } => {
            input_minify_command(
                target_name,
                &args,
                &compiled_target,
                &cargo_args,
                &process::Stdio::inherit,
            )?;
        }
        FuzzerCommand::Read { .. } => {
            let exec = launch_executable(
                target_name,
                &args,
                &compiled_target,
                &cargo_args,
                &process::Stdio::inherit,
            )?;
            exec.wait_with_output()?;
        }
    }
    Ok(())
}
