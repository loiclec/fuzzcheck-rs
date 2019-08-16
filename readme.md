# Fuzzcheck

> I made Fuzzcheck in my free time during my summer vacation. It is an
> improved port of FuzzCheck for Swift, which I wrote a year ago and 
> discussed in a [talk at the Functional Swift conference](https://www.youtube.com/watch?v=23_qZePMQjA). 
> There are many, many ways in which it could be improved such that it
> becomes a powerful, easy-to-use tool for any Rust programmer. I would love 
> to keep working on it, but I have to go back to university in a week, and I
> find it hard to justify spending a significant amount of time on it.
> If you would like me to keep developing Fuzzcheck or help your company use
> it, please hire me and help me pay for my studies :)
>
> I would also greatly appreciate any contribution to the code. Unfortunately,
> I am still in the process of adding precise documentation. But you can open 
> an issue or contact me by email to ask for an explanation about Fuzzcheck 
> or propose an idea.

Fuzzcheck is a structure-aware, in-process, coverage-guided, evolutionary 
fuzzing engine for Rust functions. 

Its main aim is to be used as the input generator of property-based tests.
Detecting security flaws in an application is a non-goal.

Given a function `test: (T) -> Bool`, it tries to find a value of type `T` that
fails the test or leads to a crash.

Unlike other fuzzing engines, it doesn't work with bitstrings but instead work 
with values of any type `T` directly. The complexity of the inputs and the way
to mutate them is given by functions defined by the user.

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

Then, we add a dependency to `Cargo.toml`:
```toml
[dependencies]
fuzzcheck_input = { git = "https://github.com/loiclec/fuzzcheck-input.git", branch = "master" }
```

`fuzzcheck_input` is a library that contains useful mutators to use
with Fuzzcheck. For now, it only supports integers and vectors, but I intend
to add more generators over time.

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

Note that the `target` key is important. Replace its value with the correct
triple if you are not using macOS.

We can now write the test function and the code that will launch the
fuzzing process in `main.rs`.

We will need to use two dependencies: `fuzzcheck`, `fuzzcheck_input`.
```rust
// main.rs
extern crate fuzzcheck;
use fuzzcheck::fuzzer;

use fuzzcheck_input::integer::*;
use fuzzcheck_input::vector::*;
```

Let's define a function `fn test(input: Vec<u8>) -> bool` that fails under
very specific conditions.

```rust
//main.rs
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
    let u8_gen = IntegerGenerator::<u8>::new();
    let vec_gen = VectorGenerator::new(u8_gen);
    let result = fuzzer::launch(test, vec_gen);
    println!("{:?}", result);
}
```

The first step is to create an `InputGenerator`, which is something that can
determine the complexity of an input, generate a random input, mutate it, as
well as decode it and encode it to a file in order to save the result of the
fuzzing process.

To create an `InputGenerator` that manipulates values of type `Vec<u8>`, I
compose a `VectorGenerator` with an `IntegerGenerator` of `u8`. These
generators are defined in a separate crate called `fuzzcheck_input`.

Then, I call the function `fuzzcheck::fuzzer::launch`, and pass it the test
function and the input generator.

The final step is to compile the executable and use it via the `fuzzcheck`
command line tool.

To compile it, define the environment variable `FUZZCHECK_LIB` to your own
absolute path pointing to `fuzzcheck-rs/target/release/deps`.

```bash
# Do not copy and paste. Replace the path with the correct value for your computer
export FUZZCHECK_LIB="/Users/loiclecrenier/Documents/rust/fuzzcheck-rs/target/release/deps/"
```

And then build with cargo:

```bash
cargo build --release
```

