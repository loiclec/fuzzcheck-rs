# Fuzzing

We know have everything needed to fuzz-test `pulldown-cmark`.

Inside the `tests` module in `src/lib.rs`, add the following `#[test]` function:

```rust ignore
use fuzzcheck::mutators::grammar;

#[test]
fn fuzz() {
    let mutator = grammar::grammar_based_ast_mutator(markdown()).with_string();
    let result = fuzzcheck::fuzz_test(|(_, string): &(AST, String)| {
            push_html_does_not_crash(string)
        })
        .mutator(mutator)
        .serde_serializer()
        .default_sensor_and_pool()
        .arguments_from_cargo_fuzzcheck()
        .launch();
    assert!(!result.found_test_failure);
}
```

We launch the fuzz test using:
```
cargo fuzzcheck tests::fuzz
```

Updates about the fuzzing progress will be printed, such as:
```
7s + 68782 simplest_cov(292 cov: 1937/2617 cplx: 31.75) diverse_cov_20(1904) diverse_cov_1(1514) max_each_cov_hits(501 sum: 8601361) max_total_cov_hits(5799778) failures(3) iter/s 10071
```
After about a minute, fuzzcheck should have found at least two distinct test failures:

They are stored in the `fuzz/<fuzz_test>/corpus/test_failures/` folder with the following structure:
```yaml
- tests::fuzz/corpus/test_failures
    # each folder here corresponds to a distinct test failure (e.g. different panic locations)
    - 5460776198281478584
        # each folder here corresponds to a test case complexity
        - 26.0000
            # this is an actual failing test case, of complexity 26.0
            - 3d3294fac0c0e5e4.json
            - c780161f12e87030.json
        - 54.0000
            - etc.
```
The `.json` file contain both the serialized syntax trees and the markdown string. For example,
`3d3294fac0c0e5e4.json` is:
```json
[
    ">\t<g\tO\n",
    { /* serialized AST */ }
]
```
We can debug this string by making a new test function:

```rust ignore
#[test]
fn reproduce_crash() {
    let s = ">\t<g\tO\n";
    push_html_does_not_crash(s);
}
```

Running the test gives us a panic message:
```sh
thread 'tests::reproduce_crash' panicked at 'range start index 7 out of range for slice of length 5', src/scanners.rs:871:17
```