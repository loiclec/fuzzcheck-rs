use fuzzcheck_arg_parser::*;

use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::string::String;

use std::fs;
use std::io::Read;
use std::io::Write;

use std::fmt;
use std::fmt::Display;

pub const COMMAND_INIT: &str = "init";
pub const COMMAND_RUN: &str = "run";
pub const COMMAND_CLEAN: &str = "clean";

#[macro_use]
mod templates;

#[derive(Debug)]
enum MyError {
    Toml(toml::de::Error),
    Io(std::io::Error),
    Str(String),
}
impl Display for MyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MyError::Toml(e) => write!(f, "{}", e),
            MyError::Io(e) => write!(f, "{}", e),
            MyError::Str(e) => write!(f, "{}", e),
        }
    }
}
impl From<std::io::Error> for MyError {
    fn from(e: std::io::Error) -> Self {
        MyError::Io(e)
    }
}
impl From<toml::de::Error> for MyError {
    fn from(e: toml::de::Error) -> Self {
        MyError::Toml(e)
    }
}
impl From<String> for MyError {
    fn from(e: String) -> Self {
        MyError::Str(e)
    }
}

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

    match env_args[start_idx].as_str() {
        COMMAND_INIT => {
            let fuzzcheck_path = if env_args.len() > (start_idx + 1) {
                env_args[start_idx + 1].as_str().trim()
            } else {
                "https://github.com/loiclec/fuzzcheck-rs"
            };

            let result = init_command(fuzzcheck_path);
            println!("{:#?}", result);
            return;
        }
        COMMAND_CLEAN => {
            let result = clean_command();
            println!("{:#?}", result);
            return;
        }
        COMMAND_RUN => {
            if env_args.len() <= start_idx + 1 {
                println!("No fuzz target was given.");
                println!();
                println!("{}", help);
                return;
            }
            let target = &env_args[start_idx + 1];

            let mut defaults = DEFAULT_ARGUMENTS.clone();
            let defaults_in_corpus = format!("fuzz/non_instrumented/fuzz_targets/{}/", target) + defaults.in_corpus;
            let defaults_out_corpus = format!("fuzz/non_instrumented/fuzz_targets/{}/", target) + defaults.out_corpus;
            let defaults_artifacts = format!("fuzz/non_instrumented/fuzz_targets/{}/", target) + defaults.artifacts;
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
        _ => {
            println!("Invalid command: {}", env_args[1]);
            println!();
            println!("{}", help);
            return;
        }
    }
}

