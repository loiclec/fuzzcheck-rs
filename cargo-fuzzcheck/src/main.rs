use fuzzcheck_arg_parser::*;

use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::string::String;

use std::fs;
use std::io::Read;
use std::io::Write;

pub const COMMAND_INIT: &str = "init";
pub const COMMAND_RUN: &str = "run";

#[macro_use]
extern crate error_chain;

#[macro_use]
mod templates;

error_chain! {
    foreign_links {
        Toml(toml::de::Error);
        Io(::std::io::Error);
    }
}

const FUZZCHECK_PATH: &str = "https://github.com/loiclec/fuzzcheck-rs";
// static FUZZCHECK_REVISION: &str = "bf7948bb2b1f911197ca66af094ac20021fdd7f9";

#[cfg(target_os = "macos")]
pub fn default_target() -> &'static str {
    "x86_64-apple-darwin"
}

#[cfg(not(target_os = "macos"))]
pub fn default_target() -> &'static str {
    "x86_64-unknown-linux-gnu"
}
fn main() {
    let target_triple = default_target();

    let parser = options_parser();

    let env_args: Vec<String> = std::env::args().collect();

    let mut help = format!(
        r#"
USAGE:
    fuzzcheck {init}
    => Initialize the fuzz folder

    fuzzcheck {run} <TARGET> <SUBCOMMAND> [OPTIONS]
    => Execute the subcommand on the given fuzz target.
       The target name is the name of its folder in fuzz/fuzz_targets/.

SUBCOMMANDS:
    {fuzz}    Run the fuzz test
    {tmin}    Minify a crashing test input, requires --{input_file}
    {cmin}    Minify a corpus of test inputs, requires --{in_corpus}
"#,
        init = COMMAND_INIT,
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
        corpus_size = CORPUS_SIZE_FLAG
    )
    .as_str();

    if env_args.len() <= 1 {
        println!("{}", help);
        return;
    }

    let start_idx = if env_args[1] == "fuzzcheck" { 2 } else { 1 };

    if env_args.len() <= start_idx {
        println!("{}", help);
        return;
    }

    if env_args[start_idx] == COMMAND_INIT {
        let result = init_command();
        println!("{:#?}", result);
        return;
    }

    if env_args[start_idx] != COMMAND_RUN {
        println!("Invalid command: {}", env_args[1]);
        println!();
        println!("{}", help);
        return;
    } else if env_args.len() <= start_idx + 1 {
        println!("No fuzz target was given.");
        println!();
        println!("{}", help);
        return;
    }

    let target = &env_args[start_idx + 1];

    let mut defaults = DEFAULT_ARGUMENTS.clone();
    let defaults_in_corpus = format!("fuzz/fuzz_targets/{}/", target) + defaults.in_corpus;
    let defaults_out_corpus = format!("fuzz/fuzz_targets/{}/", target) + defaults.out_corpus;
    let defaults_artifacts = format!("fuzz/fuzz_targets/{}/", target) + defaults.artifacts;
    defaults.in_corpus = &defaults_in_corpus;
    defaults.out_corpus = &defaults_out_corpus;
    defaults.artifacts = &defaults_artifacts;

    let args = match CommandLineArguments::from_parser(&parser, &env_args[start_idx + 2..], defaults) {
        Ok(r) => r,
        Err(e) => {
            println!("{}", e);
            println!();
            println!("{}", help);
            return;
        }
    };
    let r = match args.command {
        FuzzerCommand::Fuzz => exec_normal_command(args, &target, target_triple),
        FuzzerCommand::MinifyInput => exec_input_minify_command(args, &target, target_triple),
        FuzzerCommand::Read => {
            panic!("unimplemented");
        }
        FuzzerCommand::MinifyCorpus => exec_normal_command(args, &target, target_triple),
    };
    if let Err(e) = r {
        println!("{}", e);
    }
}

fn root_package_name(root_folder: &PathBuf) -> Result<String> {
    let filename = root_folder.join("Cargo.toml");
    let mut file = fs::File::open(&filename)?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;
    let value: toml::Value = toml::from_slice(&data)?;
    let name = value
        .as_table()
        .and_then(|v| v.get("package"))
        .and_then(toml::Value::as_table)
        .and_then(|v| v.get("name"))
        .and_then(toml::Value::as_str);
    if let Some(name) = name {
        Ok(String::from(name))
    } else {
        Err(format!("{:?} (package.name) is malformed", filename).into())
    }
}

/// Add a new fuzz target script with a given name
fn create_target_template(
    root_package_name: &str,
    fuzz_folder: &PathBuf,
    fuzz_targets_folder: &PathBuf,
    target: &str,
) -> Result<()> {
    let mut target_path = fuzz_targets_folder.clone();
    target_path.push(target);
    let fuzzer_output_folder = target_path.clone();
    target_path.set_extension("rs");

    let mut script = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&target_path)
        .chain_err(|| format!("could not create target script file at {:?}", target_path))?;

    script.write_fmt(target_template!(root_package_name.replace("-", "_")))?;

    fs::create_dir(fuzzer_output_folder)?;

    let mut cargo_toml_path = fuzz_folder.clone();
    cargo_toml_path.push("Cargo.toml");

    let mut cargo = fs::OpenOptions::new().append(true).open(cargo_toml_path)?;

    Ok(cargo.write_fmt(toml_bin_template!(target))?)
}

