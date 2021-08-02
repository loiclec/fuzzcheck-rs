use getopts::Options;
use std::{
    net::{SocketAddr, ToSocketAddrs},
    path::PathBuf,
};

pub const TIMEOUT_FLAG: &str = "timeout";
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
}
impl Default for DefaultArguments {
    #[no_coverage]
    fn default() -> Self {
        Self {
            command: FuzzerCommand::Fuzz,
            in_corpus: PathBuf::from("fuzz/in_corpus"),
            out_corpus: PathBuf::from("fuzz/out_corpus"),
            artifacts: PathBuf::from("fuzz/artifacts"),
            max_nbr_of_runs: core::usize::MAX,
            max_input_cplx: 4096.0,
            timeout: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub enum FuzzerCommand {
    Fuzz,
    Read { input_file: PathBuf },
    MinifyInput { input_file: PathBuf },
    MinifyCorpus { corpus_size: usize },
}
impl Default for FuzzerCommand {
    fn default() -> Self {
        Self::Fuzz
    }
}

#[derive(Debug, Clone)]
pub struct Arguments {
    pub command: FuzzerCommand,
    pub max_input_cplx: f64,
    pub timeout: usize,
    pub corpus_in: Option<PathBuf>,
    pub corpus_out: Option<PathBuf>,
    pub artifacts_folder: Option<PathBuf>,
    pub socket_address: Option<SocketAddr>,
}

#[must_use]
#[no_coverage]
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
    options.optopt("", CORPUS_SIZE_FLAG, "target size of the corpus", "N");
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
        "address of the TCP socket for communication between cargo-fuzzcheck and the fuzz target",
        "127.0.0.1:0",
    );

    options.optflag("", "help", "print this help menu");

    options
}

impl Arguments {
    #[no_coverage]
    pub fn from_parser(options: &Options, args: &[&str]) -> Result<Self, String> {
        let matches = options.parse(args).map_err(|e| e.to_string())?;

        // TODO: factor that out and make it prettier/more useful
        if matches.opt_present("help") || args.is_empty() {
            return Err("".to_owned());
        }

        if !matches!(
            args[0],
            COMMAND_FUZZ | COMMAND_READ | COMMAND_MINIFY_INPUT | COMMAND_MINIFY_CORPUS
        ) {
            return Err(format!(
                r#"The command {c} is not supported. It can either be ‘{fuzz}’, ‘{tmin}’, or ‘{cmin}’."#,
                c = args[0],
                fuzz = COMMAND_FUZZ,
                tmin = COMMAND_MINIFY_INPUT,
                cmin = COMMAND_MINIFY_CORPUS
            ));
        }

        let max_input_cplx: Option<f64> = matches
            .opt_str(MAX_INPUT_CPLX_FLAG)
            .and_then(|x| x.parse::<usize>().ok())
            .map(|x| x as f64);

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

        let timeout: Option<usize> = matches.opt_str(TIMEOUT_FLAG).and_then(|x| x.parse::<usize>().ok());

        let socket_address = matches.opt_str(SOCK_ADDR_FLAG).and_then(|x| {
            if let Ok(mut addrs) = x.to_socket_addrs() {
                addrs.next()
            } else {
                None
            }
        });

        let input_file: Option<PathBuf> = matches.opt_str(INPUT_FILE_FLAG).and_then(|x| x.parse::<PathBuf>().ok());
        let corpus_size: Option<usize> = matches.opt_str(CORPUS_SIZE_FLAG).and_then(|x| x.parse::<usize>().ok());

        // verify all the right options are here

        let command = match args[0] {
            COMMAND_FUZZ => FuzzerCommand::Fuzz,
            COMMAND_READ => {
                let input_file =
                    input_file.expect(&format!("An input file must be provided when reading a test case. Use --{}", INPUT_FILE_FLAG));
                FuzzerCommand::Read { input_file }
            }
            COMMAND_MINIFY_INPUT => {
                let input_file =
                    input_file.expect(&format!("An input file must be provided when minifying a test case. Use --{}", INPUT_FILE_FLAG));
                FuzzerCommand::MinifyInput { input_file }
            }
            COMMAND_MINIFY_CORPUS => {
                let corpus_size =
                    corpus_size.expect(&format!("A corpus size must be provided when minifying a corpus. Use --{}", CORPUS_SIZE_FLAG));
                if corpus_in.is_none() {
                    panic!("An input corpus must be provided when minifying a corpus. Use --{}", IN_CORPUS_FLAG)
                }
                FuzzerCommand::MinifyCorpus { corpus_size }
            }
            _ => unreachable!(),
        };

        // use defaults
        let defaults = DefaultArguments::default();

        let max_input_cplx: f64 = max_input_cplx.unwrap_or(defaults.max_input_cplx as f64);

        let corpus_in: Option<PathBuf> = if no_in_corpus.is_some() {
            None
        } else {
            corpus_in.clone()
        };
        let corpus_out: Option<PathBuf> = if no_out_corpus.is_some() {
            None
        } else {
            corpus_out.clone()
        };

        let artifacts_folder: Option<PathBuf> = if no_artifacts.is_some() {
            None
        } else {
            artifacts_folder.clone()
        };

        let timeout: usize = timeout.unwrap_or(defaults.timeout);

        let socket_address = socket_address;

        Ok(Arguments {
            command,
            max_input_cplx,
            timeout,
            corpus_in,
            corpus_out,
            artifacts_folder,
            socket_address,
        })
    }
}
