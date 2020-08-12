extern crate toml;
use crate::default_target;
use crate::project::*;

use std::path::Path;

pub const DEFAULT_TARGET_NAME: &str = "target1";
pub const DEFAULT_LTO: bool = true;
pub const DEFAULT_COVERAGE_LEVEL: u8 = 4;
pub const DEFAULT_TRACE_COMPARES: bool = true;
pub const DEFAULT_STACK_DEPTH: bool = false;

impl Fuzz {
    pub fn init(path: &Path, library: &str, fuzzcheck_path_or_version: &str) -> Self {
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

        let instrumented = Instrumented::init(library, &fuzzcheck_deps.1, &fuzzcheck_deps.2);
        let instrumented_folder = path.join("instrumented");

        let non_instrumented = NonInstrumented::init(library, &instrumented_folder, &fuzzcheck_deps.0);

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

        let mut config_targets = HashMap::new();
        let mut config_target = Config::empty();
        config_target.in_corpus = Some(Path::new("fuzz/corpora/target1").to_path_buf());
        config_target.out_corpus = Some(Path::new("fuzz/corpora/target1").to_path_buf());
        config_target.artifacts = Some(Path::new("fuzz/artifacts/target1").to_path_buf());

        config_targets.insert("target1".to_string(), config_target);

        let config_toml = ConfigToml {
            default: Config {
                coverage_level: Some(DEFAULT_COVERAGE_LEVEL),
                trace_compares: Some(DEFAULT_TRACE_COMPARES),
                stack_depth: Some(DEFAULT_STACK_DEPTH),

                lto: Some(DEFAULT_LTO),
                extra_cargo_flags: None,

                instrumented_default_features: None,
                non_instrumented_default_features: None,
                instrumented_features: None,
                non_instrumented_features: None,

                corpus_size: Some(100),
                max_nbr_of_runs: None,
                max_cplx: Some(DEFAULT_ARGUMENTS.max_input_cplx),
                timeout: None,

                in_corpus: None,
                out_corpus: None,
                artifacts: None,

                no_in_corpus: None,
                no_out_corpus: None,
                no_artifacts: None,
            },
            targets: config_targets,
        };

        Self {
            instrumented,
            non_instrumented,
            corpora,
            artifacts,
            gitignore,
            config_toml,
        }
    }
}

impl NonInstrumented {
    pub fn init(library: &str, instrumented_folder: &Path, fuzzcheck_dep: &str) -> Self {
        let src = SrcLibRs::init_non_instrumented();

        let fuzz_targets = FuzzTargets::init(library);

        let instrumented_target_folder_0 = instrumented_folder.join("target/release/deps");
        let instrumented_target_folder_1 =
            instrumented_folder.join(format!("target/{}/release/deps", default_target()));

        let build_rs = BuildRs::init(instrumented_target_folder_0, instrumented_target_folder_1);

        let cargo_toml = CargoToml::init_non_instrumented(library, &fuzzcheck_dep);

        Self {
            src,
            fuzz_targets,
            build_rs,
            cargo_toml,
        }
    }
}