fn root_package_name(root_folder: &PathBuf) -> Result<String, MyError> {
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

fn clean_command() -> Result<(), MyError> {
    let fuzz_folder = std::env::current_dir()?.join("fuzz");
    let non_instrumented_folder = fuzz_folder.join("non_instrumented");
    let instrumented_folder = fuzz_folder.join("instrumented");

    Command::new("cargo")
        .current_dir(non_instrumented_folder)
        .args(vec!["clean"])
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .output()?;

    Command::new("cargo")
        .current_dir(instrumented_folder)
        .args(vec!["clean"])
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .output()?;

    Ok(())
}

fn init_command(fuzzcheck_path: &str) -> Result<(), MyError> {
    let target = "target1";

    let root_folder = std::env::current_dir()?;
    let root_package_name = root_package_name(&root_folder)?;

    let fuzz_folder = root_folder.join("fuzz");
    let non_instrumented_folder = fuzz_folder.join("non_instrumented");
    let instrumented_folder = fuzz_folder.join("instrumented");
    let fuzz_targets_folder = non_instrumented_folder.join("fuzz_targets");

    {
        // fuzz
        if !fuzz_folder.as_path().is_dir() {
            fs::create_dir(&fuzz_folder)?;
        }

        let gitignore_file = fuzz_folder.join(".gitignore");
        if !gitignore_file.as_path().is_file() {
            let mut ignore = fs::File::create(gitignore_file)?;
            ignore.write_fmt(gitignore_template!())?;
        }
    }

    {
        // non-instrumented

        // create fuzz/non_instrumented directory
        if !non_instrumented_folder.as_path().is_dir() {
            fs::create_dir(&non_instrumented_folder)?;

            // create fuzz/non_instrumented/src/lib.rs
            let src_folder = non_instrumented_folder.join("src");
            fs::create_dir(&src_folder)?;
            let lib_rs_path = src_folder.join("lib.rs");
            let _ = fs::File::create(lib_rs_path)?;
        }

        // create Cargo.toml
        let cargo_non_instrumented_toml_file = non_instrumented_folder.join("Cargo.toml");
        if !cargo_non_instrumented_toml_file.as_path().is_file() {
            let mut cargo = fs::File::create(cargo_non_instrumented_toml_file)?;
            let fuzzcheck_deps = if fuzzcheck_path.starts_with("file://") {
                let folder = Path::new(fuzzcheck_path.trim_start_matches("file://"));
                (
                    format!("path = \"{}\"", folder.join("fuzzcheck").display()),
                    format!("path = \"{}\"", folder.join("fuzzcheck_mutators").display()),
                    format!("path = \"{}\"", folder.join("fuzzcheck_serializer").display()),
                )
            } else {
                (
                    format!("git = \"{}\"", fuzzcheck_path),
                    format!("git = \"{}\"", fuzzcheck_path),
                    format!("git = \"{}\"", fuzzcheck_path),
                )
            };
            cargo.write_fmt(non_instrumented_toml_template!(
                root_package_name,
                fuzzcheck_deps.0,
                fuzzcheck_deps.1,
                fuzzcheck_deps.2,
                target
            ))?;
        }
        // create build.rs
        let build_rs_path = non_instrumented_folder.join("build.rs");
        if !build_rs_path.is_file() {
            let mut build_rs = fs::File::create(build_rs_path)?;

            let instrumented_target_folder_0 = instrumented_folder.join("target/release/deps");
            let instrumented_target_folder_1 =
                instrumented_folder.join(format!("target/{}/release/deps", default_target()));
            build_rs.write_fmt(build_rs_template!(
                instrumented_target_folder_0,
                instrumented_target_folder_1
            ))?;
        }

        // create fuzz/non_instrumented/fuzz_targets directory
        if !fuzz_targets_folder.as_path().is_dir() {
            fs::create_dir(&fuzz_targets_folder)?;
        }

        // fill fuzz/non_instrumented/fuzz_targets directory
        let mut target_path = fuzz_targets_folder;
        target_path.push(target);
        // fuzzer output_folder is the fuzz/non_instrumented/fuzz_targets/target1/
        let fuzzer_output_folder = target_path.clone();
        // target_path is fuzz/non_instrumented/fuzz_targets/target1.rs
        target_path.set_extension("rs");

        if !target_path.is_file() {
            let mut script = fs::File::create(target_path)?;
            script.write_fmt(target_template!(root_package_name.replace("-", "_")))?;
        }

        if !fuzzer_output_folder.as_path().is_dir() {
            fs::create_dir(fuzzer_output_folder)?;
        }
    }

    {
        // instrumented
        if !instrumented_folder.as_path().is_dir() {
            fs::create_dir(&instrumented_folder)?;
        }

        let cargo_instrumented_toml_file = instrumented_folder.join("Cargo.toml");
        if !cargo_instrumented_toml_file.as_path().is_file() {
            let mut cargo = fs::File::create(cargo_instrumented_toml_file)?;
            cargo.write_fmt(instrumented_toml_template!(root_package_name))?;
        }

        let src_folder = instrumented_folder.join("src");
        if !src_folder.as_path().is_dir() {
            fs::create_dir(&src_folder)?;
            let lib_rs_path = src_folder.join("lib.rs");
            let mut lib_rs = fs::File::create(lib_rs_path)?;
            lib_rs.write_fmt(instrumented_lib_rs_template!(root_package_name.replace("-", "_")))?;
        }
    }

    Ok(())
}

fn use_gold_linker() -> bool {
    match Command::new("which") // check if the gold linker is available
        .args(&["ld.gold"])
        .status()
    {
        Err(_) => false,
        Ok(status) => match status.code() {
            Some(0) => true,
            _ => false,
        },
    }
}

fn instrumented_compile(instrumented_folder: &PathBuf, target_triple: &str) -> Result<(), MyError> {
    let mut rustflags: String = "--cfg fuzzing \
                                 -Ctarget-cpu=native \
                                 -Cmetadata=fuzzing \
                                 -Cpasses=sancov \
                                 -Clinker-plugin-lto=1 \
                                 -Cllvm-args=-sanitizer-coverage-level=4 \
                                 -Cllvm-args=-sanitizer-coverage-inline-8bit-counters \
                                 -Cforce-frame-pointers=yes"
        .into();

    if use_gold_linker() {
        rustflags.push_str(" -Clink-arg=-fuse-ld=gold");
    }

    let output = Command::new("cargo")
        .env("RUSTFLAGS", rustflags)
        .arg("build")
        .arg("--manifest-path")
        .arg(instrumented_folder.join("Cargo.toml"))
        .arg("--release")
        .arg("--target")
        .arg(target_triple)
        .arg("--verbose")
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .output()?;

    if output.status.success() {
        Ok(())
    } else {
        Err("Could not compile the instrumented part of the fuzz target"
            .to_string()
            .into())
    }
}

fn command_line_arguments_string(args: &CommandLineArguments) -> Vec<String> {
    let mut s: Vec<String> = Vec::new();

    let input_file_args = args
        .input_file
        .clone()
        .map(|f| vec!["--".to_owned() + INPUT_FILE_FLAG, path_str(f)]);

    let corpus_in_args = args
        .corpus_in
        .clone()
        .map(|f| vec!["--".to_owned() + IN_CORPUS_FLAG, path_str(f)])
        .unwrap_or_else(|| vec!["--".to_owned() + NO_IN_CORPUS_FLAG]);

    let corpus_out_args = args
        .corpus_out
        .clone()
        .map(|f| vec!["--".to_owned() + OUT_CORPUS_FLAG, path_str(f)])
        .unwrap_or_else(|| vec!["--".to_owned() + NO_OUT_CORPUS_FLAG]);

    let artifacts_args = args
        .artifacts_folder
        .clone()
        .map(|f| vec!["--".to_owned() + ARTIFACTS_FLAG, path_str(f)])
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
        "--".to_owned() + MAX_NBR_RUNS_FLAG,
        args.max_nbr_of_runs.to_string(),
    ]);

    s
}

