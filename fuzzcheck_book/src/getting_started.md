# Getting Started

## Prerequisites

First, remember that you need to run either Linux or macOS, as
[Windows is not yet supported](https://github.com/loiclec/fuzzcheck-rs/issues/8).

Fuzzcheck comes with a helper tool that helps you pass the right flags to 
compile fuzz targets. It is recommended to use it. To install it, run:

```sh
cargo install cargo-fuzzcheck
```

It is also necessary to use Rust nightly. You can install it using `rustup`:
```sh
rustup install nightly
```

## What type of function can I fuzz-test?

### 1) Pure, fast functions

It is best if the tested function is “pure”, meaning that it behaves in exactly
the same way every time it is called with the same input. It is also best if the
tested function runs very fast, ideally in less than a tenth of a millisecond.
This is because fuzzcheck often needs hundreds of thousands of iterations to build
a corpus of interesting test cases.

Functions that perform file or network operations may have unpredictable behaviour
or take too long to run and are thus not a great fit for fuzz-testing.

Note also that the tested function takes an **immutable reference** as argument.
It is important that it does not mutate its argument through types
such as `Cell`, `RefCell`, or `UnsafeCell`.

### 2) No dependence on information that is lost when cloning

Furthermore, be aware that some types lose information when serialised or cloned.
This is the case for `Vec<T>` and its `capacity` property. If the tested function
changes its behaviour based on these kinds of properties, fuzzcheck will be less
useful. For a concrete example, imagine that the tested function is:
```rust
fn test_vector_capacity(v: &Vec<T>) -> bool {
	v.capacity < 10
}
```
Then an empty vector with a capacity of `12` would fail the test. But fuzzcheck
will save this failing test case as:
```json
[]
```
And when the test case is deserialised, the resulting vector may have a capacity
of `0`, and thus will not fail the test anymore. 

Problems also arise when the tested function makes internal decisions based on `capacity`:
```rust
fn test_vector(v: &Vec<T>) -> bool {
	if v.capacity < 10 {
		// do something
	} else {
		// do something else
	}
}
```
Now the problem is more subtle. Imagine that `test_vector` is called on an
empty vector with capacity `12`. Then the second branch will be taken and 
fuzzcheck may judge that the test case is interesting because it explored a 
previously uncovered code region. The test case is thus saved to the internal
pool by cloning. But cloning does *not* preserve the `capacity` property. The
vector that is saved in fuzzcheck’s pool is an “impostor” in some sense. This
may stall the fuzzer’s progress.

<!-- 
## How to add fuzzcheck as a dependency in Cargo.toml

Then, in the Cargo.toml of the crate you want to test, add a development dependency:
```toml
[dev-dependencies]
fuzzcheck = "0.11"
```

Note that the tool `cargo-fuzzcheck` automatically adds the `--cfg fuzzing`
option when compiling a fuzz test. Therefore, you can also choose to import
fuzzcheck as a dependency only when `cfg(fuzzing)` is enabled:
```toml
[target.'cfg(fuzzing)'.dev-dependencies]
fuzzcheck = "0.11"
```

Furthermore, fuzzcheck has a few features that are enabled by default:
* `serde_json_serializer` imports `serde` and `serde_json` to serialise failing
test cases.
* `grammar_mutator` adds the ability to generate abstract syntax trees conforming
to a grammar. It doesn't require additional dependencies but adds a bit of compile time
* `regex_grammar` builds on `grammar_mutator` and makes it possible to specify grammars
using the `regex_syntax` crate

You can depend on fuzzcheck with a minimal set of features to reduce compile times:
```toml
[target.'cfg(fuzzing)'.dev-dependencies]
fuzzcheck = { version = "0.11", default_features = false, features = ["serde_json_serializer"] }
``` -->


## Getting Started

To learn how to use fuzzcheck, go to the [quick start](quick_start.md) section or follow one of the tutorials:
* [Tutorial 1](tutorial1.md) sets up a differential property test between a binary search tree and `std::collections::BTreeSet`. Through it, you can learn about the different parts of the fuzzer and learn to interpret the files generated by it. No bug is directly found in this tutorial, but we guess a potential stack overflow by looking at the content of the generated corpus.
* [Tutorial 2](tutorial2.md) fuzz-test the `pulldown-cmark` crate using a grammar-based mutator. We find test cases that trigger a panic at multiple different locations.