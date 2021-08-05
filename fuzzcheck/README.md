# fuzzcheck

Fuzzcheck is a structure-aware and coverage-guided fuzzing engine for Rust 
functions. It works on macOS and linux, x86-64 and aarch64. Windows support is 
[possible, but I need some help to add it](https://github.com/loiclec/fuzzcheck-rs/issues/8).

Given a function `test: (T) -> bool`, you can use fuzzcheck to find a value of
type `T` that fails the test or leads to a crash.

Fuzzcheck works by maintaining a pool of test inputs and ranking them using
the uniqueness of the code coverage caused by running `test(input)`. 
From that pool, it selects a high-ranking input, mutates it, and runs the test
function again. If the new mutated input has an interesting code coverage then
it is added to the pool, otherwise, fuzzcheck tries again with a different 
input and mutation.

In pseudocode:

```rust
loop {
    let input = pool.select();
    mutate(&mut input);

    let analysis = analyze(test, &input);

    match analysis {
        Failed => reportFailure(input),
        Interesting(score) => pool.add(input, score),
        NotInteresting => continue
    }
}
```

Fuzzcheck is unique because, unlike other coverage-guided fuzzing engines, it 
doesn't work with bitstrings but instead works with values of any type `T` 
directly. The complexity of the inputs and the way to mutate them is given by 
functions defined by the user.

## Setup

Rust nightly is required. You can install it with:
```
rustup toolchain install nightly
```

While it is not strictly necessary, installing the `cargo-fuzzcheck` 
executable will make it easier to run fuzzcheck.
```bash
cargo install cargo-fuzzcheck
```

In you `Cargo.toml` file, add `fuzzcheck` as a dev dependency:
```toml
[dev-dependencies]
fuzzcheck = "0.7"
```

Then, we need a way to serialize values. By default, fuzzcheck uses `serde_json` for that purpose (but it can be changed). 
That means our data types should implement serde's traits. In `Cargo.toml`, add:
```
[dependencies]
serde = { version = "1.0", features = ["derive"] }
```

## Usage

Below is an example of how to use fuzz test. Note:
1. every code related to fuzzcheck is conditional on `#[cfg(test)]` because we 
don't want to carry the fuzzcheck dependency in normal builds
2. the `#![cfg_attr(test, feature(no_coverage))]` that is required by fuzzcheck’s procedural macros
3. the use of `derive(fuzzcheck::DefaultMutator)` to make a custom type fuzzable 

```rust
// this nightly feature is required by fuzzcheck’s procedural macros
#![cfg_attr(test, feature(no_coverage))]

// The DefaultMutator macro creates a mutator for a custom type
// The mutator is accessible via SampleStruct::<T, U>::default_mutator()
#[cfg_attr(test, derive(fuzzcheck::DefaultMutator))]
// the fuzzer needs to serialize and deserialize test cases,
// we use serde by default, but that can be changed
#[derive(Clone, Serialize, Deserialize)]
struct SampleStruct<T, U> {
    x: T,
    y: U,
}

#[cfg_attr(test, derive(fuzzcheck::DefaultMutator))]
#[derive(Clone, Serialize, Deserialize)]
enum SampleEnum {
    A(u16),
    B,
    C { x: bool, y: bool },
}

fn should_not_crash(xs: &[SampleStruct<u8, SampleEnum>]) {
    if xs.len() > 3
        && xs[0].x == 100
        && matches!(xs[0].y, SampleEnum::C { x: false, y: true })
        && xs[1].x == 55
        && matches!(xs[1].y, SampleEnum::C { x: true, y: false })
        && xs[2].x == 87
        && matches!(xs[2].y, SampleEnum::C { x: false, y: false })
        && xs[3].x == 24
        && matches!(xs[3].y, SampleEnum::C { x: true, y: true })
    {
        panic!()
    }
}

// fuzz tests reside along your other tests and have the #[test] attribute
#[cfg(test)]
mod tests {
    use super::*;
    use fuzzcheck::{FuzzerBuilder, DefaultMutator, SerdeSerializer};
    #[test]
    fn test_function_shouldn_t_crash() {
        FuzzerBuilder::test(should_not_crash) // first give the function to test
            // second, the mutator to generate the function’s inputs
            .mutator(<Vec<SampleStruct<u8, SampleEnum>>>::default_mutator()) 
            // third, the serializer, which we chose to be based on serde
            .serializer(SerdeSerializer::default())
            // fourth, we take the rest of the arguments from the cargo-fuzzcheck tool
            .arguments_from_cargo_fuzzcheck()
            // finally, tell the fuzzer the files for which code coverage is recorded
            .observe_only_files_from_current_dir()
            // we're now ready to launch the fuzzer!
            .launch()
            //
            // note 1: all these arguments must be given in this specific order
            // the code won't compile otherwise
            //
            // note 2: if this test is run with cargo test, it will simply do nothing
            //
    }
}
```

We can now use `cargo-fuzzcheck` to launch the test, using Rust nightly:
```sh
rustup override set nightly
# first argument is the *exact* path to the test function
# second argument is the action to perform. In this case, "fuzz"
# --artifacts specifies the folder within which to save the failing test cases
cargo fuzzcheck tests::test_function_shouldn_t_crash fuzz --artifacts fuzz/artifacts
```

This starts a loop that will stop when a failing test has been found.

A line will be printed whenever a newsworthy event happened, along with some
statistics. For example:

```
NEW     7825    score: 18.70    pool: 7 exec/s: 728516  cplx: 41.29
```

* `NEW` means that a new input was added to the pool of interesting inputs
* `7825` is the number of iterations that were performed so far
* `score: 18.70` is a measure of the total code coverage caused by all inputs
in the pool
* `pool: 7` is the number of inputs in the pool
* `exec/s: 728516` is the average number of iterations performed every second
* `cplx: 41.29` is the average complexity of the inputs in the pool

When a failing test has been found, the following is printed:
```
================ TEST FAILED ================
13024   score: 20.90    pool: 7 exec/s: 1412576 cplx: 41.29
Saving at "fuzz/artifacts/59886edc1de2dcc1.json"
```

Here, the path to the artifact file is 
`fuzz/artifacts/59886edc1de2dcc1.json`. 
It contains the JSON-encoded input that failed the test.

```json
[
  {
    "x": 100,
    "y": {
      "C": {
        "x": false,
        "y": true
      }
    }
  },
  {
    "x": 55,
    "y": {
      "C": {
        "x": true,
        "y": false
      }
    }
  },
  ..
]
```

Moreover, the fuzzer can maintain a copy of its input pool in the file system
by passing the argument `--out-corpus <folder path>`. Fuzzing corpora 
are useful to kick-start a fuzzing process by providing a list of known 
interesting inputs through the option `--in-corpus <folder path>`.

## Minifying failing test inputs

Fuzzcheck can also be used to *minify* a large input that fails a test.

Let's say you have a file `crash.json` containing an input that you would like
to minify. Launch `cargo fuzzcheck <exact name of fuzz test>` with the `tmin` command
and an `--input-file` option.

```bash
cargo fuzzcheck "tests::test_function_shouldn_t_crash" tmin --input-file "crash.json"
```

This will repeatedly launch the fuzzer in “minify” mode and save the
artifacts in the folder `artifacts/crash.minified`. The name of each artifact 
will be prefixed with the complexity of its input. For example,
`crash.minified/800--fe958d4f003bd4f5.json` has a complexity of `8.00`.

You can stop the minifying fuzzer at any point and look for the least complex
input in the `crash.minified` folder.

## Creating a Mutator

If you would like to fuzz-test your own custom type `T` without the 
`DefaultMutator` derive attribute or the `make_mutator!` procedural macro, 
you will have to create a type that conforms to the `Mutator<T>` trait.

Ask for help in the Github issues or send me an email if you would like
some help or advice on how to write a good mutator.

## Previous work on fuzzing engines

As far as I know, evolutionary, coverage-guided fuzzing engines were
popularized by [American Fuzzy Lop (AFL)](http://lcamtuf.coredump.cx/afl/).  
Fuzzcheck is also evolutionary and coverage-guided.

Later on, LLVM released its own fuzzing engine, 
[libFuzzer](https://www.llvm.org/docs/LibFuzzer.html), which is based on the
same ideas as AFL, but it uses Clang’s 
[SanitizerCoverage](https://clang.llvm.org/docs/SanitizerCoverage.html) and is
in-process (it lives in the same process as the program being fuzz-tested.  
Fuzzcheck is also in-process and also uses SanitizerCoverage.

Both AFL and libFuzzer work by manipulating bitstrings (e.g. `1011101011`).
However, many programs work on structured data, and mutations at the
bitstring level may not map to meaningful mutations at the level of the
structured data. This problem can be partially addressed by using a compact
binary encoding such as protobuf and providing custom mutation functions to
libFuzzer that work on the structured data itself. This is a way to perform
“structure-aware fuzzing” ([talk](https://www.youtube.com/watch?v=U60hC16HEDY),
[tutorial](https://github.com/google/fuzzer-test-suite/blob/master/tutorial/structure-aware-fuzzing.md)).

An alternative way to deal with structured data is to use generators just like
QuickCheck’s `Arbitrary` trait. And then to “treat the raw byte buffer input 
provided by the coverage-guided fuzzer as a sequence of random values and
implement a “random” number generator around it.” 
([cited blog post by @fitzgen](https://fitzgeraldnick.com/2019/09/04/combining-coverage-guided-and-generation-based-fuzzing.html)). 
The tool `cargo-fuzz` has
[recently](https://fitzgeraldnick.com/2020/01/16/better-support-for-fuzzing-structured-inputs-in-rust.html) 
implemented that approach.

Fuzzcheck is also structure-aware, but unlike previous attempts at
structure-aware fuzzing, it doesn't use an intermediary binary encoding such as
protobuf nor does it use Quickcheck-like generators.
Instead, it directly mutates the typed values in-process.
This is better many ways. First, it is faster because there is no
need to encode and decode inputs at each iteration. Second, the complexity of
the input is given by a user-defined function, which will be more accurate than
counting the bytes of the protobuf encoding.
Finally, and most importantly, the mutations are faster and more meaningful 
than those done on protobuf or `Arbitrary`’s byte buffer-based RNG.
A detail that I particularly like about fuzzcheck, and that is possible only 
because it mutates typed values, is that every mutation is done **in-place**
and is reversable. That means that generating a new test case is super fast, 
and can often even be done with zero allocations.

As I was developing Fuzzcheck for Swift, a few researchers developed Fuzzchick
for Coq ([paper](https://www.cs.umd.edu/~mwh/papers/fuzzchick-draft.pdf)). It 
is a coverage-guided property-based testing tool implemented as an extension to
Quickchick. As far as I know, it is the only other tool with the same philosophy
as fuzzcheck. The similarity between the names `fuzzcheck` and `Fuzzchick` is a 
coincidence.
