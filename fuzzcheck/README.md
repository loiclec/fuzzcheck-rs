# Fuzzcheck (note: still experimental)

Fuzzcheck is a structure-aware, in-process, coverage-guided, evolutionary 
fuzzing engine for Rust functions. 

Its main aim is to be used as the input generator of property-based tests.

Given a function `test: (T) -> Bool`, it tries to find a value of type `T` that
fails the test or leads to a crash.

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

Unlike other coverage-guided fuzzing engines, it doesn't work with bitstrings 
but instead works with values of any type `T` directly. The complexity of the 
inputs and the way to mutate them is given by functions defined by the user.

## Note

Please contact me if you have questions/comments or if you would like to try it
but don't know where to start. As far as I know, I am the only one who has ever
tried to use it, so I am quite curious to hear how easy/difficult it is for 
others to pick it up.

I would also *love* to find a contributor/maintainer, because I don't have much
time to spend on it except during school breaks. So if you would like to work 
on a fast, novel fuzzing engine specifically made for Rust, please talk to me! 
I can guide you through the design and code and suggest tasks to get started,
there are many.

## Usage

The first step is to install the `cargo-fuzzcheck` executable using cargo nightly. 

```bash
cargo +nightly install cargo-fuzzcheck
```

Then, somewhere else, create a new cargo crate. It will contain the
library code that you want to fuzz-test. Also do not forget to set the rust
version to nightly.

```bash
cargo new --lib my_library
cd my_library
rustup override set nightly
```

Then, run `cargo fuzzcheck init` to initialize a `fuzz` folder that will
contain all future fuzz tests.

```
cargo fuzzcheck init
```

A sample test function was created at `fuzz/instrumented/src/lib.rs`.
It contains this basic fuzz test, which you can replace with a test for your 
library. Note that while the input is of type `Vec<u8>`, it could be anything
else such as `String`, `HashMap<T, U>`, etc. so long as an appropriate mutator
has been written for it.

```rust
extern crate my_library;

pub fn test(input: &[u8]) -> bool {
    // test goes here
    if 
        input.len() > 14 &&
        input[0] == 0 &&
        input[1] == 167 &&
        input[2] == 200 &&
        input[3] == 103 &&
        input[4] == 56 &&
        input[5] == 78 &&
        input[6] == 2 &&
        input[7] == 254 &&
        input[8] == 0 &&
        input[9] == 167 &&
        input[10] == 200 &&
        input[11] == 103 &&
        input[12] == 56 &&
        input[13] == 78 &&
        input[14] == 103
    {
        false
    }
    else {
        true
    }
}
```

And an executable script was created at 
`fuzz/non_instrumented/fuzz_targets/target1.rs`. It launches the fuzzing engine
on the above test function using the mutator `VecMutator<U8Mutator>`.
Both `VecMutator` and `U8Mutator` are provided by fuzzcheck. However, more
mutators can be created for values of any type.

```rust
/* Various import statements not included in this example */

// Makes the SerdeSerializer available to fuzzcheck
define_serde_serializer!();

fn main() {
    // Will mutate values of type Vec<u8>
    let mutator = VecMutator<U8Mutator>::default();
    // Test inputs will be encoded with serde_json 
    let serializer = SerdeSerializer::<Vec<u8>>::default();
    // Launch the fuzzing process on the test function
    let _ = fuzzcheck::launch(test, mutator, serializer);
}
```

You can already try launching this test: 

```
cargo fuzzcheck run target1 fuzz
```

This starts a loop that will stop when a failing test has been found.

A line will be printed whenever a newsworthy event happened, along with some
statistics. For example:

```
NEW     221525  score: 170      pool: 16        exec/s: 4381081 cplx: 1172500
```

* `NEW` means that a new input was added to the pool of interesting inputs
* `221525` is the number of iterations that were performed so far
* `score: 170` is a measure of the total code coverage caused by all inputs
in the pool
* `pool: 16` is the number of inputs in the pool
* `exec/s: 4381081` is the average number of iterations performed every second
* `cplx: 117.25` is the average complexity of the inputs in the pool

When a failing test has been found, the following is printed:
```
================ TEST FAILED ================
270134  score: 170      pool: 16        exec/s: 4381081 cplx: 117.25
Saving at "fuzz/non_instrumented/fuzz_targets/target1/artifacts/b62fcaf08890a875.json"
```

Here, the path to the artifact file is 
`fuzz/non_instrumented/fuzz_targets/target1/artifacts/b62fcaf08890a875.json`. 
It contains a JSON-encoding of the input that failed the test.

```json
[0,167,200,103,56,78,2,254,0,167,200,103,56,78,103]
```

Moreover, the fuzzer can maintain a copy of its input pool in the file system,
which is located by default at `fuzz_targets/<target>/fuzz-corpus/`. Fuzzing corpora 
are useful to kick-start a fuzzing process by providing a list of known interesting inputs.
If you try to run the fuzzer again, you will see that it finds the problematic input much 
quicker. This is because it first read the values written inside `fuzz-corpus` and used 
them as starting points.

