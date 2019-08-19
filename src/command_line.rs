use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};

use std::path::PathBuf;

#[derive(Debug, Clone, Copy)]
pub enum FuzzerCommand {
    Minimize,
    Fuzz,
    Read,
    Shrink,
}

pub static MAX_NBR_RUNS_FLAG: &str = "max-iter";
pub static MAX_INPUT_CPLX_FLAG: &str = "max-cplx";
pub static MUT_DEPTH_FLAG: &str = "mut-depth";
pub static INPUT_FILE_FLAG: &str = "input-file";
pub static CORPUS_IN_FLAG: &str = "corpus_in";
pub static CORPUS_OUT_FLAG: &str = "corpus_out";
pub static ARTIFACTS_FLAG: &str = "artifacts";
pub static CORPUS_SIZE_FLAG: &str = "corpus_size";
pub static DEBUG_FLAG: &str = "debug";

#[derive(Debug, Clone)]
pub struct CommandLineArguments {
    pub command: FuzzerCommand,
    pub max_nbr_of_runs: usize,
    pub max_input_cplx: f64,
    pub mutate_depth: usize,
    pub corpus_size: usize,
    pub debug: bool,
    pub input_file: Option<PathBuf>,
    pub corpus_in: Option<PathBuf>,
    pub corpus_out: Option<PathBuf>,
    pub artifacts_folder: Option<PathBuf>,
}
impl CommandLineArguments {
    pub fn from_arg_matches(app_m: &ArgMatches) -> Self {
        let (command, command_name) = match app_m.subcommand_name() {
            Some("fuzz") => (FuzzerCommand::Fuzz, "fuzz"),
            Some("read") => (FuzzerCommand::Read, "read"),
            Some("minimize") => (FuzzerCommand::Minimize, "minimize"),
            Some("shrink") => (FuzzerCommand::Shrink, "shrink"),
            _ => (FuzzerCommand::Fuzz, "fuzz"),
        };
        let app_m = app_m.subcommand_matches(command_name).unwrap_or(app_m);

        let max_nbr_of_runs: usize = app_m
            .value_of(MAX_NBR_RUNS_FLAG)
            .unwrap_or_default()
            .parse::<usize>()
            .unwrap_or(0);
        let max_input_cplx: usize = app_m
            .value_of(MAX_INPUT_CPLX_FLAG)
            .map(|x| x.parse::<usize>().ok())
            .flatten()
            .unwrap_or(256);
        let mutate_depth: usize = app_m
            .value_of(MUT_DEPTH_FLAG)
            .map(|x| x.parse::<usize>().ok())
            .flatten()
            .unwrap_or(5);
        let input_file: Option<PathBuf> = app_m
            .value_of(INPUT_FILE_FLAG)
            .map(|x| x.parse::<PathBuf>().ok())
            .flatten();
        let corpus_size: usize = app_m
            .value_of(CORPUS_SIZE_FLAG)
            .map(|x| x.parse::<usize>().ok())
            .flatten()
            .unwrap_or(100);
        let debug: bool = app_m.is_present(DEBUG_FLAG);
        let corpus_in: Option<PathBuf> = app_m
            .value_of(CORPUS_IN_FLAG)
            .map(|x| x.parse::<PathBuf>().ok())
            .flatten();
        let corpus_out: Option<PathBuf> = app_m
            .value_of(CORPUS_OUT_FLAG)
            .map(|x| x.parse::<PathBuf>().ok())
            .flatten();
        let artifacts_folder: Option<PathBuf> = app_m
            .value_of(ARTIFACTS_FLAG)
            .map(|x| x.parse::<PathBuf>().ok())
            .flatten();

        Self {
            command,
            max_nbr_of_runs,
            max_input_cplx: max_input_cplx as f64,
            mutate_depth,
            corpus_size,
            debug,
            input_file,
            corpus_in,
            corpus_out,
            artifacts_folder,
        }
    }
}