fn launch_exec(
    args: &CommandLineArguments,
    target_folder: &PathBuf,
    non_instrumented_folder: &PathBuf,
) -> Result<std::process::Output, MyError> {
    let s = command_line_arguments_string(args);

    let exec = non_instrumented_folder.join(format!(
        "target/{}/release/{}",
        default_target(),
        &target_folder.file_name().unwrap().to_str().unwrap()
    ));

    Command::new(exec)
        .args(s)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .output()
        .map_err(|x| x.into())
}

fn run_command(
    args: &CommandLineArguments,
    target_folder: &PathBuf,
    instrumented_folder: &PathBuf,
    non_instrumented_folder: &PathBuf,
    target_triple: &str,
) -> Result<std::process::Output, MyError> {
    let s = command_line_arguments_string(args);

    instrumented_compile(instrumented_folder, target_triple)?;

    let mut rustflags: String = "--cfg fuzzing -Ctarget-cpu=native".to_string();

    if use_gold_linker() {
        rustflags.push_str(" -Clink-arg=-fuse-ld=gold");
    }

    Command::new("cargo")
        .env("RUSTFLAGS", rustflags)
        .arg("run")
        .arg("--bin")
        .arg(target_folder.file_name().unwrap())
        .arg("--manifest-path")
        .arg(non_instrumented_folder.join("Cargo.toml"))
        .arg("--release")
        .arg("--target")
        .arg(target_triple)
        .arg("--verbose")
        .arg("--")
        .args(s)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .output()
        .map_err(|x| x.into())
}

fn exec_normal_command(arguments: CommandLineArguments, target: &str, target_triple: &str) -> Result<(), MyError> {
    let root_folder = std::env::current_dir()?;

    let fuzz_folder = root_folder.join("fuzz");
    let instrumented_folder = fuzz_folder.join("instrumented");
    let non_instrumented_folder = fuzz_folder.join("non_instrumented");
    let fuzz_targets_folder = fuzz_folder.join("fuzz_targets");
    let target_folder = fuzz_targets_folder.join(target);

    run_command(
        &arguments,
        &target_folder,
        &instrumented_folder,
        &non_instrumented_folder,
        target_triple,
    )?;

    Ok(())
}

// TODO: rename CommandLineArguments
fn exec_input_minify_command(
    mut arguments: CommandLineArguments,
    target: &str,
    target_triple: &str,
) -> Result<(), MyError> {
    let root_folder = std::env::current_dir()?;
    let fuzz_folder = &root_folder.join("fuzz");
    let instrumented_folder = fuzz_folder.join("instrumented");
    let non_instrumented_folder = fuzz_folder.join("non_instrumented");
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

    let o = run_command(
        &arguments,
        target_folder,
        &instrumented_folder,
        &non_instrumented_folder,
        target_triple,
    )?;
    assert!(!o.status.success());

    // hjhjb.minifyd/hshs.parent() != hjhjb.minifyd/ -> copy hshs to hjhjb.minifyd/hshs
    //let destination = artifacts_folder.join(arguments.input_file.file_name());
    // if arguments.input_file.unwrap().parent() != Some(artifacts_folder.as_path()) {
    //     std::fs::copy(arguments.input_file, artifacts_folder.to_owned() + arguments.input_file);
    // }

    arguments.command = FuzzerCommand::MinifyInput;

    loop {
        arguments.input_file = simplest_input_file(&artifacts_folder).or(arguments.input_file);

        launch_exec(&arguments, target_folder, &non_instrumented_folder)?;
    }
}

fn path_str(p: PathBuf) -> String {
    p.as_path().to_str().unwrap().to_owned()
}
