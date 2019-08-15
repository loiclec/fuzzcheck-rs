# Fuzzcheck

## TODO: Note informing people of the state of the project and asking to be hired

Fuzzcheck is a structure-aware, in-process, coverage-guided, evolutionary 
fuzzing engine for Rust functions.

Given a function `test: (T) -> Bool`, it tries to find a value of type `T` that
fails the test or leads to a crash.

Unlike other fuzzing engines, it doesn't work with raw binary buffers but 
instead work with values of any type `T` directly. The complexity of the inputs 
and the way to mutate them is given by functions defined by the user.

Fuzzcheck works by maintaining a pool of test inputs and ranking them using the
complexity of the input and the uniqueness of the code coverage caused by 
`test(input)`. From that pool, it selects a high-ranking input, mutates it, and
runs the test function again. If the new mutated input has an interesting code 
coverage then it is added to the pool, otherwise, Fuzzcheck tries again with a 
different input and mutation.

In pseudocode:
```rust
loop {
    let mut input = pool.select();
    mutate(&mut input);

    let analysis = analyze(test, &input);

    match analysis {
        Failed => reportFailure(input),
        Interesting(score) => pool.add(input, score),
        NotInteresting => continue
    }
}
```

## Usage

You can find an example project using Fuzzcheck 
[here](https:://github.com/loiclec/fuzzcheck-rs-example).

The first step is to clone Fuzzcheck somewhere on your computer and
build it with cargo nightly.

```bash
git clone https:://github.com/loiclec/fuzzcheck-rs
cd fuzzcheck-rs
cargo +nightly build --release
```

Then, somewhere else, create a new cargo binary crate. It will contain the
test function and the code necessary to launch the fuzzer.

```bash
# Create the directory
mkdir fuzzcheck-test
cd fuzzcheck-test
# Set the cargo version to nightly
rustup override set nightly
# Create the crate
cargo init --bin
```

Then, we need to tell cargo where to find the fuzzcheck library that we just
built. This is done by creating a `build.rs` file, which will expect
the `FUZZCHECK_LIB` environment variable to be set to the 
`./target/release/deps` folder inside `fuzzcheck-rs`. On my computer, it is 
`/Users/loiclecrenier/Documents/rust/fuzzcheck-rs/target/release/deps`.

```rust
// ./build.rs
use std::env;

fn main() {
    let lib = env::var("FUZZCHECK_LIB").unwrap();
    println!("cargo:rustc-link-search=all={}", lib);
    println!("cargo:rerun-if-changed={}", lib);
}
```

Then, we add two dependencies to `Cargo.toml`:
```toml
[dependencies]
rand = "0.7.0"

# Todo use git version, as I don't think fuzzcheck-input will be 
# published to crates.io
fuzzcheck_input = "0.1.0"
```

`fuzzcheck_input` is a library that contains useful mutators to use
with Fuzzcheck. 
`rand` is needed for technical reasons that I won't explain here.

Then, we tell cargo which instrumentation to use when building the project,
such that its code coverage can be analyzed. This can be done by adding a
file `.cargo/config`:

```toml
[build]
rustflags = [
    "-Cpasses=sancov",
    "-Cllvm-args=-sanitizer-coverage-level=4",
    "-Cllvm-args=-sanitizer-coverage-trace-compares",
    "-Cllvm-args=-sanitizer-coverage-trace-divs",
    "-Cllvm-args=-sanitizer-coverage-trace-geps",
    "-Cllvm-args=-sanitizer-coverage-prune-blocks=0"
]
target = "x86_64-apple-darwin"
```

Note that the `target` key is important. Replace its value with 
`#TODO` if you are using Windows or `#TODO` if you are using Linux.

We can now write the test function and write the code that will launch the
fuzzing process.

We will need to use three dependencies: `fuzzcheck`, `fuzzcheck_input`, and `rand`.
```rust
extern crate fuzzcheck;
use fuzzcheck::fuzzer;

use fuzzcheck_input::integer::*;
use fuzzcheck_input::vector::*;

use rand;
```

In `maim.rs`, we can define a function `fn test(input: Vec<u8>) -> bool` that
fails under very specific conditions.

```rust
fn test(data: &Vec<u8>) -> bool {
    if 
        data.len() > 7 &&
        data[0] == 0 &&
        data[1] == 78 &&
        data[2] == 56 &&
        data[3] == 2 &&
        data[4] == 76 &&
        data[5] == 7 &&
        data[6] == 100 &&
        data[7] == 102
    {
        false
    } else {
        true
    }
}
```

Note that while the input is of type `Vec<u8>`, it could equally easily be
anything such as `String`, `HashMap<T, U>`, etc. The example linked at the
beginning of the readme tests a function working with a Graph data structure
defined by a third-party library.

Then, in the body of the `main` function, launch the fuzzing process:

```rust
fn main() {
    let u8_gen = IntegerGenerator::<u8>::new(10); // don't pay attention to the `10` parameter (TODO: make it the default)
    let vec_gen = VectorGenerator::new(u8_gen);
    let result = fuzzer::launch(test, vec_gen, rand::thread_rng());
    println!("{:?}", result);
}
```

The first step is to create an `InputGenerator`, which is something that can
determine the complexity of an input, generate a random input, mutate it, as
well as decode it and encode it to a file in order to save the result of the
fuzzing process.

To create an `InputGenerator` that manipulates values of type `Vec<u8>`, I
compose a `VectorGenerator` with an `IntegerGenerator` of `u8`. These
generators are defined in a separate crate called `fuzzcheck-input`.

Then, I call the function `fuzzcheck::fuzzer::launch`, and pass it the test
function, the input generator, and a random number generator (TODO: remove rng
requirement? It may not be so simple).

The final step is to compile the executable and use it via the `fuzzcheck`
command line tool.

To compile it, define the environment variable `FUZZCHECK_LIB` to your own
absolute path pointing to `fuzzcheck-rs/target/release/deps`.

```bash
export FUZZCHECK_LIB="/Users/loiclecrenier/Documents/rust/fuzzcheck/target/release/deps/"
```

And then build with cargo:

```bash
cargo build --release
```

You can launch the fuzzer either by launching the executable directly (not recommended, as you won't have access to every feature):

```bash
cargo run --release
```

or via the `fuzzcheck` executable (if you didn't install it to your $PATH, you should use the full path to the executable, such as 
`/Users/loiclecrenier/Documents/rust/fuzzcheck/target/release/fuzzcheck`).

```bash
fuzzcheck --target target/x86_64-apple-darwin/release/fuzzcheck-test
```

## TODO: Commands

## TODO: Flags

## TODO: Creating an InputGenerator

## TODO: Why fuzzcheck cannot be used as a simple dependency, and why it cannot export any trait that depend on third-party libraries

## TODO: Ideas on how to best use Fuzzcheck

