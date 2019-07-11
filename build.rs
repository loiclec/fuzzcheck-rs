use cc;

fn main() {
    cc::Build::new().file("src/c_builtins.c").compile("c_builtins");
}
