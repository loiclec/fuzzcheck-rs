use std::error::Error;
use std::fmt::{Debug, Display};
use std::path::PathBuf;
use std::time::Duration;

use getopts::{Fail, Matches, Options};

pub const MAX_INPUT_CPLX_FLAG: &str = "max-cplx";
pub const INPUT_FILE_FLAG: &str = "input-file";
pub const IN_CORPUS_FLAG: &str = "in-corpus";
pub const NO_IN_CORPUS_FLAG: &str = "no-in-corpus";
pub const OUT_CORPUS_FLAG: &str = "out-corpus";
pub const NO_OUT_CORPUS_FLAG: &str = "no-out-corpus";
pub const ARTIFACTS_FLAG: &str = "artifacts";
pub const NO_ARTIFACTS_FLAG: &str = "no-artifacts";
pub const STATS_FLAG: &str = "stats";
pub const NO_STATS_FLAG: &str = "no-stats";
pub const COMMAND_FLAG: &str = "command";

pub const MAX_DURATION_FLAG: &str = "stop-after-duration";
pub const MAX_ITERATIONS_FLAG: &str = "stop-after-iterations";
pub const STOP_AFTER_FIRST_FAILURE_FLAG: &str = "stop-after-first-failure";

pub const DETECT_INFINITE_LOOP_FLAG: &str = "detect-infinite-loop";

pub const COMMAND_FUZZ: &str = "fuzz";
pub const COMMAND_MINIFY_INPUT: &str = "minify";
pub const COMMAND_READ: &str = "read";

#[derive(Clone)]
pub struct DefaultArguments {
    pub max_input_cplx: f64,
}
impl Default for DefaultArguments {
    #[coverage(off)]
    fn default() -> Self {
        Self { max_input_cplx: 4096.0 }
    }
}

/// The task that the fuzzer is asked to perform.
#[derive(Debug, Clone)]
pub enum FuzzerCommand {
    Fuzz,
    Read { input_file: PathBuf },
    MinifyInput { input_file: PathBuf },
}
impl Default for FuzzerCommand {
    fn default() -> Self {
        Self::Fuzz
    }
}

/// Various arguments given to the fuzzer, typically provided by the `cargo fuzzcheck` command line tool.
#[derive(Debug, Clone)]
pub struct Arguments {
    pub command: FuzzerCommand,
    pub max_input_cplx: f64,
    pub detect_infinite_loop: bool,
    pub maximum_duration: Duration,
    pub maximum_iterations: usize,
    pub stop_after_first_failure: bool,
    pub corpus_in: Option<PathBuf>,
    pub corpus_out: Option<PathBuf>,
    pub artifacts_folder: Option<PathBuf>,
    pub stats_folder: Option<PathBuf>,
}
impl Arguments {
    pub fn for_internal_documentation_test() -> Self {
        Self {
            command: FuzzerCommand::Fuzz,
            max_input_cplx: 256.,
            detect_infinite_loop: false,
            maximum_duration: Duration::MAX,
            maximum_iterations: usize::MAX,
            stop_after_first_failure: true,
            corpus_in: None,
            corpus_out: None,
            artifacts_folder: None,
            stats_folder: None,
        }
    }
}

/// The command line argument parser used by the fuzz target and `cargo fuzzcheck`
#[must_use]
#[coverage(off)]
pub fn options_parser() -> Options {
    let mut options = Options::new();

    let defaults = DefaultArguments::default();
    options.optopt(
        "",
        COMMAND_FLAG,
        &format!(
            "the action to be performed (default: fuzz). --{} is required when using `{}`",
            INPUT_FILE_FLAG, COMMAND_MINIFY_INPUT
        ),
        &format!("<{} | {}>", COMMAND_FUZZ, COMMAND_MINIFY_INPUT),
    );
    options.optopt(
        "",
        MAX_DURATION_FLAG,
        "maximum duration of the fuzz test, in seconds",
        "N",
    );
    options.optopt("", MAX_ITERATIONS_FLAG, "maximum number of iterations", "N");

    options.optflag(
        "",
        DETECT_INFINITE_LOOP_FLAG,
        "fail on tests running for more than one second",
    );

    options.optflag(
        "",
        STOP_AFTER_FIRST_FAILURE_FLAG,
        "stop the fuzzer after the first test failure is found",
    );

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
    options.optopt("", STATS_FLAG, "folder where the statistics will be written", "PATH");
    options.optflag(
        "",
        NO_STATS_FLAG,
        format!("do not save statistics, overrides --{stats}", stats = STATS_FLAG).as_str(),
    );
    options.optopt("", INPUT_FILE_FLAG, "file containing a test case", "PATH");
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
    options.optflag("h", "help", "print this help menu");

    options
}

