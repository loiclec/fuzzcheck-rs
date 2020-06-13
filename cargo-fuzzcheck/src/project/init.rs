extern crate toml;
use crate::default_target;
use crate::project::*;

use std::path::Path;

const DEFAULT_TARGET_NAME: &str = "target1";

impl Fuzz {
    pub fn init(path: &Path, library: &str, fuzzcheck_path_or_version: &str) -> Self {
        let instrumented = Instrumented::init(library);
        let instrumented_folder = path.join("instrumented");

        let non_instrumented = NonInstrumented::init(library, &instrumented_folder, fuzzcheck_path_or_version);

        let corpora_folder = path.join("corpora");
        let corpora = Ok(Corpora::init(&corpora_folder));

        let artifacts_folder = path.join("artifacts");
        let artifacts = Ok(Artifacts::init(&artifacts_folder));

        let gitignore = Some(
            r##"
target
corpora
artifacts
fuzzcheck-rs
"##
            .to_string(),
        );

        Self {
            instrumented,
            non_instrumented,
            corpora,
            artifacts,
            gitignore,
        }
    }
}

impl NonInstrumented {
    pub fn init(library: &str, instrumented_folder: &Path, fuzzcheck_path_or_version: &str) -> Self {
        let src = SrcLibRs::init_non_instrumented();

        let fuzz_targets = FuzzTargets::init(library);

        let instrumented_target_folder_0 = instrumented_folder.join("target/release/deps");
        let instrumented_target_folder_1 =
            instrumented_folder.join(format!("target/{}/release/deps", default_target()));

        let build_rs = BuildRs::init(instrumented_target_folder_0, instrumented_target_folder_1);

        let fuzzcheck_deps = if fuzzcheck_path_or_version.starts_with("file://") {
            let folder = Path::new(fuzzcheck_path_or_version.trim_start_matches("file://"));
            (
                format!("path = \"{}\"", folder.join("fuzzcheck").display()),
                format!("path = \"{}\"", folder.join("fuzzcheck_mutators").display()),
                format!("path = \"{}\"", folder.join("fuzzcheck_serializer").display()),
            )
        } else if fuzzcheck_path_or_version.starts_with("http") {
            (
                format!("git = \"{}\"", fuzzcheck_path_or_version),
                format!("git = \"{}\"", fuzzcheck_path_or_version),
                format!("git = \"{}\"", fuzzcheck_path_or_version),
            )
        } else {
            (
                format!("version = \"{}\"", fuzzcheck_path_or_version),
                format!("version = \"{}\"", fuzzcheck_path_or_version),
                format!("version = \"{}\"", fuzzcheck_path_or_version),
            )
        };

        let cargo_toml =
            CargoToml::init_non_instrumented(library, &fuzzcheck_deps.0, &fuzzcheck_deps.1, &fuzzcheck_deps.2);

        Self {
            src,
            fuzz_targets,
            build_rs,
            cargo_toml,
        }
    }
}

impl Instrumented {
    pub fn init(library: &str) -> Self {
        Self {
            src: SrcLibRs::init_instrumented(library),
            cargo_toml: CargoToml::init_instrumented(library),
        }
    }
}

impl Corpora {
    pub fn init(path: &Path) -> Self {
        Self {
            corpora: vec![path.join(DEFAULT_TARGET_NAME)],
        }
    }
}
impl Artifacts {
    pub fn init(path: &Path) -> Self {
        Self {
            artifacts: vec![path.join(DEFAULT_TARGET_NAME)],
        }
    }
}

impl BuildRs {
    pub fn init(instrumented_target_folder_0: PathBuf, instrumented_target_folder_1: PathBuf) -> Self {
        let content = format!(
            r##"
fn main() {{
    println!("cargo:rustc-link-search={0}");
    println!("cargo:rustc-link-search={1}");
    println!("cargo:rerun-if-changed={0}");
    println!("cargo:rerun-if-changed={1}");
}}
"##,
            instrumented_target_folder_0.display(),
            instrumented_target_folder_1.display()
        );

        Self {
            content: content.into_bytes(),
        }
    }
}

