use getopts::Options;
use std::{
    net::{SocketAddr, ToSocketAddrs},
    path::PathBuf,
};

#[derive(Debug, Clone, Copy)]
pub enum FuzzerCommand {
    MinifyInput,
    Fuzz,
    Read,
    MinifyCorpus,
}
impl Default for FuzzerCommand {
    fn default() -> Self {
        Self::Fuzz
    }
}

pub const TIMEOUT_FLAG: &str = "timeout";
pub const MAX_NBR_RUNS_FLAG: &str = "max-iter";
pub const MAX_INPUT_CPLX_FLAG: &str = "max-cplx";
pub const INPUT_FILE_FLAG: &str = "input-file";
pub const IN_CORPUS_FLAG: &str = "in-corpus";
pub const NO_IN_CORPUS_FLAG: &str = "no-in-corpus";
pub const OUT_CORPUS_FLAG: &str = "out-corpus";
pub const NO_OUT_CORPUS_FLAG: &str = "no-out-corpus";
pub const ARTIFACTS_FLAG: &str = "artifacts";
pub const NO_ARTIFACTS_FLAG: &str = "no-artifacts";
pub const CORPUS_SIZE_FLAG: &str = "corpus-size";
pub const SOCK_ADDR_FLAG: &str = "socket-address";

pub const COMMAND_FUZZ: &str = "fuzz";
pub const COMMAND_MINIFY_INPUT: &str = "tmin";
pub const COMMAND_MINIFY_CORPUS: &str = "cmin";
pub const COMMAND_READ: &str = "read";

