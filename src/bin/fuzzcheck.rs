
extern crate serde_json;
use std::cmp::Ordering;
use std::process::Command;
use std::path::{Path, PathBuf};
use std::fs::DirBuilder;

fn main() {
    // let output = Command::new("echo")
    //     .arg("Hello world")
    //     .output()
    //     .expect("Failed to execute command");
    
    // let x = output.stdout.as_slice();
    // println!("{:?}", x);
}

// // TODO: rename CommandLineArguments
// fn minimize_command(executable: &Path, mut arguments: CommandLineArguments) -> ! {
//     let file_to_minimize = arguments.world_info.input_file.unwrap();
//     let exec = Command::new(executable.to_path_buf());

//     let artifacts_folder = {
//         let mut x = file_to_minimize.parent().unwrap().to_path_buf();
//         x.push(file_to_minimize.file_name().unwrap());
//         x.push(".minimized");
//         x
//     };
//     let _ = std::fs::create_dir(&artifacts_folder);
//     arguments.world_info.artifacts_folder = Some(artifacts_folder);    

//     fn simplest_input_file(folder: &Path) -> Option<PathBuf> {
//         let files_with_complexity = std::fs::read_dir(&folder).unwrap().map(|path| -> Option<(PathBuf, f64)> {
//             let path = path.ok()?.path();
//             let data = std::fs::read_to_string(&path).ok()?;
//             let json = serde_json::from_str::<serde_json::Value>(&data).ok()?;
//             let complexity = json["complexity"].as_f64()?;
//             Some((path.to_path_buf(), complexity))
//         })?;
//         let (file, _) = files_with_complexity.min_by(|x, y| std::cmp::PartialOrd::partial_cmp(&x.1, &y.1).unwrap_or(Ordering::Equal))?;
//         Some(file)
//     }

//     arguments.world_info.input_file = simplest_input_file(&artifacts_folder);
//     arguments.settings.command = FuzzerCommand::Read;

//     //Command::new(executable).args(arguments.)

//     loop {

//     }

//     std::process::exit(0);
// }