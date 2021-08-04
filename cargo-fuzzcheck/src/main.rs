extern crate cargo_fuzzcheck;
use cargo_fuzzcheck::*;
use fuzzcheck_common::arg::*;
use std::{process, string::String};

fn main() {
    let parser = options_parser();

    let env_args: Vec<String> = std::env::args().collect();

    let mut help = format!(
        r#"
USAGE:
    cargo-fuzzcheck <FUZZ_TEST> <SUBCOMMAND> [OPTIONS]
    => Execute the subcommand on the given fuzz test.

SUBCOMMANDS:
    {fuzz}    Run the fuzz test
    {tmin}    Minify a crashing test input, requires --{input_file}
    {cmin}    Minify a corpus of test inputs, requires --{in_corpus} and --{corpus_size}
"#,
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

## Examples:

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
    )
    .as_str();

    if env_args.len() <= 1 {
        println!("{}", help);
        return; // TODO: change that
    }

    let start_idx = if env_args[1] == "fuzzcheck" { 2 } else { 1 };

    if env_args.len() <= start_idx {
        println!("{}", help);
        return;
    }

    let target_name = &env_args[start_idx];

    let args = env_args[start_idx + 1..]
        .into_iter()
        .map(|s| s.as_str())
        .collect::<Vec<_>>();

    let args = match Arguments::from_parser(&parser, &args) {
        Ok(r) => r,
        Err(e) => {
            println!("{}", e);
            println!();
            println!("{}", help);
            return;
        }
    };

    let r = match args.command {
        FuzzerCommand::Fuzz => {
            let exec =
                launch_executable(target_name, &args, &process::Stdio::inherit).expect("failed to launch fuzz target");
            exec.wait_with_output().expect("failed to wait on fuzz test process");
            return;
        }
        FuzzerCommand::MinifyInput { .. } => input_minify_command(target_name, &args, &process::Stdio::inherit),
        FuzzerCommand::Read { .. } => {
            todo!();
        }
        FuzzerCommand::MinifyCorpus { .. } => launch_executable(target_name, &args, process::Stdio::inherit)
            .and_then(|child| child.wait_with_output().map_err(|err| err.into()))
            .map(|_| ()),
    };
    if let Err(e) = r {
        println!("{}", e);
    }
}