impl Instrumented {
    pub fn init(library: &str, fuzzcheck_mutators_dep: &str, fuzzcheck_serializer_dep: &str) -> Self {
        Self {
            src: SrcLibRs::init_instrumented(library),
            cargo_toml: CargoToml::init_instrumented(library, fuzzcheck_mutators_dep, fuzzcheck_serializer_dep),
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
extern crate fuzzcheck_mutators;
extern crate serde;

// re-export fuzzcheck_serializer so it can be used by the fuzz targets
pub extern crate fuzzcheck_serializer;

use serde::{{Serialize, Deserialize}};
use fuzzcheck_mutators::DefaultMutator;

#[derive(Clone, Default, DefaultMutator, Serialize, Deserialize)]
pub struct SampleData<A, B, C> {{
    a: A,
    b: Vec<B>,
    c: C
}}


// Note: the test function should not be generic, otherwise it will get monomorphised
// when compiling the non-instrumented crate, and will therefore not be instrumented
pub fn test(input: &[SampleData<u8, Option<u8>, u8>]) -> bool {{
    if 
        input.len() > 5 &&
        input[0].a == 0 &&
        input[0].b == vec![Some(2), None, Some(187)] &&
        input[0].c == 9 &&
        input[1].a == 189 &&
        input[1].b.len() > 5 &&
        input[1].b[0] == Some(89) &&
        input[1].b[1] == None &&
        input[1].b[2] == Some(213) &&
        input[1].b[3] == Some(189) &&
        input[1].b[4] == None &&
        input[1].b[5] == Some(32) &&
        input[1].c == 19 &&
        input[2].a == 200&&
        input[2].b.len() < 5 &&
        input[2].c == 132 &&
        input[3].a == 78 &&
        input[3].b.len() == 3 &&
        input[3].b[0] == Some(90) &&
        input[3].b[1] == Some(80) &&
        input[3].b[2] == Some(70) &&
        input[3].c == 156 &&
        input[4].a == 1 &&
        input[4].b == vec![Some(255), None, None, None] &&
        input[4].c == 150 &&
        input[5].a == 10 &&
        input[5].b == vec![] &&
        input[5].c == 43
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
    pub fn init_non_instrumented(library: &str, fuzzcheck_dep: &str) -> Self {
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

[dependencies.fuzzcheck]
{fuzzcheck_dep}

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
            target = DEFAULT_TARGET_NAME
        );

        let toml = toml::from_str(&content).unwrap();

        Self { toml }
    }
    pub fn init_instrumented(library: &str, fuzzcheck_mutators_dep: &str, fuzzcheck_serializer_dep: &str) -> Self {
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

[dependencies.serde]
version = '1.0'
features = ["derive"]

[dependencies.serde_json]
version = '1.0'

[dependencies.fuzzcheck_mutators]
{fuzzcheck_mutators_dep}

[dependencies.fuzzcheck_serializer]
{fuzzcheck_serializer_dep}
features = ["serde_serializer"]

# Prevent this from interfering with workspaces
[workspace]
members = ["."]

[profile.release]
debug = false
opt-level = 3
codegen-units = 1
overflow-checks = true
incremental = false

[profile.release.package.fuzzcheck_mutators_derive]
opt-level = 0
codegen-units = 16

[profile.release.package.proc-macro2]
opt-level = 0
codegen-units = 16

[profile.release.package.syn]
opt-level = 0
codegen-units = 16

[profile.release.package.quote]
opt-level = 0
codegen-units = 16

[profile.release.package.serde]
opt-level = 0
codegen-units = 16

[profile.release.package.serde_derive]
opt-level = 0
codegen-units = 16

[profile.release.package.serde_json]
opt-level = 0
codegen-units = 16
"##,
            library = library,
            fuzzcheck_mutators_dep = fuzzcheck_mutators_dep,
            fuzzcheck_serializer_dep = fuzzcheck_serializer_dep
        );
        let toml = toml::from_str(&content).unwrap();
        Self { toml }
    }
}

impl FuzzTargets {
    pub fn init(library: &str) -> Self {
        let content = format!(
            r#"
extern crate {0};
extern crate {0}_non_instrumented_fuzz;
extern crate {0}_instrumented_fuzz;

extern crate fuzzcheck;
extern crate fuzzcheck_mutators;
extern crate fuzzcheck_traits;

// Note: fuzzcheck_serializer was re-exported by the instrumented crate
// This must be done because fuzzcheck_serializer uses serde’s Serialize and 
// Deserialize traits and because serde is compiled in the instrumented crate. 
// If, instead, we added fuzzcheck_serializer to the non-instrumented crate’s 
// dependencies, it would be recompiled. Two serde crates with incompatible 
// Serialize traits would then live in the same binary. This can result in 
// confusing error messages.
use {0}_instrumented_fuzz::fuzzcheck_serializer;

use {0}_instrumented_fuzz::{{SampleData, test}};
use fuzzcheck_mutators::DefaultMutator;
use fuzzcheck_serializer::SerdeSerializer;

fn main() {{
    let mutator = Vec::<SampleData<u8, Option<u8>, u8>>::default_mutator();
    let serializer = SerdeSerializer::default();
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
