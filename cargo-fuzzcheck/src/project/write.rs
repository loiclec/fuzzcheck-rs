use decent_toml_rs_alternative as toml;
use toml::ToToml;

use crate::project::*;

use std::result::Result;

use std::fs;

use std::io;

use std::path::Path;

impl Fuzz {
    pub fn write(&self, path: &Path) -> Result<(), io::Error> {
        let fuzz_path = path.to_path_buf();
        fs::create_dir(&fuzz_path)?;

        self.non_instrumented.write(&fuzz_path)?;
        self.instrumented.write(&fuzz_path)?;
        if let Ok(corpora) = &self.corpora {
            corpora.write()?;
        }

        if let Ok(artifacts) = &self.artifacts {
            artifacts.write()?;
        }

        if let Some(gitignore) = &self.gitignore {
            let gitignore_path = path.join(".gitignore");
            fs::write(gitignore_path, gitignore.to_string().into_bytes())?;
        }

        let config_toml_path = path.join("fuzzcheck.toml");

        let config_toml_value = self.config_toml.to_toml().unwrap();
        let config_toml_contents = toml::to_toml_file_content(config_toml_value);

        fs::write(config_toml_path, config_toml_contents.into_bytes())?;

        Ok(())
    }
}

impl NonInstrumented {
    pub fn write(&self, path: &Path) -> Result<(), io::Error> {
        let non_instrumented_path = path.join("non_instrumented");
        fs::create_dir(&non_instrumented_path)?;

        self.src.write(&non_instrumented_path)?;
        self.fuzz_targets.write(&non_instrumented_path)?;
        self.build_rs.write(&non_instrumented_path)?;
        self.cargo_toml.write(&non_instrumented_path)?;

        Ok(())
    }
}

impl Instrumented {
    pub fn write(&self, path: &Path) -> Result<(), io::Error> {
        let instrumented_path = path.join("instrumented");
        fs::create_dir(&instrumented_path)?;

        self.src.write(&instrumented_path)?;
        self.cargo_toml.write(&instrumented_path)?;
        Ok(())
    }
}

impl SrcLibRs {
    pub fn write(&self, path: &Path) -> Result<(), io::Error> {
        let src_path = path.join("src");
        fs::create_dir(&src_path)?;

        let src_lib_rs_path = src_path.join("lib.rs");
        fs::write(src_lib_rs_path, &self.content)?;

        Ok(())
    }
}

impl Corpora {
    pub fn write(&self) -> Result<(), io::Error> {
        for path in &self.corpora {
            fs::create_dir_all(path)?;
        }

        Ok(())
    }
}
impl Artifacts {
    pub fn write(&self) -> Result<(), io::Error> {
        for path in &self.artifacts {
            fs::create_dir_all(path)?;
        }

        Ok(())
    }
}

impl BuildRs {
    pub fn write(&self, path: &Path) -> Result<(), io::Error> {
        let build_rs_path = path.join("build.rs");
        fs::write(build_rs_path, &self.content)?;

        Ok(())
    }
}

impl CargoToml {
    pub fn write(&self, path: &Path) -> Result<(), io::Error> {
        let cargo_toml_path = path.join("Cargo.toml");

        let config_toml_contents = toml::print(&self.toml);

        fs::write(cargo_toml_path, &config_toml_contents.into_bytes())?;

        Ok(())
    }
}

impl FuzzTargets {
    pub fn write(&self, path: &Path) -> Result<(), io::Error> {
        let fuzz_targets_path = path.join("fuzz_targets");
        fs::create_dir(&fuzz_targets_path)?;

        for (target_name, content) in &self.targets {
            let target_path = fuzz_targets_path.join(target_name).with_extension("rs");
            fs::write(target_path, content)?;
        }
        Ok(())
    }
}
