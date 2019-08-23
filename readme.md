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
> or propose an idea. For now, the best way to contribute is to write generators
> for more types of inputs. Or, even better, to write a generator-generator.

Fuzzcheck is a structure-aware, in-process, coverage-guided, evolutionary 
fuzzing engine for Rust functions. 

Its main aim is to be used as the input generator of property-based tests.
Detecting security flaws in an application is a non-goal.

Given a function `test: (T) -> Bool`, it tries to find a value of type `T` that
fails the test or leads to a crash.

Unlike other fuzzing engines, it doesn't work with bitstrings but instead works
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

The first step is to install the `cargo-fuzzcheck` executable using cargo nightly. 

```bash
cargo +nightly install --git https://github.com/loiclec/fuzzcheck-rs
```

Then, somewhere else, create a new cargo library crate. It will contain the
library code that you want to fuzz-test. Also do not forget to set the rust
version to nightly.

```bash
cargo new --lib my_library
cd my_library
rustup override set nightly
```

Then, run `cargo fuzzcheck init` to install the fuzzcheck library and 
initialize a `fuzz` folder that will contain all future fuzz tests.

```
cargo fuzzcheck init
```

An executable script was created at `fuzz/fuzz_targets/target1.rs`. It contains
a basic fuzz test that works with values of type `Vec<u8>`.

```rust
extern crate my_library;

extern crate fuzzcheck;
use fuzzcheck::fuzzer;

extern crate fuzzcheck_input;
use fuzzcheck_input::integer::IntegerGenerator;
use fuzzcheck_input::vector::VectorGenerator;

fn test(input: &Vec<u8>) -> bool {
    // property test goes here
    if 
        input.len() > 7 &&
        input[0] == 0 &&
        input[1] == 167 &&
        input[2] == 200 &&
        input[3] == 103 &&
        input[4] == 56 &&
        input[5] == 78 &&
        input[6] == 2 &&
        input[7] == 254
    {
        false
    }
    else {
        true
    }
}

fn main() {
    let u8_gen = IntegerGenerator::<u8>::new();
    let vec_gen = VectorGenerator::new(u8_gen);
    
    let _ = fuzzer::launch(test, vec_gen);
}
```

Note that while the input is of type `Vec<u8>`, it could equally easily be
anything such as `String`, `HashMap<T, U>`, etc. The example linked at the
beginning of the readme tests a function working with a Graph data structure
defined by a third-party library.

You can already try launching this test: 

```
cargo fuzzcheck run target1
```

This starts a loop that will stop when a failing test has been found.

A line will be printed whenever a newsworthy event happened, along with some
statistics. For example:

```
NEW     180086  score: 493      pool: 48        exec/s: 132713  cplx: 79792
```

* `NEW` means that a new input was added to the pool of interesting inputs
* `180086` is the number of iterations that were performed so far
* `score: 493` is a measure of the total code coverage caused by all inputs
in the pool
* `pool: 48` is the number of inputs in the pool
* `exec/s: 132713` is the average number of iterations performed every second
* `cplx: 79792` is the average complexity of the inputs in the pool

When a failing test has been found, the following is printed:
```
================ TEST FAILED ================
188241  score: 495      pool: 51        exec/s: 132659  cplx: 81373
Saving at "./fuzz/fuzz_targets/target1/artifacts/1c10daa13e9b1721.json"
```

Here, the path to the artifact file is `./fuzz/fuzz_targets/target1/artifacts/1c10daa13e9b1721.json`. 
It contains a JSON-encoding of the input that failed the test.

```json
[0, 167, 200, 103, 56, 78, 2, 254]
```

Moreover, the fuzzer can maintain a copy of its input pool in the file system,
which is located by default at `fuzz_targets/<target>/fuzz-corpus/`. Fuzzing corpora 
are useful to kick-start a fuzzing process by providing a list of known interesting inputs.
If you try to run the fuzzer again, you will see that it finds the problematic input much 
quicker. This is because it first read the values written inside `fuzz-corpus` and used 
them as starting points.

## Minifying failing test inputs

Fuzzcheck can also be used to *minify* a large input that fails a test.

Let's say you have a file `crash.json` containing an input that you would like
to minify:

```json
[0,78,56,2,76,7,100,102,102,0,0,78,56,2,76,
7,100,102,102,0,234,169,95,18,254,102,81,
41,212,142,0,78,56,2,76,7,100,102,102,0]
```

Launch `cargo-fuzzcheck run` on your target with the `tmin` command and an `--input-file` flag.

```bash
cargo fuzzcheck run target1 tmin --input-file "artifacts/crash.json"
```

This will repeatedly launch the fuzzer in “minify” mode and save the
artifacts in the folder `artifacts/crash.minified`. The name of each artifact will
be prefixed with the complexity of its input. For example,
`crash.minified/800--fe958d4f003bd4f5.json` has a complexity of `8.00`.

You can stop the minifying fuzzer at any point and look for the least complex
input in the `crash.minified` folder.

## Creating an InputGenerator

If you would like to fuzz-test your own custom type, you will have to create
an input generator for it. You can do so by creating a type that conforms to
the `InputGenerator` trait.

```rust
pub trait InputGenerator {
    type Input: Clone;

    fn hash<H>(input: &Self::Input, state: &mut H) where H: Hasher;

    fn base_input() -> Self::Input;
    fn complexity(input: &Self::Input) -> f64;
    
    fn new_input(&mut self, max_cplx: f64) -> Self::Input;

    fn mutate(&mut self, input: &mut Self::Input, spare_cplx: f64) -> bool;

    fn from_data(data: &Vec<u8>) -> Option<Self::Input>;
    fn to_data(input: &Self::Input) -> Vec<u8>;
}
```

* `Input` is the type being tested, it must be cloneable.

* `hash` should be implemented in the same way as the `hash` associated function 
of the `Hash` trait.

* `base_input` is the simplest value of type `Input` that you can think of.
It may be `0` for numbers or an empty vector for `Vec`.

* `complexity` returns a float that estimates how “complex” the input is. For
an integer, this might be the number of bytes used to represent it. For an 
array, it might be the sum of complexities of each of its elements.

* `new_input` returns a random input with a complexity smaller than `max_cplx`

* `mutate` mutates the given input without increasing its complexity by more
than `spare_cplx` (otherwise, the fuzzer will ignore the result and skip the
current iteration, which is not too bad but slows it down). 
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
in-process (it lives in the same process as the program being fuzz-tested.  
Fuzzcheck is also in-process and also uses SanitizerCoverage.

Both AFL and libFuzzer work by manipulating bitstrings (e.g. `1011101011`).
However, many programs work on structured data, and mutations at the
bitstring level may not map to meaningful mutations at the level of the
structured data. This problem can be partially addresses by using a compact
binary encoding such as protobuf and providing custom mutation functions to
libFuzzer that work on the structured data itself. This is a way to perform
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

TODO: mention FuzzChick for Coq