use std::path::PathBuf;

use structopt::StructOpt;

#[derive(Debug, Clone, Copy, StructOpt)]
#[structopt(name = "fuzzer-command")]
pub enum FuzzerCommand {
    #[structopt(name = "minimize")]
    Minimize,
    #[structopt(name = "fuzz")]
    Fuzz,
    #[structopt(name = "read")]
    Read,
}

#[derive(Debug, StructOpt)]
pub struct CommandLineArguments {
    #[structopt(flatten)]
    pub settings: FuzzerSettings,
    #[structopt(flatten)]
    pub world_info: CommandLineFuzzerInfo
}

#[derive(Debug, StructOpt)]
pub struct FuzzerSettings {
    #[structopt(subcommand)]
    pub command: FuzzerCommand,

    #[structopt(
        long = "max-number-of-runs",
        short = "runs",
        default_value = "18446744073709551615",
        help = "The number of fuzzer iterations to run before exiting"
    )]
    pub max_nbr_of_runs: usize,

    #[structopt(
        long = "max-complexity",
        short = "cplx",
        default_value = "256",
        help = "The upper bound on the complexity of the test inputs"
    )]
    pub max_input_cplx: f64,

    #[structopt(
        long = "mutation-depth",
        short = "depth",
        default_value = "3",
        help = "The number of consecutive mutations applied to an input in a single iteration"
    )]
    pub mutate_depth: usize,
}

#[derive(Debug, StructOpt)]
pub struct CommandLineFuzzerInfo {
    #[structopt(long = "input-file", help = "Help: TODO")]
    pub input_file: Option<PathBuf>,
    #[structopt(long = "input-folder", help = "Help: TODO")]
    pub input_folder: Option<PathBuf>,
    #[structopt(long = "output-folder", help = "Help: TODO")]
    pub output_folder: Option<PathBuf>,
    #[structopt(long = "artifacts-folder", help = "Help: TODO")]
    pub artifacts_folder: Option<PathBuf>,
}