fn init_command() -> Result<()> {
    let target = "target1";

    let root_folder = std::env::current_dir()?;
    let root_package_name = root_package_name(&root_folder)?;

    let fuzz_folder = root_folder.join("fuzz");
    let fuzz_targets_folder = fuzz_folder.join("fuzz_targets");

    // TODO: check if the project is already initialized
    fs::create_dir(&fuzz_folder)?;
    fs::create_dir(&fuzz_targets_folder)?;

    clone_and_compile_fuzzcheck_library(&fuzz_folder);

    let mut cargo = fs::File::create(fuzz_folder.join("Cargo.toml"))?;
    cargo.write_fmt(toml_template!(root_package_name))?;

    let mut ignore = fs::File::create(fuzz_folder.join(".gitignore"))?;
    ignore.write_fmt(gitignore_template!())?;

    create_target_template(&root_package_name, &fuzz_folder, &fuzz_targets_folder, target)
        .chain_err(|| format!("could not create template file for target {:?}", target))?;

    Ok(())
}

// fn collect_targets(manifest: &toml::Value) -> Vec<String> {
//     let bins = manifest
//         .as_table()
//         .and_then(|v| v.get("bin"))
//         .and_then(toml::Value::as_array);
//     if let Some(bins) = bins {
//         bins.iter()
//             .map(|bin| bin.as_table().and_then(|v| v.get("name")).and_then(toml::Value::as_str))
//             .filter_map(|name| name.map(String::from))
//             .collect()
//     } else {
//         Vec::new()
//     }
// }

fn clone_and_compile_fuzzcheck_library(fuzz_folder: &PathBuf) {
    Command::new("git")
        .current_dir(fuzz_folder)
        .args(vec!["clone", FUZZCHECK_PATH])
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .output()
        .expect("failed to execute process");

    // Command::new("git")
    //     .current_dir("fuzzcheck-rs")
    //     .args(vec!["checkout", FUZZCHECK_REVISION])
    //     .stdout(std::process::Stdio::inherit())
    //     .stderr(std::process::Stdio::inherit())
    //     .output()
    //     .expect("failed to execute process");

    let mut fuzzcheck_clone_folder = fuzz_folder.clone();
    fuzzcheck_clone_folder.push("fuzzcheck-rs");

    Command::new("cargo")
        .current_dir(fuzzcheck_clone_folder)
        .args(vec!["build", "--package", "fuzzcheck", "--release"])
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .output()
        .expect("failed to execute process");
}

fn exec_normal_command(arguments: CommandLineArguments, target: &str, target_triple: &str) -> Result<()> {
    let root_folder = std::env::current_dir()?;
    //let root_package_name = root_package_name(&root_folder)?;

    let fuzz_folder = root_folder.join("fuzz");
    let fuzz_targets_folder = fuzz_folder.join("fuzz_targets");
    let target_folder = fuzz_targets_folder.join(target);

    run_command(&arguments, &fuzz_folder, &target_folder, target_triple)?;

    Ok(())
}

// TODO: rename CommandLineArguments
fn exec_input_minify_command(mut arguments: CommandLineArguments, target: &str, target_triple: &str) -> Result<()> {
    let root_folder = std::env::current_dir()?;
    let fuzz_folder = &root_folder.join("fuzz");
    let fuzz_targets_folder = fuzz_folder.join("fuzz_targets");
    let target_folder = &fuzz_targets_folder.join(target);

    let file_to_minify = (&arguments.input_file).as_ref().unwrap().clone();

    let artifacts_folder = {
        let mut x = file_to_minify.parent().unwrap().to_path_buf();
        x.push(file_to_minify.file_stem().unwrap());
        x = x.with_extension("minified");
        x
    };
    let _ = std::fs::create_dir(&artifacts_folder);
    arguments.artifacts_folder = Some(artifacts_folder.clone());

    fn simplest_input_file(folder: &Path) -> Option<PathBuf> {
        let files_with_complexity = std::fs::read_dir(folder)
            .ok()?
            .filter_map(|path| -> Option<(PathBuf, f64)> {
                let path = path.ok()?.path();
                let name_components: Vec<&str> = path.file_stem()?.to_str()?.splitn(2, "--").collect();
                if name_components.len() == 2 {
                    let cplx = name_components[0].parse::<f64>().ok()?;
                    Some((path.to_path_buf(), cplx))
                } else {
                    None
                }
            });

        files_with_complexity
            .min_by(|x, y| std::cmp::PartialOrd::partial_cmp(&x.1, &y.1).unwrap_or(Ordering::Equal))
            .map(|x| x.0)
    }

    if let Some(simplest) = simplest_input_file(&artifacts_folder.as_path()) {
        arguments.input_file = Some(simplest);
    }
    arguments.command = FuzzerCommand::Read;

    let o = run_command(&arguments, fuzz_folder, target_folder, target_triple)?;
    assert!(!o.status.success());

    // hjhjb.minifyd/hshs.parent() != hjhjb.minifyd/ -> copy hshs to hjhjb.minifyd/hshs
    //let destination = artifacts_folder.join(arguments.input_file.file_name());
    // if arguments.input_file.unwrap().parent() != Some(artifacts_folder.as_path()) {
    //     std::fs::copy(arguments.input_file, artifacts_folder.to_owned() + arguments.input_file);
    // }

    arguments.command = FuzzerCommand::MinifyInput;

    loop {
        arguments.input_file = simplest_input_file(&artifacts_folder).or(arguments.input_file);

        run_command(&arguments, fuzz_folder, target_folder, target_triple)?;
    }
}