impl Arguments {
    /// Create an `Arguments` from the parsed result of [`options_parser()`].
    ///
    /// ### Arguments
    /// * `for_cargo_fuzzcheck` : true if this method is called within `cargo fuzzcheck`, false otherwise.
    ///   This is because `cargo fuzzcheck` also needs a fuzz target as argument, while the fuzzed binary
    ///   does not.
    #[coverage(off)]
    pub fn from_matches(matches: &Matches, for_cargo_fuzzcheck: bool) -> Result<Self, ArgumentsError> {
        if matches.opt_present("help") || matches.free.contains(&"help".to_owned()) {
            return Err(ArgumentsError::WantsHelp);
        }

        if for_cargo_fuzzcheck && matches.free.is_empty() {
            return Err(ArgumentsError::Validation(
                "A fuzz target must be given to cargo fuzzcheck.".to_string(),
            ));
        }

        let command = matches.opt_str(COMMAND_FLAG).unwrap_or_else(
            #[coverage(off)]
            || COMMAND_FUZZ.to_owned(),
        );

        let command = command.as_str();

        if !matches!(command, COMMAND_FUZZ | COMMAND_READ | COMMAND_MINIFY_INPUT) {
            return Err(ArgumentsError::Validation(format!(
                r#"The command {c} is not supported. It can either be ‘{fuzz}’ or ‘{minify}’."#,
                c = &matches.free[0],
                fuzz = COMMAND_FUZZ,
                minify = COMMAND_MINIFY_INPUT,
            )));
        }

        let max_input_cplx: Option<f64> = matches
            .opt_str(MAX_INPUT_CPLX_FLAG)
            .and_then(
                #[coverage(off)]
                |x| x.parse::<usize>().ok(),
            )
            .map(
                #[coverage(off)]
                |x| x as f64,
            );

        let detect_infinite_loop = matches.opt_present(DETECT_INFINITE_LOOP_FLAG);

        let corpus_in: Option<PathBuf> = matches.opt_str(IN_CORPUS_FLAG).and_then(
            #[coverage(off)]
            |x| x.parse::<PathBuf>().ok(),
        );

        let no_in_corpus = if matches.opt_present(NO_IN_CORPUS_FLAG) {
            Some(())
        } else {
            None
        };

        let corpus_out: Option<PathBuf> = matches.opt_str(OUT_CORPUS_FLAG).and_then(
            #[coverage(off)]
            |x| x.parse::<PathBuf>().ok(),
        );

        let no_out_corpus = if matches.opt_present(NO_OUT_CORPUS_FLAG) {
            Some(())
        } else {
            None
        };

        let artifacts_folder: Option<PathBuf> = matches.opt_str(ARTIFACTS_FLAG).and_then(
            #[coverage(off)]
            |x| x.parse::<PathBuf>().ok(),
        );

        let no_artifacts = if matches.opt_present(NO_ARTIFACTS_FLAG) {
            Some(())
        } else {
            None
        };

        let stats_folder: Option<PathBuf> = matches.opt_str(STATS_FLAG).and_then(
            #[coverage(off)]
            |x| x.parse::<PathBuf>().ok(),
        );

        let no_stats = if matches.opt_present(NO_STATS_FLAG) {
            Some(())
        } else {
            None
        };

        let input_file: Option<PathBuf> = matches.opt_str(INPUT_FILE_FLAG).and_then(
            #[coverage(off)]
            |x| x.parse::<PathBuf>().ok(),
        );

        // verify all the right options are here

        let command = match command {
            COMMAND_FUZZ => FuzzerCommand::Fuzz,
            COMMAND_READ => {
                let input_file = input_file.unwrap_or_else(
                    #[coverage(off)]
                    || {
                        panic!(
                            "An input file must be provided when reading a test case. Use --{}",
                            INPUT_FILE_FLAG
                        )
                    },
                );
                FuzzerCommand::Read { input_file }
            }
            COMMAND_MINIFY_INPUT => {
                let input_file = input_file.unwrap_or_else(
                    #[coverage(off)]
                    || {
                        panic!(
                            "An input file must be provided when minifying a test case. Use --{}",
                            INPUT_FILE_FLAG
                        )
                    },
                );
                FuzzerCommand::MinifyInput { input_file }
            }
            _ => unreachable!(),
        };

        let maximum_duration = {
            let seconds = matches
                .opt_str(MAX_DURATION_FLAG)
                .and_then(
                    #[coverage(off)]
                    |x| x.parse::<u64>().ok(),
                )
                .unwrap_or(u64::MAX);
            Duration::new(seconds, 0)
        };
        let maximum_iterations = matches
            .opt_str(MAX_ITERATIONS_FLAG)
            .and_then(
                #[coverage(off)]
                |x| x.parse::<usize>().ok(),
            )
            .unwrap_or(usize::MAX);
        let stop_after_first_failure = matches.opt_present(STOP_AFTER_FIRST_FAILURE_FLAG);

        let defaults = DefaultArguments::default();
        let max_input_cplx: f64 = max_input_cplx.unwrap_or(defaults.max_input_cplx as f64);
        let corpus_in: Option<PathBuf> = if no_in_corpus.is_some() { None } else { corpus_in };
        let corpus_out: Option<PathBuf> = if no_out_corpus.is_some() { None } else { corpus_out };

        let artifacts_folder: Option<PathBuf> = if no_artifacts.is_some() { None } else { artifacts_folder };
        let stats_folder: Option<PathBuf> = if no_stats.is_some() { None } else { stats_folder };

        Ok(Arguments {
            command,
            detect_infinite_loop,
            maximum_duration,
            maximum_iterations,
            stop_after_first_failure,
            max_input_cplx,
            corpus_in,
            corpus_out,
            artifacts_folder,
            stats_folder,
        })
    }
}

