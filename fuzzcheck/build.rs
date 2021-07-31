fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();
    let file_to_compile = match target_os.as_str() {
        "macos" | "ios" => "src/code_coverage_sensor/instrumentation_pointers_mac.c",
        "linux" => "src/code_coverage_sensor/instrumentation_pointers_linux.c",
        _ => panic!("fuzzcheck only work on macOS and Linux")
    };

    cc::Build::new()
        .file(file_to_compile)
        .compile("instrumentation_pointers");
    println!("cargo:rerun-if-changed={}", file_to_compile);
}
