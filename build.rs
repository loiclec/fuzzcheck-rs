use cc;

fn main() {
    cc::Build::new()
        .file("src/fuzzer/c_builtins.c")
        .compile("c_builtins");
}