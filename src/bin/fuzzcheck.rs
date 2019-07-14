
extern crate serde_json;
use std::cmp::Ordering;
use std::process::Command;
use std::path::{Path, PathBuf};
use std::string::String;
use fuzzcheck::command_line::*;
use structopt::StructOpt;

fn main() {
    let args = ToolArguments::from_args();
    match args.args.settings.command {
        FuzzerCommand::Fuzz => fuzz_command(&args.executable.as_path(), args.args),
        FuzzerCommand::Minimize => minimize_command(&args.executable.as_path(), args.args),
        FuzzerCommand::Read => panic!("unimplemented"),
    }
}

#[derive(StructOpt)]
struct ToolArguments {
    #[structopt(long = "exec", help = "Help: TODO")]
    executable: PathBuf,
    #[structopt(flatten)]
    args: CommandLineArguments
}

fn fuzz_command(executable: &Path, arguments: CommandLineArguments) {
    let o = Command::new(executable)
        .args(args_to_string(&arguments))
        .stdout(std::process::Stdio::inherit())
        .output()
        .expect("failed to execute process");
}

// TODO: rename CommandLineArguments
fn minimize_command(executable: &Path, mut arguments: CommandLineArguments) -> ! {
    let file_to_minimize = (&arguments.world_info.input_file).as_ref().unwrap().clone();

    let artifacts_folder = {
        let mut x = file_to_minimize.parent().unwrap().to_path_buf();
        x.push(file_to_minimize.file_name().unwrap());
        x = x.with_extension("minimized");
        x
    };
    let _ = std::fs::create_dir(&artifacts_folder);
    arguments.world_info.artifacts_folder = Some(artifacts_folder.clone());

    fn simplest_input_file(folder: &Path) -> Option<PathBuf> {
        let files_with_complexity = std::fs::read_dir(folder).unwrap().filter_map(|path| -> Option<(PathBuf, f64)> {
            let path = path.ok()?.path();
            let data = std::fs::read_to_string(&path).ok()?;
            let json = serde_json::from_str::<serde_json::Value>(&data).ok()?;
            // TODO: encode complexity
            let complexity = json["cplx"].as_f64()?;
            Some((path.to_path_buf(), complexity))
        });
        let (file, _) = files_with_complexity.min_by(|x, y| std::cmp::PartialOrd::partial_cmp(&x.1, &y.1).unwrap_or(Ordering::Equal))?;
        Some(file)
    }

    if let Some(simplest) = simplest_input_file(&artifacts_folder.as_path()) {
        arguments.world_info.input_file = Some(simplest);
    }
    arguments.settings.command = FuzzerCommand::Read;

    println!("{:?}", args_to_string(&arguments));

    Command::new(executable)
        .args(args_to_string(&arguments))
        .stdout(std::process::Stdio::inherit())
        .output()
        .expect("failed to execute process");

    // assert!(o.status.success() == false);

    arguments.settings.command = FuzzerCommand::Minimize;

    loop {
        arguments.world_info.input_file = simplest_input_file(&artifacts_folder);
        println!("{:?}", args_to_string(&arguments));

        let o = Command::new(executable)
            .args(args_to_string(&arguments))
            .stdout(std::process::Stdio::inherit())
            .output()
            .expect("failed to execute process");
    }
}

fn args_to_string(args: &CommandLineArguments) -> Vec<String> {
    
    let mut s: Vec<String> = Vec::new();
    
    // TODO: that doesn't seem like the best way to do it
    if let Some(f) = &args.world_info.input_file {
        s.push(String::from("--input-file"));
        s.push(f.as_path().to_str().unwrap().to_string());
    }
    if let Some(f) = &args.world_info.input_folder {
        s.push(String::from("--input-folder"));
        s.push(f.as_path().to_str().unwrap().to_string());
    }
    if let Some(f) = &args.world_info.output_folder {
        s.push(String::from("--output-folder"));
        s.push(f.as_path().to_str().unwrap().to_string());
    }
    if let Some(f) = &args.world_info.artifacts_folder {
        s.push(String::from("--artifacts-folder"));
        s.push(f.as_path().to_str().unwrap().to_string());
    }

    s.push(String::from("--max-number-of-runs"));
    s.push(args.settings.max_nbr_of_runs.to_string());
    s.push(String::from("--max-complexity"));
    s.push(args.settings.max_input_cplx.to_string());
    s.push(String::from("--mutation-depth"));
    s.push(args.settings.mutate_depth.to_string());

    s.push(String::from(
        match args.settings.command {
            FuzzerCommand::Read => "read",
            FuzzerCommand::Minimize => "minimize",
            FuzzerCommand::Fuzz => "fuzz"
        }
    ));

    s
}
