extern crate cargo_fuzzcheck;
use cargo_fuzzcheck::project;
use cargo_fuzzcheck::*;

use fuzzcheck_arg_parser::*;

use std::string::String;

use std::env;

pub const COMMAND_INIT: &str = "init";
pub const COMMAND_RUN: &str = "run";
pub const COMMAND_CLEAN: &str = "clean";

fn main() {
    if let Err(e) = _main() {
        println!("{}", e);
    }
}

fn _main() -> Result<(), CargoFuzzcheckError> {
    let parser = options_parser();

    let env_args: Vec<String> = std::env::args().collect();

    let mut help = format!(
        r#"
USAGE:
    fuzzcheck {init} <optional path to fuzzcheck-rs git repo>
    => Initialize the fuzz folder

    fuzzcheck {clean}
    => Clean all build artifacts

    fuzzcheck {run} <TARGET> <SUBCOMMAND> [OPTIONS]
    => Execute the subcommand on the given fuzz target.
       The target name is the name of its folder in fuzz/fuzz_targets/.

SUBCOMMANDS:
    {fuzz}    Run the fuzz test
    {tmin}    Minify a crashing test input, requires --{input_file}
    {cmin}    Minify a corpus of test inputs, requires --{in_corpus}
"#,
        init = COMMAND_INIT,
        clean = COMMAND_CLEAN,
        run = COMMAND_RUN,
        fuzz = COMMAND_FUZZ,
        tmin = COMMAND_MINIFY_INPUT,
        input_file = INPUT_FILE_FLAG,
        cmin = COMMAND_MINIFY_CORPUS,
        in_corpus = IN_CORPUS_FLAG,
    );
    help += parser.usage("").as_str();
    help += format!(
        r#"

## Examples:

cargo-fuzzcheck {init}

cargo-fuzzcheck {run} target1 {fuzz}
    Launch the fuzzer on “target1” with default options.

cargo-fuzzcheck {run} target2 fuzz --{max_cplx} 4000 --{out_corpus} fuzz_results/out/
    Fuzz “target2”, generating inputs of complexity no greater than 4000, 
    and write the output corpus (i.e. the folder of most interesting test cases) 
    to fuzz_results/out/.

cargo-fuzzcheck {run} target1 {tmin} --{input_file} "artifacts/crash.json"

    Using “target1”, minify the test input defined in the file 
    "artifacts/crash.json". It will put minified inputs in the folder 
    artifacts/crash.minified/ and name them {{complexity}}-{{hash}}.json. 
    For example, artifacts/crash.minified/4213--8cd7777109b57b8c.json
    is a minified input of complexity 42.13.

cargo-fuzzcheck {run} target1 {cmin} --{in_corpus} "fuzz-corpus" --{corpus_size} 25

    Using “target1”, minify the corpus defined by the folder "fuzz-corpus",
    which should contain JSON-encoded test inputs.
    It will remove files from that folder until only the 25 most important
    test inputs remain.
"#,
        init = COMMAND_INIT,
        run = COMMAND_RUN,
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
        return Ok(()); // TODO: change that
    }

    let start_idx = if env_args[1] == "fuzzcheck" { 2 } else { 1 };

    if env_args.len() <= start_idx {
        println!("{}", help);
        return Ok(()); // TODO: change that
    }

    let root_path = &std::env::current_dir()?;

    match env_args[start_idx].as_str() {
        COMMAND_INIT => {
            let fuzzcheck_path = if env_args.len() > (start_idx + 1) {
                env_args[start_idx + 1].as_str().trim()
            } else {
                env!("CARGO_PKG_VERSION")
            };
            let root = project::NonInitializedRoot::from_path(root_path)?;
            let result = root.init_command(fuzzcheck_path);
            println!("{:#?}", result);
            Ok(())
        }
        COMMAND_CLEAN => {
            let root = project::Root::from_path(root_path)?;
            let result = root.clean_command();
            println!("{:#?}", result);
            Ok(())
        }
        COMMAND_RUN => {
            if env_args.len() <= start_idx + 1 {
                println!("No fuzz target was given.");
                println!();
                println!("{}", help);
                return Ok(()); // TODO: change that
            }
            let root = project::Root::from_path(root_path)?;

            let target_name = &env_args[start_idx + 1];

            let args = match CommandLineArguments::from_parser(&parser, &env_args[start_idx + 2..] /*, defaults*/) {
                Ok(r) => r,
                Err(e) => {
                    println!("{}", e);
                    println!();
                    println!("{}", help);
                    return Ok(()); // TODO: change that
                }
            };
            let r = match args.command {
                FuzzerCommand::Fuzz => root.run_command(&args, target_name).map(|_| ()),
                FuzzerCommand::MinifyInput => root.input_minify_command(&args, target_name),
                FuzzerCommand::Read => {
                    panic!("unimplemented");
                }
                FuzzerCommand::MinifyCorpus => root.launch_executable(&args, target_name),
            };
            if let Err(e) = r {
                println!("{}", e);
            }
            Ok(())
        }
        _ => {
            println!("Invalid command: {}", env_args[start_idx]);
            println!();
            println!("{}", help);
            Ok(())
        }
    }
}