fn path_str(p: PathBuf) -> String {
    p.as_path().to_str().unwrap().to_owned()
}

// fn prepended_with_if_relative_to_target_folder(path: PathBuf, to_prepend: &PathBuf) -> String {
//     if path.is_relative() && !(path.starts_with("fuzz/fuzz_targets")) {
//         let mut new_path = to_prepend.clone();
//         new_path.push(path);
//         new_path
//     } else {
//         path
//     }
//     .as_path()
//     .to_str()
//     .unwrap()
//     .to_owned()
// }

fn run_command(
    args: &CommandLineArguments,
    fuzz_folder: &PathBuf,
    target_folder: &PathBuf,
    target_triple: &str,
) -> Result<std::process::Output> {
    let mut s: Vec<String> = Vec::new();

    let input_file_args = args.input_file.clone().map(|f| {
        vec![
            "--".to_owned() + INPUT_FILE_FLAG,
            path_str(f),
        ]
    });

    let corpus_in_args = args
        .corpus_in
        .clone()
        .map(|f| {
            vec![
                "--".to_owned() + IN_CORPUS_FLAG,
                path_str(f),
            ]
        })
        .unwrap_or_else(|| vec!["--".to_owned() + NO_IN_CORPUS_FLAG]);

    let corpus_out_args = args
        .corpus_out
        .clone()
        .map(|f| {
            vec![
                "--".to_owned() + OUT_CORPUS_FLAG,
                path_str(f),
            ]
        })
        .unwrap_or_else(|| vec!["--".to_owned() + NO_OUT_CORPUS_FLAG]);

    let artifacts_args = args
        .artifacts_folder
        .clone()
        .map(|f| {
            vec![
                "--".to_owned() + ARTIFACTS_FLAG,
                path_str(f),
            ]
        })
        .unwrap_or_else(|| vec!["--".to_owned() + NO_ARTIFACTS_FLAG]);

    match args.command {
        FuzzerCommand::Read => s.push(COMMAND_READ.to_owned()),
        FuzzerCommand::MinifyInput => s.push(COMMAND_MINIFY_INPUT.to_owned()),
        FuzzerCommand::MinifyCorpus => s.push(COMMAND_MINIFY_CORPUS.to_owned()),
        FuzzerCommand::Fuzz => s.push(COMMAND_FUZZ.to_owned()),
    };

    if let Some(input_file_args) = input_file_args {
        s.append(&mut input_file_args.clone());
    }
    s.append(&mut vec![
        "--".to_owned() + CORPUS_SIZE_FLAG,
        args.corpus_size.to_string(),
    ]);

    s.append(&mut corpus_in_args.clone());
    s.append(&mut corpus_out_args.clone());
    s.append(&mut artifacts_args.clone());
    s.append(&mut vec![
        "--".to_owned() + MAX_INPUT_CPLX_FLAG,
        args.max_input_cplx.to_string(),
    ]);

    s.append(&mut vec![
        "--".to_owned() + MUT_DEPTH_FLAG,
        args.mutate_depth.to_string(),
    ]);
    s.append(&mut vec![
        "--".to_owned() + MAX_NBR_RUNS_FLAG,
        args.max_nbr_of_runs.to_string(),
    ]);

    let fuzzcheck_lib = fuzz_folder.join("fuzzcheck-rs/target/release/deps");

    let rustflags: String = format!(
        "--cfg fuzzing \
         -Cpasses=sancov \
         -Cllvm-args=-sanitizer-coverage-level=4 \
         -Cllvm-args=-sanitizer-coverage-trace-pc-guard \
         -Cllvm-args=-sanitizer-coverage-trace-compares \
         -Cllvm-args=-sanitizer-coverage-trace-divs \
         -Cllvm-args=-sanitizer-coverage-trace-geps \
         -Cllvm-args=-sanitizer-coverage-prune-blocks=0 \
         -L {}",
        fuzzcheck_lib.display()
    );

    Command::new("cargo")
        .env("RUSTFLAGS", rustflags)
        .arg("run")
        .arg("--manifest-path")
        .arg(fuzz_folder.join("Cargo.toml"))
        .arg("--bin")
        .arg(target_folder.file_name().unwrap())
        .arg("--release")
        .arg("--target")
        .arg(target_triple)
        .arg("--")
        .args(s)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .output()
        .map_err(|x| x.into())
}