#[derive(Clone)]
pub struct DefaultArguments {
    pub command: FuzzerCommand,
    pub in_corpus: PathBuf,
    pub out_corpus: PathBuf,
    pub artifacts: PathBuf,
    pub max_nbr_of_runs: usize,
    pub max_input_cplx: f64,
    pub timeout: usize,
    pub corpus_size: usize,
}
impl Default for DefaultArguments {
    fn default() -> Self {
        Self {
            command: FuzzerCommand::Fuzz,
            in_corpus: PathBuf::from("corpus".to_string()),
            out_corpus: PathBuf::from("corpus".to_string()),
            artifacts: PathBuf::from("artifacts".to_string()),
            max_nbr_of_runs: core::usize::MAX,
            max_input_cplx: 4096.0,
            timeout: 0,
            corpus_size: 100,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FullCommandLineArguments {
    pub command: FuzzerCommand,
    pub max_nbr_of_runs: usize,
    pub max_input_cplx: f64,
    pub timeout: usize,
    pub corpus_size: usize,
    pub input_file: Option<PathBuf>,
    pub corpus_in: Option<PathBuf>,
    pub corpus_out: Option<PathBuf>,
    pub artifacts_folder: Option<PathBuf>,
    pub socket_address: Option<SocketAddr>,
}

#[derive(Default, Debug, Clone)]
pub struct CommandLineArguments {
    pub command: FuzzerCommand,
    pub max_nbr_of_runs: Option<usize>,
    pub max_input_cplx: Option<f64>,
    pub timeout: Option<usize>,
    pub corpus_size: Option<usize>,
    pub input_file: Option<PathBuf>,
    pub corpus_in: Option<PathBuf>,
    pub corpus_out: Option<PathBuf>,
    pub artifacts_folder: Option<PathBuf>,

    pub no_in_corpus: Option<()>,
    pub no_out_corpus: Option<()>,
    pub no_artifacts: Option<()>,

    pub socket_address: Option<SocketAddr>,
}

#[must_use]
pub fn options_parser() -> Options {
    let mut options = Options::new();

    let defaults = DefaultArguments::default();

    options.long_only(true);
    options.optopt("", IN_CORPUS_FLAG, "folder for the input corpus", "PATH");
    options.optflag(
        "",
        NO_IN_CORPUS_FLAG,
        format!(
            "do not use an input corpus, overrides --{in_corpus}",
            in_corpus = IN_CORPUS_FLAG
        )
        .as_str(),
    );
    options.optopt("", OUT_CORPUS_FLAG, "folder for the output corpus", "PATH");
    options.optflag(
        "",
        NO_OUT_CORPUS_FLAG,
        format!(
            "do not use an output corpus, overrides --{out_corpus}",
            out_corpus = OUT_CORPUS_FLAG
        )
        .as_str(),
    );
    options.optopt("", ARTIFACTS_FLAG, "folder where the artifacts will be written", "PATH");
    options.optflag(
        "",
        NO_ARTIFACTS_FLAG,
        format!(
            "do not save artifacts, overrides --{artifacts}",
            artifacts = ARTIFACTS_FLAG
        )
        .as_str(),
    );
    options.optopt("", INPUT_FILE_FLAG, "file containing a JSON-encoded input", "PATH");
    options.optopt(
        "",
        CORPUS_SIZE_FLAG,
        format!(
            "target size of the corpus (default: {default})",
            default = defaults.corpus_size
        )
        .as_str(),
        "N",
    );
    options.optopt(
        "",
        MAX_INPUT_CPLX_FLAG,
        format!(
            "maximum allowed complexity of inputs (default: {default})",
            default = defaults.max_input_cplx
        )
        .as_str(),
        "N",
    );
    options.optopt("", MAX_NBR_RUNS_FLAG, "maximum number of iterations", "N");
    options.optopt(
        "",
        TIMEOUT_FLAG,
        format!(
            "maximum allowed time in milliseconds for a single run to finish, or 0 for no limit (default: {default})",
            default = defaults.timeout
        )
        .as_str(),
        "N",
    );

    options.optopt(
        "",
        SOCK_ADDR_FLAG,
        format!("address of the TCP socket for communication between cargo-fuzzcheck and the fuzz target",).as_str(),
        "127.0.0.1:0",
    );

    options.optflag("", "help", "print this help menu");

    options
}

impl CommandLineArguments {
    pub fn from_parser(options: &Options, args: &[String]) -> Result<Self, String> {
        let matches = options.parse(args).map_err(|e| e.to_string())?;

        // TODO: factor that out and make it prettier/more useful
        if matches.opt_present("help") || args.is_empty() {
            return Err("".to_owned());
        }

        let command: FuzzerCommand = match args[0].as_str() {
            COMMAND_FUZZ => FuzzerCommand::Fuzz,
            COMMAND_READ => FuzzerCommand::Read,
            COMMAND_MINIFY_INPUT => FuzzerCommand::MinifyInput,
            COMMAND_MINIFY_CORPUS => FuzzerCommand::MinifyCorpus,
            _ => Err(format!(
                r#"The command {c} is not supported. It can either be ‘{fuzz}’, ‘{tmin}’, or ‘{cmin}’."#,
                c = args[0],
                fuzz = COMMAND_FUZZ,
                tmin = COMMAND_MINIFY_INPUT,
                cmin = COMMAND_MINIFY_CORPUS
            ))?,
        };

        let max_input_cplx: Option<f64> = matches
            .opt_str(MAX_INPUT_CPLX_FLAG)
            .and_then(|x| x.parse::<usize>().ok())
            .map(|x| x as f64);

        let input_file: Option<PathBuf> = matches.opt_str(INPUT_FILE_FLAG).and_then(|x| x.parse::<PathBuf>().ok());

        let corpus_size: Option<usize> = matches.opt_str(CORPUS_SIZE_FLAG).and_then(|x| x.parse::<usize>().ok());

        let corpus_in: Option<PathBuf> = matches.opt_str(IN_CORPUS_FLAG).and_then(|x| x.parse::<PathBuf>().ok());

        let no_in_corpus = if matches.opt_present(NO_IN_CORPUS_FLAG) {
            Some(())
        } else {
            None
        };

        let corpus_out: Option<PathBuf> = matches.opt_str(OUT_CORPUS_FLAG).and_then(|x| x.parse::<PathBuf>().ok());

        let no_out_corpus = if matches.opt_present(NO_OUT_CORPUS_FLAG) {
            Some(())
        } else {
            None
        };

        let artifacts_folder: Option<PathBuf> = matches.opt_str(ARTIFACTS_FLAG).and_then(|x| x.parse::<PathBuf>().ok());

        let no_artifacts = if matches.opt_present(NO_ARTIFACTS_FLAG) {
            Some(())
        } else {
            None
        };

        let max_nbr_of_runs: Option<usize> = matches.opt_str(MAX_NBR_RUNS_FLAG).and_then(|x| x.parse::<usize>().ok());

        let timeout: Option<usize> = matches.opt_str(TIMEOUT_FLAG).and_then(|x| x.parse::<usize>().ok());

        let socket_address = matches.opt_str(SOCK_ADDR_FLAG).and_then(|x| {
            if let Some(mut addrs) = x.to_socket_addrs().ok() {
                addrs.next()
            } else {
                None
            }
        });

        Ok(Self {
            command,
            max_nbr_of_runs,
            max_input_cplx,
            timeout,
            corpus_size,
            input_file,
            corpus_in,
            corpus_out,
            artifacts_folder,
            no_in_corpus,
            no_out_corpus,
            no_artifacts,
            socket_address,
        })
    }

    pub fn resolved(&self, defaults: DefaultArguments) -> FullCommandLineArguments {
        let command = self.command;

        let max_input_cplx: f64 = self.max_input_cplx.unwrap_or(defaults.max_input_cplx as f64);

        let input_file: Option<PathBuf> = self.input_file.clone();

        let corpus_size: usize = self.corpus_size.unwrap_or(defaults.corpus_size);

        let corpus_in: Option<PathBuf> = if self.no_in_corpus.is_some() {
            None
        } else {
            self.corpus_in.clone()
        };

        match (command, &input_file, &corpus_in) {
            (FuzzerCommand::MinifyInput, &None, _) => {
                panic!("An input file must be given when minifying a test case".to_owned())
                // TODO: return error for that
            }
            (FuzzerCommand::MinifyCorpus, _, &None) => {
                panic!("An input corpus must be given when minifying a corpus".to_owned())
            }
            _ => (),
        }

        let corpus_out: Option<PathBuf> = if self.no_out_corpus.is_some() {
            None
        } else {
            self.corpus_out.clone()
        };

        let artifacts_folder: Option<PathBuf> = if self.no_artifacts.is_some() {
            None
        } else {
            self.artifacts_folder.clone()
        };

        let max_nbr_of_runs: usize = self.max_nbr_of_runs.unwrap_or(core::usize::MAX);

        let timeout: usize = self.timeout.unwrap_or(defaults.timeout);

        let socket_address = self.socket_address;

        FullCommandLineArguments {
            command,
            max_nbr_of_runs,
            max_input_cplx,
            timeout,
            corpus_size,
            input_file,
            corpus_in,
            corpus_out,
            artifacts_folder,
            socket_address,
        }
    }
}

impl FullCommandLineArguments {
    /// Get the command line arguments to the fuzzer from the option parser
    /// # Errors
    /// TODO
    pub fn from_parser(options: &Options, args: &[String]) -> Result<Self, String> {
        Ok(CommandLineArguments::from_parser(options, args)?.resolved(DefaultArguments::default()))
    }
}
