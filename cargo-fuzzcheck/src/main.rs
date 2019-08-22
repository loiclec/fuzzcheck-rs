use fuzzcheck_arg_parser::*;

use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::string::String;

use std::env;
use std::fs;
use std::io::Write;
use std::io::Read;

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

/// The default target to pass to cargo, to workaround issue #11.
#[cfg(target_os = "macos")]
pub fn default_target() -> &'static str {
    "x86_64-apple-darwin"
}

/// The default target to pass to cargo, to workaround issue #11.
#[cfg(not(target_os = "macos"))]
pub fn default_target() -> &'static str {
    "x86_64-unknown-linux-gnu"
}
fn main() {
    let target_triple = default_target();

    let parser = options_parser();

    let env_args: Vec<String> = std::env::args().collect();
    
    let mut help = format!(r#"
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
        init=COMMAND_INIT,
        run=COMMAND_RUN,
        fuzz=COMMAND_FUZZ,
        tmin=COMMAND_MINIFY_INPUT,
        input_file=INPUT_FILE_FLAG,
        cmin=COMMAND_MINIFY_CORPUS,
        in_corpus=IN_CORPUS_FLAG,
    );
    help += parser.usage("").as_str();
    help += format!(r#"

## Examples:

fuzzcheck {init}

fuzzcheck {run} target1 {fuzz}
    Launch the fuzzer on “target1” with default options.

fuzzcheck {run} target1 {tmin} --{input_file} "artifacts/crash.json"

    Using “target1”, minify the test input defined in the file 
    "artifacts/crash.json". It will put minified inputs in the folder 
    artifacts/crash.minified/ and name them {{complexity}}-{{hash}}.json. 
    For example, artifacts/crash.minified/4213--8cd7777109b57b8c.json
    is a minified input of complexity 42.13.

fuzzcheck {run} target1 {cmin} --{in_corpus} "fuzz-corpus" --{corpus_size} 25

    Using “target1”, minify the corpus defined by the folder "fuzz-corpus",
    which should contain JSON-encoded test inputs.
    It will remove files from that folder until only the 25 most important
    test inputs remain.
"#,
        init=COMMAND_INIT,
        run=COMMAND_RUN,
        fuzz=COMMAND_FUZZ,
        tmin=COMMAND_MINIFY_INPUT,
        input_file=INPUT_FILE_FLAG,
        cmin=COMMAND_MINIFY_CORPUS,
        in_corpus=IN_CORPUS_FLAG,
        corpus_size=CORPUS_SIZE_FLAG
    ).as_str();

    if env_args.len() <= 1 {
        println!("{}", help);
        return
    }

    if env_args[1] == COMMAND_INIT {
        let result = init_command();
        println!("{:#?}", result);
        return
    }
    
    if env_args[1] != COMMAND_RUN {
        println!("Invalid command: {}", env_args[1]);
        println!();
        println!("{}", help);
        return;
    } else if env_args.len() <= 2 {
        println!("No fuzz target was given.");
        println!();
        println!("{}", help);    
        return;
    }

    let args = match CommandLineArguments::from_parser(&parser, &env_args[3..]) {
        Ok(r) => r,
        Err(e) => {
            println!("{}", e);
            println!();
            println!("{}", help);
            return;
        }
    };
    match args.command {
        FuzzerCommand::Fuzz => fuzz_command(args, target_triple),
        FuzzerCommand::Minimize => minimize_command(args, target_triple),
        FuzzerCommand::Read => panic!("unimplemented"),
        FuzzerCommand::Shrink => shrink_command(args, target_triple),
    }
}


fn is_fuzz_manifest(value: &toml::Value) -> bool {
    let is_fuzz = value.as_table().and_then(|v| v.get("package"))
                                  .and_then(toml::Value::as_table)
                                  .and_then(|v| v.get("metadata"))
                                  .and_then(toml::Value::as_table)
                                  .and_then(|v| v.get("cargo-fuzz"))
                                  .and_then(toml::Value::as_bool);
    is_fuzz == Some(true)
}
/// Returns the path for the first found non-fuzz Cargo package
fn find_package() -> Result<PathBuf> {
    let mut dir = env::current_dir()?;
    let mut data = Vec::new();
    loop {
        let manifest_path = dir.join("Cargo.toml");
        match fs::File::open(&manifest_path) {
            Err(_) => {},
            Ok(mut f) => {
                f.read_to_end(&mut data)?;
                let value: toml::Value = toml::from_slice(&data)
                    .chain_err(||
                        format!("could not decode the manifest file at {:?}", manifest_path)
                    )?;
                if !is_fuzz_manifest(&value) {
                    // Not a cargo-fuzz project => must be a proper cargo project :)
                    return Ok(dir);
                }
            }
        }
        if !dir.pop() { break; }
    }
    Err("could not find a cargo project".into())
}

