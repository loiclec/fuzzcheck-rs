# Fuzzcheck

> I made Fuzzcheck in my free time during my summer vacation. It is a much
> improved port of FuzzCheck for Swift, which I wrote a year ago.
> There are many, many ways in which it could be improved such that it
> becomes a powerful, easy-to-use tool for any Rust programmer. I would love 
> to keep working on it, but I am now back to university, and I
> find it hard to justify spending a significant amount of time on it.
> If you would like me to keep developing Fuzzcheck or help your company use
> it, please hire me and help me pay for my studies :)

Fuzzcheck is a structure-aware, in-process, coverage-guided, evolutionary 
fuzzing engine for Rust functions. 

Its main aim is to be used as the input generator of property-based tests.
Detecting security flaws in an application is a non-goal.

Given a function `test: (T) -> Bool`, it tries to find a value of type `T` that
fails the test or leads to a crash.

Unlike other coverage-guided fuzzing engines, it doesn't work with bitstrings 
but instead works with values of any type `T` directly. The complexity of the 
inputs and the way to mutate them is given by functions defined by the user.

Fuzzcheck works by maintaining a pool of test inputs and ranking them using the
complexity of the input and the uniqueness of the code coverage caused by 
`test(input)`. From that pool, it selects a high-ranking input, mutates it, and
runs the test function again. If the new mutated input has an interesting code 
coverage then it is added to the pool, otherwise, Fuzzcheck tries again with a 
different input and mutation.

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

## Usage

The first step is to install the `cargo-fuzzcheck` executable using cargo nightly. 

```bash
cargo +nightly install --git https://github.com/loiclec/fuzzcheck-rs
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
else such as `String`, `HashMap<T, U>`, etc. so long as an appropriate input
handler has been written for it.

```rust
extern crate my_library;

pub fn test(input: &Vec<u8>) -> bool {
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
on the above test function using the input handler `FuzzedVector<FuzzedU8>`.
Both `FuzzedVector` and `FuzzedU8` are provided by fuzzcheck. However, more
handlers can be created for values of any type.

```rust
extern crate fuzzcheck;
use fuzzcheck::fuzzer;

extern crate fuzzcheck_input;
use fuzzcheck_input::integer::*;
use fuzzcheck_input::vector::*;

extern crate my_library_instrumented_fuzz;
use my_library_instrumented_fuzz::test;

fn main() {
    let _ = fuzzer::launch::<_, FuzzedVector<FuzzedU8>>(test);
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
* `cplx: 1172500` is the average complexity of the inputs in the pool

When a failing test has been found, the following is printed:
```
================ TEST FAILED ================
270134  score: 170      pool: 16        exec/s: 4381081 cplx: 1172500
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
(called the `fuzz_targets`) as well as eventual custom `FuzzedInput` 
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
│        └── lib.rs               # contains code that the fuzzer needs, such as custom input handlers
└── src
   └── lib.rs                     # your library code
```

Honestly it is a bit convoluted and I think it may break under certain 
circumstances, I will have to test and reevaluate that approach. Maybe special
compiler support will be needed to properly support fuzzcheck’s use case.

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

## Creating an input handler

If you would like to fuzz-test your own custom type, you will have to create
an input handler for it. You can do so by creating a type that conforms to
the `FuzzedInput` trait.

```rust
pub trait FuzzedInput {
    type Value: Clone;
    type State: Clone;
    type UnmutateToken;

    fn default() -> Self::Value;

    fn state_from_value(value: &Self::Value) -> Self::State;

    fn arbitrary(seed: usize, max_cplx: f64) -> Self::Value;

    fn max_complexity() -> f64;
    fn min_complexity() -> f64;

    fn hash_value<H: Hasher>(value: &Self::Value, state: &mut H);

    fn complexity(value: &Self::Value, state: &Self::State) -> f64;

    fn mutate(value: &mut Self::Value, state: &mut Self::State, max_cplx: f64) -> Self::UnmutateToken;

    fn unmutate(value: &mut Self::Value, state: &mut Self::State, t: Self::UnmutateToken);

    fn from_data(data: &[u8]) -> Option<Self::Value>;
    fn to_data(value: &Self::Value) -> Vec<u8>;
}
```

This trait can be a bit difficult to implement, but it is very powerful and it
is possible to write efficient and composable input handlers with it. For 
example, fuzzcheck implements `FuzzedU8` (u8), `FuzzedOption` (Option), and
`FuzzedVector` (Vec). They compose such that it possible to write 
`FuzzedVector<FuzzedVector<FuzzedOption<FuzzedU8>>>` to fuzz values of type 
`Vec<Vec<Option<u8>>>`.

I would like to write a guide to fuzzcheck to explain the trait and how to work
with it. But in the meantime, if you have questions, please send me an email or
create an issue on GitHub. 

My goal is to write more handlers for common types and building blocks for 
composability such that a custom implementation of `FuzzedInput` is rarely 
needed.

An input handler is responsible for computing the complexity of an input, to
encode and decode it to the file system, and to generate new inputs by mutating
existing ones.

Many of the functions operate on both an input value (e.g. `Vec<T>`) and 
a _state_ associated with it (`e.g. FuzzedVectorState<S>`). The state might hold
the complexity of the value, its previous mutations, or any other thing to help
implement the functions of `FuzzedInput`. 

Note the `unmutate` function, it is responsible for reversing the mutation done by
a previous call of the `mutate` function. The implication is that it is possible
to create new test inputs _in place_. For example, the `FuzzedVector` handler can
mutate a single element at index `i` , without cloning the vector first, and then
reverse that mutation after the test function has been run, therefore generating
new test inputs with zero allocations.

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

Fuzzcheck is also structure-aware, but unlike previous attempts at
structure-aware fuzzing, it doesn't use an intermediary binary encoding such as
protobuf nor does it use Quickcheck-like generators.
Instead, it directly mutates the typed values in-process.
This is better many ways. First, it is faster because there is no
need to encode and decode inputs at each iteration. Second, the complexity of
the input is given by a user-defined function, which will be more accurate than
counting the bytes of the protobuf encoding. Third, the artifact files and the
fuzzing corpora can be JSON-encoded, which is more user-friendly than protobuf.
Finally, and most importantly, the mutations are faster and more meaningful 
than those done on protobuf or `Arbitrary`’s byte buffer-based RNG.

As I was developing Fuzzcheck for Swift, a few researchers developed Fuzzchick
for Coq ([paper](https://www.cs.umd.edu/~mwh/papers/fuzzchick-draft.pdf)). It 
is a coverage-guided property-based testing tool implemented as an extension to
Quickchick. As far as I know, it is the only other tool with the same philosophy
as Fuzzcheck. The similarity between the names `Fuzzcheck` and `Fuzzchick` is a 
coincidence.
