use std::path::Path;

pub struct LLVMCovSections {
    pub covfun: Vec<u8>,
    pub covmap: Vec<u8>,
}

#[no_coverage]
pub fn get_llvm_cov_sections(path: &Path) -> LLVMCovSections {
    let exe_data = std::fs::read(path).unwrap();
    let macho = goblin::Object::parse(&exe_data).unwrap();
    match macho {
        goblin::Object::Elf(x) => {
            let mut covfun = Vec::new();
            let mut covmap = Vec::new();
            for h in &x.section_headers {
                let name_index = h.sh_name;
                let name = x.shdr_strtab.get_at(name_index).unwrap();
                if name == "__llvm_cov_fun" {
                    let content_range = h.file_range().unwrap();
                    let data = &exe_data[content_range];
                    covfun = data.to_vec();
                }
                if name == "__llvm_cov_map" {
                    let content_range = h.file_range().unwrap();
                    let data = &exe_data[content_range];
                    covmap = data.to_vec();
                }
            }
            assert!(!covfun.is_empty());
            assert!(!covmap.is_empty());
            LLVMCovSections {
                    covfun,
                    covmap,
                }
        },
        goblin::Object::PE(_x) => todo!(),
        goblin::Object::Mach(x) => match x {
            goblin::mach::Mach::Fat(_x) => todo!(),
            goblin::mach::Mach::Binary(x) => {
                let mut covfun = Vec::new();
                let mut covmap = Vec::new();
                let xs = x.segments.as_slice();
                for x in xs {
                    let name = &x.segname;
                    let mut name: &[u8] = name;
                    for i in 0..16 {
                        if name[i] == 0 {
                            name = &name[..i];
                            break;
                        }
                    }
                    let name = String::from_utf8_lossy(&name).to_string();
                    if name == "__LLVM_COV" {
                        let sections = x.sections().unwrap();
                        for (section, data) in sections {
                            let name = section.name().unwrap();
                            if name == "__llvm_covfun" {
                                covfun = data.to_vec();
                            }
                            if name == "__llvm_covmap" {
                                covmap = data.to_vec();
                            }
                        }
                    }
                }
                assert!(!covfun.is_empty());
                assert!(!covmap.is_empty());
                LLVMCovSections {
                    covfun,
                    covmap,
                }
            }
        },
        goblin::Object::Archive(_x) => todo!(),
        goblin::Object::Unknown(_x) => todo!(),
    }
}