fn root_package_name(root_folder: &PathBuf) -> Result<String> {
    let filename = root_folder.join("Cargo.toml");
    let mut file = fs::File::open(&filename)?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;
    let value: toml::Value = toml::from_slice(&data)?;
    let name = value.as_table().and_then(|v| v.get("package"))
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
fn create_target_template(root_package_name: &str, fuzz_folder: &PathBuf, fuzzed_targets_folder: &PathBuf, target: &str) -> Result<()> {
    let mut target_path = fuzzed_targets_folder.clone();
    target_path.push(target);
    target_path.set_extension("rs");

    let mut script = fs::OpenOptions::new().write(true).create_new(true).open(&target_path)
        .chain_err(|| format!("could not create target script file at {:?}", target_path))?;
    
    script.write_fmt(target_template!(root_package_name.replace("-", "_")))?;

    let mut cargo_toml_path = fuzz_folder.clone();
    cargo_toml_path.push("Cargo.toml");

    let mut cargo = fs::OpenOptions::new().append(true).open(cargo_toml_path)?;
    
    Ok(cargo.write_fmt(toml_bin_template!(target))?)
}

fn init_command() -> Result<()> {
    let target = "target1";

    let root_folder = find_package()?;
    let root_package_name = root_package_name(&root_folder)?;

    let fuzz_folder = root_folder.join("fuzz");

    clone_and_compile_fuzzcheck_library(&fuzz_folder);

    let fuzzed_targets_folder = fuzz_folder.join("fuzzed_projects");

    // TODO: check if the project is already initialized
    fs::create_dir(&fuzz_folder)?;
    fs::create_dir(&fuzzed_targets_folder)?;

    let mut cargo = fs::File::create(fuzz_folder.join("Cargo.toml"))?;
    cargo.write_fmt(toml_template!(root_package_name))?;

    let mut ignore = fs::File::create(fuzz_folder.join(".gitignore"))?;
    ignore.write_fmt(gitignore_template!())?;

    create_target_template(&root_package_name, &fuzz_folder, &fuzzed_targets_folder, target)
        .chain_err(|| format!("could not create template file for target {:?}", target))?;

    Ok(())
}

fn collect_targets(manifest: &toml::Value) -> Vec<String> {
    let bins = manifest.as_table().and_then(|v| v.get("bin"))
                                  .and_then(toml::Value::as_array);
    if let Some(bins) = bins {
        bins.iter().map(|bin|
            bin.as_table().and_then(|v| v.get("name")).and_then(toml::Value::as_str)
        ).filter_map(|name| name.map(String::from)).collect()
    } else {
        Vec::new()
    }
}

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

fn fuzz_command(arguments: CommandLineArguments, target_triple: &str) {
    run_command(&arguments, target_triple);
}

fn shrink_command(arguments: CommandLineArguments, target_triple: &str) {
    run_command(&arguments, target_triple);
}

// TODO: rename CommandLineArguments
fn minimize_command(mut arguments: CommandLineArguments, target_triple: &str) -> ! {
    let file_to_minimize = (&arguments.input_file).as_ref().unwrap().clone();

    let artifacts_folder = {
        let mut x = file_to_minimize.parent().unwrap().to_path_buf();
        x.push(file_to_minimize.file_stem().unwrap());
        x = x.with_extension("minimized");
        x
    };
    let _ = std::fs::create_dir(&artifacts_folder);
    arguments.artifacts_folder = Some(artifacts_folder.clone());

    fn simplest_input_file(folder: &Path) -> Option<PathBuf> {
        let files_with_complexity = std::fs::read_dir(folder)
            .unwrap()
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
        let (file, _) = files_with_complexity
            .min_by(|x, y| std::cmp::PartialOrd::partial_cmp(&x.1, &y.1).unwrap_or(Ordering::Equal))?;
        Some(file)
    }

    if let Some(simplest) = simplest_input_file(&artifacts_folder.as_path()) {
        arguments.input_file = Some(simplest);
    }
    arguments.command = FuzzerCommand::Read;

    let o = run_command(&arguments, target_triple);
    assert!(o.status.success() == false);

    // hjhjb.minimized/hshs.parent() != hjhjb.minimized/ -> copy hshs to hjhjb.minimized/hshs
    //let destination = artifacts_folder.join(arguments.input_file.file_name());
    // if arguments.input_file.unwrap().parent() != Some(artifacts_folder.as_path()) {
    //     std::fs::copy(arguments.input_file, artifacts_folder.to_owned() + arguments.input_file);
    // }

    arguments.command = FuzzerCommand::Minimize;

    loop {
        arguments.input_file = simplest_input_file(&artifacts_folder).or(arguments.input_file);

        run_command(&arguments, target_triple);
    }
}

fn run_command(args: &CommandLineArguments, target_triple: &str) -> std::process::Output {
    let mut s: Vec<String> = Vec::new();

    let input_file_args = args.input_file.clone().map(|f| {
        vec![
            "--".to_owned() + INPUT_FILE_FLAG,
            f.as_path().to_str().unwrap().to_string(),
        ]
    });
    let corpus_in_args = args.corpus_in.clone().map(|f| {
        vec![
            "--".to_owned() + IN_CORPUS_FLAG,
            f.as_path().to_str().unwrap().to_string(),
        ]
    });
    let corpus_out_args = args.corpus_out.clone().map(|f| {
        vec![
            "--".to_owned() + OUT_CORPUS_FLAG,
            f.as_path().to_str().unwrap().to_string(),
        ]
    });
    let artifacts_args = args.artifacts_folder.clone().map(|f| {
        vec![
            "--".to_owned() + ARTIFACTS_FLAG,
            f.as_path().to_str().unwrap().to_string(),
        ]
    });

    match args.command {
        FuzzerCommand::Read => {
            s.push("-c read".to_owned());
            if let Some(input_file_args) = input_file_args {
                s.append(&mut input_file_args.clone());
            }
            if let Some(artifacts_args) = artifacts_args {
                s.append(&mut artifacts_args.clone());
            }
        }
        FuzzerCommand::Minimize => {
            s.push("-c tmin".to_owned());
            if let Some(input_file_args) = input_file_args {
                s.append(&mut input_file_args.clone());
            }
            if let Some(artifacts_args) = artifacts_args {
                s.append(&mut artifacts_args.clone());
            }
            s.push("--".to_owned() + MUT_DEPTH_FLAG);
            s.push(args.mutate_depth.to_string());
        }
        FuzzerCommand::Shrink => {
            s.push("-c cmin".to_owned());
            if let Some(corpus_in_args) = corpus_in_args {
                s.append(&mut corpus_in_args.clone());
            }
            if let Some(corpus_out_args) = corpus_out_args {
                s.append(&mut corpus_out_args.clone());
            }
            s.push("--".to_owned() + CORPUS_SIZE_FLAG);
            s.push(args.corpus_size.to_string());
        }
        FuzzerCommand::Fuzz => {
            s.push("fuzz".to_owned());
            if let Some(corpus_in_args) = corpus_in_args {
                s.append(&mut corpus_in_args.clone());
            }
            if let Some(corpus_out_args) = corpus_out_args {
                s.append(&mut corpus_out_args.clone());
            }
            if let Some(artifacts_args) = artifacts_args {
                s.append(&mut artifacts_args.clone());
            }
            // TODO: no-corpus-in, no-corpus-out, no-artifacts

            s.push("--".to_owned() + MAX_NBR_RUNS_FLAG);
            s.push(args.max_nbr_of_runs.to_string());
            s.push("--".to_owned() + MAX_INPUT_CPLX_FLAG);
            s.push(args.max_input_cplx.to_string());
            s.push("--".to_owned() + MUT_DEPTH_FLAG);
            s.push(args.mutate_depth.to_string());
        }
    }
    let cur_dir = std::env::current_dir().expect("");
    let fuzzcheck_lib = cur_dir.join("fuzzcheck-rs/target/release/deps");

    let rustflags: String = format!(
        "--cfg fuzzing \
         -Cpasses=sancov \
         -Cllvm-args=-sanitizer-coverage-level=4 \
         -Cllvm-args=-sanitizer-coverage-trace-pc-guard \
         -Cllvm-args=-sanitizer-coverage-trace-compares \
         -Cllvm-args=-sanitizer-coverage-trace-divs \
         -Cllvm-args=-sanitizer-coverage-trace-geps \
         -Cllvm-args=-sanitizer-coverage-prune-blocks=0 \
         -L /Users/loiclecrenier/Documents/rust/real_world_fuzzcheck/fuzzcheck-rs/target/release/deps",
    );

    Command::new("cargo")
        .env("RUSTFLAGS", rustflags)
        .arg("run")
        .arg("--release")
        .arg("--target")
        .arg(target_triple)
        .arg("--")
        .args(s)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .output()
        .expect("failed to execute process")
}