## Structure of the fuzz folder

The fuzz folder is a bit difficult to understand, because fuzzcheck needs to 
compile the crate and the fuzz test in two different ways. This is why it 
contains an `instrumented` and a `non-instrumented` folder. 

The `instrumented` folder contains all the test functions and their helper 
functions. It can use your library as a dependency but not `fuzzcheck` 
or `non_instrumented`. Every piece of code written there will be instrumented
such that its code coverage can be recorded.

The `non-instrumented` folder contains the code that launches the fuzzer 
(called the `fuzz_targets`) as well as eventual custom `Mutator` 
implementations. It uses your library, `fuzzcheck`, and `instrumented` as 
dependencies. The code there is not instrumented.

```
.
├── Cargo.toml
├── fuzz                          # everything inside `fuzz` is to be used by fuzzcheck
│  ├── instrumented               # a crate that contains the test functions
│  │  ├── Cargo.lock
│  │  ├── Cargo.toml
│  │  └── src
│  │     └── lib.rs
│  └── non_instrumented           # a crate that launches the fuzzer on specific test functions
│     ├── build.rs
│     ├── Cargo.lock
│     ├── Cargo.toml
│     ├── fuzz_targets
│     │  ├── target1
│     │  ├── target2.rs           # a fuzz-test
│     │  ├── target2
│     │  └── target1.rs           # another fuzz-test
│     └── src
│        └── lib.rs               # contains code that the fuzzer needs, such as custom mutators
└── src
   └── lib.rs                     # your library code
```

Note that if `instrumented` and `non_instrumented` both depend on a common 
crate `A`, then that crate will be compiled twice and the two versions of it
will live in the resulting binary. These two versions will have different,
incompatible versions of the types and traits defined by `A`.

## Minifying failing test inputs

Fuzzcheck can also be used to *minify* a large input that fails a test.

Let's say you have a file `crash.json` containing an input that you would like
to minify:

```json
[0,78,56,2,76,7,100,102,102,0,0,78,56,2,76,
7,100,102,102,0,234,169,95,18,254,102,81,
41,212,142,0,78,56,2,76,7,100,102,102,0]
```

Launch `cargo-fuzzcheck run` on your target with the `tmin` command and an
`--input-file` option.

```bash
cargo fuzzcheck run target1 tmin --input-file "artifacts/crash.json"
```

This will repeatedly launch the fuzzer in “minify” mode and save the
artifacts in the folder `artifacts/crash.minified`. The name of each artifact 
will be prefixed with the complexity of its input. For example,
`crash.minified/800--fe958d4f003bd4f5.json` has a complexity of `8.00`.

You can stop the minifying fuzzer at any point and look for the least complex
input in the `crash.minified` folder.

## Creating a Mutator

If you would like to fuzz-test your own custom type, you will have to create
a `Mutator` for it. You can do so by creating a type that conforms to
the `Mutator` trait.

```rust
pub trait Mutator: Sized {
    type Value: Clone;
    type Cache: Clone;
    type MutationStep;
    type UnmutateToken;

    /// Compute the cache for the given value
    fn cache_from_value(&self, value: &Self::Value) -> Self::Cache;
    /// Compute the initial mutation step for the given value
    fn mutation_step_from_value(&self, value: &Self::Value) -> Self::MutationStep;

    /// The maximum complexity of an input of this type
    fn max_complexity(&self) -> f64;
    /// The minimum complexity of an input of this type
    fn min_complexity(&self) -> f64;
    /// The complexity of the current input
    fn complexity(&self, value: &Self::Value, cache: &Self::Cache) -> f64;

    /// Create an arbitrary value
    fn arbitrary(&self, seed: usize, max_cplx: f64) -> (Self::Value, Self::Cache);

    /// Mutate the given value in-place and return a token describing how to reverse the mutation
    fn mutate(&self, value: &mut Self::Value, cache: &mut Self::Cache, step: &mut Self::MutationStep, max_cplx: f64) -> Self::UnmutateToken;

    /// Reverse a mutation
    fn unmutate(&self, value: &mut Self::Value, cache: &mut Self::Cache, t: Self::UnmutateToken);
}

```

This trait can be a bit difficult to implement, but it is very powerful and it
is possible to write efficient and composable mutators with it. For 
example, fuzzcheck implements `U8Mutator` (u8), `OptionMutator` (Option), and
`VecMutator` (Vec). They compose such that it possible to use a 
`VecMutator<VecMutator<OptionMutator<U8Mutator>>>` to fuzz values of type 
`Vec<Vec<Option<u8>>>`.

I would like to write a guide to fuzzcheck to explain the trait and how to work
with it. But in the meantime, if you have questions, please send me an email or
create an issue on GitHub. You can also look at the documentation of the trait
that explains some of the design decisions behind it.

My goal is to write more mutators for common types and building blocks for 
composability such that a custom implementation of `Mutator` is rarely 
needed.

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
