use getopts::{Fail, Options};
use std::{error::Error, fmt::{Debug, Display}, net::{SocketAddr, ToSocketAddrs}, path::PathBuf, time::Duration};

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

pub const MAX_DURATION_FLAG: &str = "max-duration";
pub const MAX_ITERATIONS_FLAG: &str = "max-iterations";
pub const STOP_AFTER_FIRST_FAILURE_FLAG: &str = "stop-after-first-failure";


pub const COMMAND_FUZZ: &str = "fuzz";
pub const COMMAND_MINIFY_INPUT: &str = "tmin";
pub const COMMAND_MINIFY_CORPUS: &str = "cmin";
pub const COMMAND_READ: &str = "read";


#[derive(Clone)]
pub struct DefaultArguments {
    pub max_input_cplx: f64,
}
impl Default for DefaultArguments {
    #[no_coverage]
    fn default() -> Self {
        Self {
            max_input_cplx: 4096.0,
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
    pub maximum_duration: Duration,
    pub maximum_iterations: usize,
    pub stop_after_first_failure: bool,
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
    options.optopt("", MAX_DURATION_FLAG, "maximum duration of the fuzz test, in seconds", "N");
    options.optopt("", MAX_ITERATIONS_FLAG, "maximum number of iterations", "N");
    options.optflag("", STOP_AFTER_FIRST_FAILURE_FLAG, "stop the fuzzer after the first test failure is found");

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
        SOCK_ADDR_FLAG,
        "address of the TCP socket for communication between cargo-fuzzcheck and the fuzz target",
        "127.0.0.1:0",
    );

    options.optflag("", "help", "print this help menu");

    options
}

impl Arguments {
    #[no_coverage]
    pub fn from_parser(options: &Options, args: &[&str]) -> Result<Self, ArgumentsError> {
        let matches = options.parse(args)?;

        if args.is_empty() {
            return Err(ArgumentsError::Validation("A command must be given to cargo fuzzcheck.".to_string()));
        }

        if matches.opt_present("help") {
            return Err(ArgumentsError::WantsHelp);
        }

        if !matches!(
            args[0],
            COMMAND_FUZZ | COMMAND_READ | COMMAND_MINIFY_INPUT | COMMAND_MINIFY_CORPUS
        ) {
            return Err(ArgumentsError::Validation(format!(
                r#"The command {c} is not supported. It can either be ‘{fuzz}’, ‘{tmin}’, or ‘{cmin}’."#,
                c = args[0],
                fuzz = COMMAND_FUZZ,
                tmin = COMMAND_MINIFY_INPUT,
                cmin = COMMAND_MINIFY_CORPUS
            )));
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
                let input_file = input_file.expect(&format!(
                    "An input file must be provided when reading a test case. Use --{}",
                    INPUT_FILE_FLAG
                ));
                FuzzerCommand::Read { input_file }
            }
            COMMAND_MINIFY_INPUT => {
                let input_file = input_file.expect(&format!(
                    "An input file must be provided when minifying a test case. Use --{}",
                    INPUT_FILE_FLAG
                ));
                FuzzerCommand::MinifyInput { input_file }
            }
            COMMAND_MINIFY_CORPUS => {
                let corpus_size = corpus_size.expect(&format!(
                    "A corpus size must be provided when minifying a corpus. Use --{}",
                    CORPUS_SIZE_FLAG
                ));
                if corpus_in.is_none() {
                    panic!(
                        "An input corpus must be provided when minifying a corpus. Use --{}",
                        IN_CORPUS_FLAG
                    )
                }
                FuzzerCommand::MinifyCorpus { corpus_size }
            }
            _ => unreachable!(),
        };

        let maximum_duration = {
            let seconds = matches.opt_str(MAX_DURATION_FLAG).and_then(|x| x.parse::<u64>().ok()).unwrap_or(u64::MAX);
            Duration::new(seconds, 0)
        };
        let maximum_iterations = matches.opt_str(MAX_ITERATIONS_FLAG).and_then(|x| x.parse::<usize>().ok()).unwrap_or(usize::MAX);
        let stop_after_first_failure = matches.opt_present(STOP_AFTER_FIRST_FAILURE_FLAG);

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

        let socket_address = socket_address;

