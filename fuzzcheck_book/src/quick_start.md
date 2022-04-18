# Quick Start

## Setup

* You can only use fuzzcheck on Linux or macOS ([Windows support is possible but I need help with it](https://github.com/loiclec/fuzzcheck-rs/issues/8)) 
* Install `cargo-fuzzcheck`
    ```sh
    cargo install cargo-fuzzcheck
    ```
* Make sure you are using Rust nightly 

* Add the following to `Cargo.toml`:
 ```toml
 [target.'cfg(fuzzing)'.dev-dependencies]
 fuzzcheck = "0.12"
 ```

* Add `serde = { version = "1.0", features = ["derive"] }` to your dependencies as well
    * **note:** fuzzcheck has a serde dependency only when the `serde_json_serializer` feature is enabled. This feature is enabled by default, but 
    you can write the following in your `Cargo.toml` to disable it:
    ```toml
    fuzzcheck = { version = "0.12", default-features = false }
    ```


* Add `#![cfg_attr(fuzzing, feature(no_coverage))]` at the top of the root module (e.g. `src/lib.rs` for a library target)
    * fuzzcheck’s procedural macros use the `no_coverage` attribute

* In your library, integration test, or executable, create a test module, gated by `#[cfg(all(fuzzing, test))]` and add a `#[test]` function inside it
    ```rust ignore
    #[cfg(all(fuzzing, test))]
    mod tests {
        #[test]
        fn my_fuzz_test() {
            // ...
        }
    }
    ```
    We will write its body later on.

* Create a test function of type `Fn(&T)` or `Fn(&T) -> bool` and put the code you want to test inside it.
You want this function to always succeed if the code is correct.
  ```rust ignore
  // for example
  fn should_always_succeed(x: &SomeType) -> bool {
      // ...
  }
  ```

## Mutator

The [`Mutator`](https://docs.rs/fuzzcheck/0.12.0/fuzzcheck/trait.Mutator.html) is responsible for generating values to feed to the test function.

The easiest way to get a mutator for a type is to use the `#[derive(fuzzcheck::DefaultMutator)]` attribute. Many `std` types also implement `DefaultMutator`.

<details>

<summary>Click here to reveal a code snippet showing a use of `DefaultMutator`</summary>

  ```rust ignore
  // example
  use fuzzcheck::DefaultMutator;
  #[derive(Clone, DefaultMutator)]
  pub struct SomeType<A, B: SomeTrait> {
      x: Option<A>,
      y: bool,
      z: Vec<Option<SomeOtherType<B>>>
  }
  #[derive(Clone, DefaultMutator)]
  pub enum SomeOtherType<T> where T: SomeTrait {
      A,
      B { x: bool, y: Box<T> }
  }
  ```

</details>

* If the test case is a `String`, you have two options:
    * Use the default mutator for `String`, which is essentially a wrapper around a `Vec<u8>` mutator and thus doesn't often generate useful strings
    * Use a grammar-based mutator (see [tutorial 2](tutorial2.md) or the [`fuzzcheck::mutators::grammar` module documentation](https://docs.rs/fuzzcheck/0.12.0/fuzzcheck/mutators/grammar/index.html))

* If the argument is a recursive type, you will need to use the `make_mutator!` macro (see the [`fuzzcheck::mutators::recursive` module documentation](https://docs.rs/fuzzcheck/0.12.0/fuzzcheck/mutators/recursive/index.html))

* otherwise, you may need to write the mutator yourself
    * or ask me to do it on GitHub, if it's a type from `std`

## Serializer

You need a way to save the generated test cases to the file system. This is most easily done 
with the `fuzzcheck::SerdeSerializer` type if the test case implements `Serialize` and `Deserialize`:
```rust ignore
use serde::{Serialize, Deserialize};
#[derive(/*..*/, Serialize, Deserialize)]
struct Foo { /* .. */}
```

But there are other choices: 
* `ByteSerializer` if the test case is `Vec<u8>` and we simply want to copy the bytes from/to the files
* `StringSerializer` if the test case is something that implements `FromStr` and `ToString`
* [your own serializer](https://docs.rs/fuzzcheck/0.12.0/fuzzcheck/trait.Serializer.html)

## Sensor and Pool

TODO (but you probably don't need to worry about it to get started)

In the meantime, you can look at the [`CodeCoverageSensorAndPoolBuilder` documentation](https://docs.rs/fuzzcheck/0.12.0/fuzzcheck/builder/struct.CodeCoverageSensorAndPoolBuilder.html), and the [`fuzzcheck::sensors_and_pools` module documentation](https://docs.rs/fuzzcheck/0.12.0/fuzzcheck/sensors_and_pools/index.html).

## Building the fuzz test

We go back to the `#[test]` fuzz function that we defined earlier and write its body.

If you used:
1. the `DefaultMutator` derive attribute, or any other type that has a default mutator 
2. `serde` to serialize the test cases

Then you can write:
```rust ignore
    #[cfg(all(fuzzing, test))]
    mod tests {
        #[test]
        fn my_fuzz_test() {
            let result = fuzzcheck::fuzz_test(should_always_succeed) // the name of the function to test
                .default_options()
                .launch();
            assert!(!result.found_test_failure);
        }
    }
```

Otherwise, you can specify each component separately. Check the [`fuzzcheck::builder` module documentation](https://docs.rs/fuzzcheck/0.12.0/fuzzcheck/builder/index.html) to learn about the different options available.
```rust ignore
    #[cfg(all(fuzzing, test))]
    mod tests {
        #[test]
        fn my_fuzz_test() {
            let mutator = /* ... */;
            let serializer = /* ... */;
            let (sensor, pool) = /* .. */;
            let _ = fuzzcheck::fuzz_test(should_always_succeed) // the name of the function to test
                .mutator(my_mutator) // or .default_mutator() for the default one
                .serializer(my_serializer) // or .serde_serializer() for the default one
                .sensor_and_pool(sensor, pool) // or .default_sensor_and_pool() for the default ones
                .arguments_from_cargo_fuzzcheck() // take the other arguments from the `cargo fuzzcheck` invocation
                .launch();
        }
    }
```

## Launching the fuzz test

On the command line, with the current directory at the root of your crate, run `cargo fuzzcheck <args..>` to
launch the fuzz test. The mandatory arguments are:

* the target to compile, i.e. where is the `#[test]` function located?
    * the package’s library → add `--lib` or nothing
    * an integration test → add `--test <NAME>`
    * an executable → add `--bin <NAME>`

* the **exact path to the test function**
    ```sh
    # example
    cargo fuzzcheck tests::my_fuzz_test
    ```

That's it for the essential arguments. You can run `cargo fuzzcheck --help` for a list of all possible arguments.

Once you run the command, the crate will be compiled and the fuzz test will start.

## Terminal output

When the fuzz test is running, a line is printed after every notable event. It looks like this:

```sh
<time> <iter nbr> <pool_name>(<pool_stats>)... failures(..) iter/s <N>
```
where:
* `time` is the time elapsed since the start
* `iter nbr` is the number of iterations performed so far
* `pool_name(pool_stats)` are statistics about the pool
* `failures(..)` is the number of test failures found
* `iter/s` is the number of iterations performed every second

## File System Output

The fuzzer creates files under the `fuzz/<fuzz_test>` folder. Each pool used by the fuzzer maintains
a copy of its content under `fuzz/<fuzz_test>/corpus/<pool_name>`. In particular, failing test cases
can be found at `fuzz/<fuzz_test>/corpus/test_failures`.

There is also a folder called `stats`, which saves information about each fuzzing run and can be read
by other tools for analysis.