/// The “help” output of cargo-fuzzcheck
#[coverage(off)]
pub fn help(parser: &Options) -> String {
    let mut help = r##"
USAGE:
    cargo-fuzzcheck <FUZZ_TEST> [OPTIONS]

FUZZ_TEST:
    The fuzz test is the exact path to the #[test] function that launches
    fuzzcheck. For example, it can be "parser::tests::fuzz_test_1" if you have 
    the following snippet located at src/parser/mod.rs:

    #[cfg(test)]
    mod tests {{
        #[test]
        fn fuzz_test_1() {{
            fuzzcheck::fuzz_test(some_function_to_test)
                .default_options()
                .launch();
        }}
    }}
"##
    .to_owned();
    help += parser.usage("").as_str();
    help += format!(
        r#"

EXAMPLES:

cargo-fuzzcheck tests::fuzz_test1
    Launch the fuzzer on "tests::fuzz_test1", located in the crate’s library, with default options.

cargo-fuzzcheck tests::fuzz_bin --bin my_program
    Launch the fuzzer on "tests::fuzz_bin", located in the "my_program" binary target, with default options.

cargo-fuzzcheck fuzz_test2 --test my_integration_test
    Launch the fuzzer on "fuzz_test2", located in the "my_integration_test" test target, with default options.

cargo-fuzzcheck tests::fuzzit --{max_cplx} 4000 --{out_corpus} fuzz_results/out/
    Fuzz "tests::fuzzit", generating inputs of complexity no greater than 4000, 
    and write the output corpus (i.e. the folder of most interesting test cases) 
    to fuzz_results/out/.

cargo-fuzzcheck tests::fuzz --command {minify} --{input_file} "artifacts/crash.json"
    Using the fuzz test located at "tests::fuzz_test", minify the test input defined 
    in the file "artifacts/crash.json". It will put minified inputs in the folder 
    artifacts/crash.minified/ and name them {{complexity}}-{{hash}}.json. 
    For example, artifacts/crash.minified/4213--8cd7777109b57b8c.json
    is a minified input of complexity 42.13.
"#,
        minify = COMMAND_MINIFY_INPUT,
        input_file = INPUT_FILE_FLAG,
        max_cplx = MAX_INPUT_CPLX_FLAG,
        out_corpus = OUT_CORPUS_FLAG,
    )
    .as_str();
    help
}

#[derive(Clone)]
pub enum ArgumentsError {
    NoArgumentsGiven(String),
    Parsing(Fail),
    Validation(String),
    WantsHelp,
}

impl Debug for ArgumentsError {
    #[coverage(off)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <Self as Display>::fmt(self, f)
    }
}
impl Display for ArgumentsError {
    #[coverage(off)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArgumentsError::NoArgumentsGiven(help) => {
                write!(f, "No arguments were given.\nHelp:\n{}", help)
            }
            ArgumentsError::Parsing(e) => {
                write!(
                    f,
                    "{}
To display the help, run: 
    cargo fuzzcheck --help",
                    e
                )
            }
            ArgumentsError::Validation(e) => {
                write!(
                    f,
                    "{} 
To display the help, run: 
    cargo fuzzcheck --help",
                    e
                )
            }
            ArgumentsError::WantsHelp => {
                write!(f, "Help requested.")
            }
        }
    }
}
impl Error for ArgumentsError {}

impl From<Fail> for ArgumentsError {
    #[coverage(off)]
    fn from(e: Fail) -> Self {
        Self::Parsing(e)
    }
}