        Ok(Arguments {
            command,
            maximum_duration,
            maximum_iterations,
            stop_after_first_failure,
            max_input_cplx,
            corpus_in,
            corpus_out,
            artifacts_folder,
            socket_address,
        })
    }
}


pub fn help(parser: &Options) -> String {

    let mut help = format!(
        r##"
USAGE:
    cargo-fuzzcheck <FUZZ_TEST> <SUBCOMMAND> [OPTIONS]
    => Execute the subcommand on the given fuzz test.

FUZZ_TEST:
    The fuzz test is the exact path to the #[test] function that launches
    fuzzcheck. For example, it can be "parser::tests::fuzz_test_1" if you have 
    the following snippet located at src/parser/mod.rs:

    #[cfg(test)]
    mod tests {{
        #[test]
        fn fuzz_test_1() {{
            fuzzcheck::test(some_function_to_test)
                .default_options()
                .launch();
        }}
    }}

SUBCOMMANDS:
    {fuzz}    Run the fuzz test
    {tmin}    Minify a crashing test input, requires --{input_file}
    {cmin}    Minify a corpus of test inputs, requires --{in_corpus} and --{corpus_size}
"##,
        fuzz = COMMAND_FUZZ,
        tmin = COMMAND_MINIFY_INPUT,
        input_file = INPUT_FILE_FLAG,
        cmin = COMMAND_MINIFY_CORPUS,
        in_corpus = IN_CORPUS_FLAG,
        corpus_size = CORPUS_SIZE_FLAG
    );
    help += parser.usage("").as_str();
    help += format!(
        r#"

EXAMPLES:

cargo-fuzzcheck target1 {fuzz}
    Launch the fuzzer on “target1” with default options.

cargo-fuzzcheck target2 fuzz --{max_cplx} 4000 --{out_corpus} fuzz_results/out/
    Fuzz “target2”, generating inputs of complexity no greater than 4000, 
    and write the output corpus (i.e. the folder of most interesting test cases) 
    to fuzz_results/out/.

cargo-fuzzcheck target1 {tmin} --{input_file} "artifacts/crash.json"
    Using “target1”, minify the test input defined in the file 
    "artifacts/crash.json". It will put minified inputs in the folder 
    artifacts/crash.minified/ and name them {{complexity}}-{{hash}}.json. 
    For example, artifacts/crash.minified/4213--8cd7777109b57b8c.json
    is a minified input of complexity 42.13.

cargo-fuzzcheck target1 {cmin} --{in_corpus} "fuzz-corpus" --{out_corpus} "minimized_corpus" --{corpus_size} 25
    Using “target1”, minify the corpus defined by the folder "fuzz-corpus",
    which should contain JSON-encoded test inputs. The 25 most important
    inputs from fuzz-corpus will be copied to minimized-corpus.
"#,
        fuzz = COMMAND_FUZZ,
        tmin = COMMAND_MINIFY_INPUT,
        input_file = INPUT_FILE_FLAG,
        cmin = COMMAND_MINIFY_CORPUS,
        in_corpus = IN_CORPUS_FLAG,
        corpus_size = CORPUS_SIZE_FLAG,
        max_cplx = MAX_INPUT_CPLX_FLAG,
        out_corpus = OUT_CORPUS_FLAG,
    ).as_str();
    help
}


pub enum ArgumentsError {
    NoArgumentsGiven(Options),
    Parsing(Fail),
    Validation(String),
    WantsHelp,
}

impl Debug for ArgumentsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <Self as Display>::fmt(&self, f)
    }
}
impl Display for ArgumentsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArgumentsError::NoArgumentsGiven(parser) => {
                write!(f, "No arguments were given.\nHelp:\n{}", help(parser))
            },
            ArgumentsError::Parsing(e) => {
                write!(f, "{}
To display the help, run: 
    cargo fuzzcheck --help", e)
            },
            ArgumentsError::Validation(e) => {
                write!(f, 
                "{} 
To display the help, run: 
    cargo fuzzcheck --help"
                , e)
            },
            ArgumentsError::WantsHelp => {
                write!(f, "Help requested.")
            },
        }
    }
}
impl Error for ArgumentsError { }

impl From<Fail> for ArgumentsError {
    fn from(e: Fail) -> Self {
        Self::Parsing(e)
    }
}