impl SrcLibRs {
    pub fn init_instrumented(library: &str) -> Self {
        let content = format!(
            r#"
extern crate {library};

pub fn test(input: &[u8]) -> bool {{
    // test goes here
    if 
        input.len() > 14 &&
        input[0] == 0 &&
        input[1] == 167 &&
        input[2] == 200 &&
        input[3] == 103 &&
        input[4] == 56 &&
        input[5] == 78 &&
        input[6] == 2 &&
        input[7] == 254 &&
        input[8] == 0 &&
        input[9] == 167 &&
        input[10] == 200 &&
        input[11] == 103 &&
        input[12] == 56 &&
        input[13] == 78 &&
        input[14] == 103
    {{
        false
    }}
    else {{
        true
    }}
}}
"#,
            library = library
        )
        .into_bytes();
        Self { content }
    }
    pub fn init_non_instrumented() -> Self {
        Self { content: Vec::new() }
    }
}

impl CargoToml {
    pub fn init_non_instrumented(
        library: &str,
        fuzzcheck_dep: &str,
        fuzzcheck_mutators_dep: &str,
        fuzzcheck_serializer_dep: &str,
    ) -> Self {
        let content = format!(
            r##"
[package]
name = "{library}-non-instrumented-fuzz"
version = "0.0.0"
authors = ["Automatically generated"]
publish = false

[package.metadata]
cargo-fuzzcheck = true

# [dependencies.{library}]
# path = "../.."
# Managed by cargo-fuzzcheck

# [dependencies.{library}-instrumented-fuzz]
# path = "../instrumented"
# Managed by cargo-fuzzcheck

[dependencies]
serde = {{ version = "1.0" }} #, features = ["derive"] }}
serde_json = "1.0"

[dependencies.fuzzcheck]
{fuzzcheck_dep}

[dependencies.fuzzcheck_mutators]
{fuzzcheck_mutators_dep}

[dependencies.fuzzcheck_serializer]
{fuzzcheck_serializer_dep}

# Prevent this from interfering with workspaces
[workspace]
members = ["."]

[[bin]]
name = "{target}"
path = "fuzz_targets/{target}.rs"

[profile.release]
debug = false
opt-level = 3
lto = "thin"
codegen-units = 1
panic = 'abort'
overflow-checks = false
incremental = false

[profile.release.package.serde_json]
opt-level = 0
codegen-units = 16

[profile.release.package.serde]
opt-level = 0
codegen-units = 16

[profile.release.package.libc]
opt-level = 0
codegen-units = 16

[profile.release.package.getopts]
opt-level = 0
codegen-units = 16

[profile.release.package.fuzzcheck_arg_parser]
opt-level = 0
codegen-units = 16
"##,
            library = library,
            fuzzcheck_dep = fuzzcheck_dep,
            fuzzcheck_mutators_dep = fuzzcheck_mutators_dep,
            fuzzcheck_serializer_dep = fuzzcheck_serializer_dep,
            target = DEFAULT_TARGET_NAME
        );

        let toml = toml::from_str(&content).unwrap();

        Self { toml }
    }
    pub fn init_instrumented(library: &str) -> Self {
        let content = format!(
            r##"
[package]
name = "{library}-instrumented-fuzz"
version = "0.0.0"
authors = ["Automatically generated"]
publish = false

[package.metadata]
cargo-fuzzcheck = true

[dependencies.{library}]
path = "../.."

# Prevent this from interfering with workspaces
[workspace]
members = ["."]

[profile.release]
debug = false
opt-level = 3
lto = "thin"
codegen-units = 1
panic = "abort"
overflow-checks = true
incremental = false
"##,
            library = library
        );
        let toml = toml::from_str(&content).unwrap();
        Self { toml }
    }
}

impl FuzzTargets {
    pub fn init(library: &str) -> Self {
        let content = format!(
            r#"
extern crate fuzzcheck;

extern crate fuzzcheck_mutators;
use fuzzcheck_mutators::integer::*;
use fuzzcheck_mutators::vector::*;

#[macro_use]
extern crate fuzzcheck_serializer;

extern crate {0};

extern crate {0}_non_instrumented_fuzz;

extern crate {0}_instrumented_fuzz;
use {0}_instrumented_fuzz::test;

extern crate serde;
extern crate serde_json;

define_serde_serializer!();

fn main() {{
    type Mutator = VecMutator<U8Mutator>;
    let mutator = Mutator::default();
    let serializer = SerdeSerializer::<Vec<u8>>::default();
    let _ = fuzzcheck::launch(test, mutator, serializer);
}}
"#,
            library
        );

        let mut targets = HashMap::new();
        targets.insert(DEFAULT_TARGET_NAME.into(), content.into_bytes());

        Self { targets }
    }
}