You can launch the fuzzer either by running the executable directly (**not 
recommended, because you won't have access to every feature**):

```bash
cargo run --release
```

or via the `fuzzcheck` executable (if you didn't install it to your $PATH, you
should use the full path to the executable, such as 
`/Users/loiclecrenier/Documents/rust/fuzzcheck/target/release/fuzzcheck`).

```bash
fuzzcheck --target ./target/x86_64-apple-darwin/release/fuzzcheck-test
```

This starts a loop that will stop when a failing test has been found.

A line will be printed whenever a newsworthy event happened, along with some
statistics. For example:

```
NEW     100848  score: 380      pool: 21        exec/s: 133345  cplx: 3162
```

* `NEW` means that a new input was added to the pool of interesting inputs
* `100848` is the number of iterations that were performed so far
* `score: 380` is a measure of the total code coverage caused by all inputs
in the pool
* `pool: 21` is the number of inputs in the pool
* `exec/s: 133345` is the average number of iterations performed every second
* `cplx: 3162` is a measure of the complexity of the inputs in the pool

When a failing test has been found, the following is printed:
```
================ TEST FAILED ================
3696671 score: 2565     pool: 170       exec/s: 71038   cplx: 4050
Saving at "./artifacts/36847bc18a955330.json"
```

Here, the path to the artifact file is `./artifacts/36847bc18a955330.json`. 
It contains a JSON-encoding of the input that failed the test.

```json
[0,78,56,2,76,7,100,102]
```

Moreover, the fuzzer can maintain a copy of its input pool in the file system,
which is located by default at `fuzz-corpus/`. Fuzzing corpora are useful to
kick-start a fuzzing process by providing a list of known interesting inputs.
If you try to run the fuzzer again, you will see that it finds the problematic
input much quicker. This is because it first read the values written inside 
`fuzz-corpus` and used them as starting points. (*Only works out-of-the-box
if the fuzzcheck executable is used.*)

## Minimize

The `fuzzcheck` executable can also be used to *minimize* a large input that
fails the test.

Let's say you have a file `crash.json` containing an input that you would like
to minimize:

```json
[0,78,56,2,76,7,100,102,102,0,0,78,56,2,76,
7,100,102,102,0,234,169,95,18,254,102,81,
41,212,142,0,78,56,2,76,7,100,102,102,0]
```

Launch the `fuzzcheck` executable and use the `minimize` command along with the
required `--input-file` flag and the path to the file.

```bash
fuzzcheck --target "target/x86_64-apple-darwin/release/fuzzcheck-example" minimize --input-file "crash.json"
```

This will repeatedly launch the fuzzer in “minimize” mode and save the
artifacts in the folder `crash.minimized`. The name of each artifact will
be prefixed with the complexity of its input. For example,
`crash.minimized/800--fe958d4f003bd4f5.json` has a complexity of `8.00`.

You can stop the minimizing fuzzer at any point and look for the least complex
input in the artifacts folder.

## Creating an InputGenerator

If you would like to fuzz-test your own custom type, you will have to create
an input generator for it. You can do so by creating a type that conforms to
the `InputGenerator` trait.

```rust
pub trait InputGenerator {
    type Input: Hash + Clone;

    fn complexity(input: &Self::Input) -> f64;
    
    fn new_input(&mut self, max_cplx: f64) -> Self::Input;

    fn mutate(&mut self, input: &mut Self::Input, spare_cplx: f64) -> bool;

    fn from_data(data: &Vec<u8>) -> Option<Self::Input>;
    fn to_data(input: &Self::Input) -> Vec<u8>;
}
```

* `Input` is the type being tested, it must be hashable and cloneable.

* `complexity` returns a float that estimates how “complex” the input is. For
an integer, this might be the number of bytes used to represent it. For an 
array, it might be the sum of complexities of each of its elements.

* `new_input` returns a random input with a smaller complexity than `max_cplx`

* `mutate` mutates the given input without increasing its complexity by more
than `spare_cplx` (otherwise, the fuzzer will ignore the result and skip the
current iteration, which is not too bad but slows down the fuzzer). 
The mutation should ideally be small, but meaningful. For example, it could:
     * append a random element to an array
     * mutate a random element in an array
     * subtract a small constant from an integer
     * change an integer to 0, or its minimum/maximum value
     * replace a substring by a keyword relevant to the test function
     * add a node to a graph data structure, and connect it to a random node

* `from_data` and `to_data` decode/encode the input. For example, a simple 
implementation of `to_data` could be:
  ```rust
  fn to_data(input: &Self::Input) -> Vec<u8> {
      serde_json::to_vec(input).unwrap()
  }
  ```

## Previous work on fuzzing engines

As far as I know, evolutionary, coverage-guided fuzzing engines were
popularized by [American Fuzzy Lop (AFL)](http://lcamtuf.coredump.cx/afl/).
Fuzzcheck is also evolutionary and coverage-guided.

Later on, LLVM released its own fuzzing engine, 
[libFuzzer](https://www.llvm.org/docs/LibFuzzer.html), which is based on the
same ideas as AFL, but it uses Clang’s 
[SanitizerCoverage](https://clang.llvm.org/docs/SanitizerCoverage.html) and is
in-process (it lives in the same process as the program being fuzz-tested
Fuzzcheck is also in-process and also uses SanitizerCoverage.

Both AFL and libFuzzer work by manipulating bitstrings (e.g. `1011101011`).
However, real-world programs work on structured data, and mutations at the
bitstring level may not map to meaningful mutations at the level of the
structured data. This problem can be partially addresses by using a compact
binary encoding such as protobuf and providing custom mutation functions to
libFuzzer that work on the structured data itself. This is called
“structure-aware fuzzing” ([talk](https://www.youtube.com/watch?v=U60hC16HEDY),
[tutorial](https://github.com/google/fuzzer-test-suite/blob/master/tutorial/structure-aware-fuzzing.md)).

Fuzzcheck is also structure-aware, but unlike previous attempts at
structure-aware fuzzing, it doesn't use an intermediary binary encoding such as
protobuf. Instead, it directly works with the typed values in-process.
This is better in at least three ways. First, it is faster because there is no
need to encode and decode inputs at each iteration. Second, the complexity of
the input is given by a user-defined function, which will be more accurate than
counting the bytes of the protobuf encoding. Third, the artifact files and the
fuzzing corpora can be JSON-encoded, which is more user-friendly than protobuf.