pub fn setup_app<'a, 'b>() -> App<'a, 'b> {
    let corpus_in_arg = Arg::with_name(CORPUS_IN_FLAG)
        .long(CORPUS_IN_FLAG)
        .value_name("path")
        .default_value("./fuzz-corpus/")
        .help("Folder for the input corpus");
    let corpus_out_arg = Arg::with_name(CORPUS_OUT_FLAG)
        .long(CORPUS_OUT_FLAG)
        .value_name("path")
        .default_value("./fuzz-corpus/")
        .help("Folder for the output corpus");

    let artifacts_arg = Arg::with_name(ARTIFACTS_FLAG)
        .long(ARTIFACTS_FLAG)
        .value_name("path")
        .default_value("./artifacts/")
        .help("Folder where artifacts will be written");

    let input_arg = Arg::with_name(INPUT_FILE_FLAG)
        .long(INPUT_FILE_FLAG)
        .takes_value(true)
        .value_name("path")
        .validator(|v| match v.parse::<PathBuf>() {
            Ok(p) => {
                let p = p.as_path();
                if !p.is_file() {
                    Err(String::from("path does not point to a file"))
                } else {
                    Ok(())
                }
            }
            Err(_) => Err(String::from("must be a valid path to an file")),
        })
        .required(true)
        .help("A file containing a JSON-encoded input");

    let max_iter_arg = Arg::with_name(MAX_NBR_RUNS_FLAG)
        .long(MAX_NBR_RUNS_FLAG)
        .value_name("n")
        .validator(|v| match v.parse::<u32>() {
            Ok(_) => Ok(()),
            Err(_) => Err(String::from("must be a valid positive integer")),
        })
        .default_value("0")
        .help("The maximum number of iterations. No limit if set to 0.");

    let mut_depth_arg = Arg::with_name(MUT_DEPTH_FLAG)
        .long(MUT_DEPTH_FLAG)
        .value_name("n")
        .default_value("5")
        .validator(|v| match v.parse::<u32>() {
            Ok(x) if x < 1 => Err(String::from("must be greater than 0")),
            Err(_) => Err(String::from("must be a valid integer greater than 0")),
            _ => Ok(()),
        })
        .help("The number of consecutive mutations for each input");

    let max_cplx_arg = Arg::with_name(MAX_INPUT_CPLX_FLAG)
        .long(MAX_INPUT_CPLX_FLAG)
        .value_name("n")
        .default_value("256")
        .validator(|v| match v.parse::<u32>() {
            Ok(x) if x < 1 => Err(String::from("must be greater than 0")),
            Err(_) => Err(String::from("must be a valid integer greater than 0")),
            _ => Ok(()),
        })
        .help("The maximum allowed complexity of inputs.");

    let corpus_size_arg = Arg::with_name(CORPUS_SIZE_FLAG)
        .long(CORPUS_SIZE_FLAG)
        .value_name("n")
        .default_value("100")
        .validator(|v| match v.parse::<u32>() {
            Ok(x) if x < 1 => Err(String::from("must be greater than 0")),
            Err(_) => Err(String::from("must be a valid integer greater than 0")),
            _ => Ok(()),
        })
        .help("The target size of the corpus.");

    let debug_arg = Arg::with_name(DEBUG_FLAG).long(DEBUG_FLAG).takes_value(false);

    App::new("fuzzcheck-target")
        .version(option_env!("CARGO_PKG_VERSION").unwrap_or("0.0.0"))
        .about(option_env!("CARGO_PKG_DESCRIPTION").unwrap_or(""))
        .setting(AppSettings::GlobalVersion)
        .setting(AppSettings::DeriveDisplayOrder)
        .subcommand(
            SubCommand::with_name("fuzz")
                .about("Fuzz-test the executable")
                .arg(&corpus_in_arg)
                .arg(&corpus_out_arg)
                .arg(&artifacts_arg)
                .arg(&max_cplx_arg)
                .arg(&max_iter_arg)
                .arg(&mut_depth_arg)
                .arg(&debug_arg),
        )
        .subcommand(
            SubCommand::with_name("minimize")
                .about("Minimize a crashing input")
                .arg(&input_arg)
                .arg(&artifacts_arg)
                .arg(&mut_depth_arg),
        )
        .subcommand(
            SubCommand::with_name("read")
                .about("Read a crashing input")
                .arg(&input_arg)
                .arg(&artifacts_arg),
        )
        .subcommand(
            SubCommand::with_name("shrink")
                .about("Shrink the size of a corpus")
                .arg(&corpus_in_arg)
                .arg(&corpus_out_arg)
                .arg(&corpus_size_arg)
                .arg(&debug_arg),
        )
}
