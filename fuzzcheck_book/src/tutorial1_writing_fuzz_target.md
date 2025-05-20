# Writing the Fuzz Target

To create the fuzz test, create a new `#[test]` function in the `tests` module and call
`fuzzcheck::fuzz_test(compare_with_btree_set::<u16>)` inside it. Note that only concrete
functions can be tested, not generic ones, which is why we needed to specify a 
type parameter using `::<u16>`.
```rust ignore
#[test]
fn fuzz_test_tree() {
    fuzzcheck::fuzz_test(compare_with_btree_set::<u16>)
        // ... we now have a FuzzerBuilder1<..>
        // we still need to pass more arguments to it
        // before calling .launch()
}
```
We have specified which function to fuzz test, but not which mutator, serializer, sensor, or pool
to use. We can add all these parameters as follows:

```rust ignore
fn fuzz_test_tree() {
    fuzzcheck::fuzz_test(compare_with_btree_set::<u16>) // : FuzzerBuilder1<..>
        .default_mutator() // : FuzzerBuilder2<..>
        .serde_serializer() // : FuzzerBuilder3<..>
        .default_sensor_and_pool() // : FuzzerBuilder4<..>
}
```
And finally, we need to give fuzzcheck other arguments such as:
* does it run a regular fuzz-test or try to minize a failing test case?
* how complex can the generated values be?
* does it stop after the first test failure, or try to find more afterwards?
* should it maintain a copy of the pool on the file system? if so, where?
* etc.

The easiest way to do that is to let `cargo-fuzzcheck` pass these arguments for you. 
So we simply call `.arguments_from_cargo_fuzzcheck()`. Once that is done, we can 
finally call `.launch()`, which returns a `FuzzingResult`. The result contains
two fields: `found_test_failure` and `reason_for_stopping`.

```rust ignore
#[test]
fn fuzz_test_tree() {
    let result = fuzzcheck::fuzz_test(compare_with_btree_set::<u16>) // : FuzzerBuilder1<..>
        .default_mutator() // : FuzzerBuilder2<..>
        .serde_serializer() // : FuzzerBuilder3<..>
        .default_sensor_and_pool() // : FuzzerBuilder4<..>
        .arguments_from_cargo_fuzzcheck() // : FuzzerBuilder5<..>
        .launch(); // FuzzingResult
    assert!(!result.found_test_failure);
}
```
Since all those options are very common, there is a shortcut for them, called `.default_options()`.
The code above is equal to the one below:
```rust ignore
#[test]
fn fuzz_test_tree() {
    let result = fuzzcheck::fuzz_test(compare_with_btree_set::<u16>) // : FuzzerBuilder1<..>
        .default_options() // FuzzerBuilder5<..>
        .launch(); // FuzzingResult
    assert!(!result.found_test_failure);
}
```

<details>

<summary> Click here to reveal a recap of the code we have written so far. </summary>

```rust ignore
#![cfg_attr(fuzzing, feature(coverage_attribute))]
use std::cmp::{self, Ordering};
pub struct Node<T: Ord> {
    key: T,
    left: Box<Tree<T>>,
    right: Box<Tree<T>>,
}
pub enum Tree<T: Ord> {
    Node(Node<T>),
    Empty,
}
impl<T: Ord> Tree<T> {
    /// Creates a new empty tree
    pub fn empty() -> Box<Self> {
        Box::new(Tree::Empty)
    }
    /// Returns true iff the tree contains the given value.
    pub fn contains(&self, k: &T) -> bool {
        match self {
            Tree::Node(Node { key, left, right }) => match k.cmp(key) {
                Ordering::Less => left.contains(k),
                Ordering::Equal => true,
                Ordering::Greater => right.contains(k),
            },
            Tree::Empty => false,
        }
    }
    /**
        Adds a value to the set.

        If the set did not have this value present, true is returned.
        If the set did have this value present, false is returned, and the tree is left unmodified.
    */
    pub fn insert(self: &mut Box<Self>, new_key: T) -> bool {
        match self.as_mut() {
            Tree::Node(Node {
                key,
                left,
                right,
            }) => match new_key.cmp(key) {
                Ordering::Less => {
                    left.insert(new_key)
                }
                Ordering::Equal => false,
                Ordering::Greater => {
                    right.insert(new_key)
                }
            },
            Tree::Empty => {
                **self = Tree::Node(Node {
                    key: new_key,
                    left: Self::empty(),
                    right: Self::empty(),
                });
                true
            }
        }
    }
}
#[cfg(all(fuzzing, test))]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use fuzzcheck::DefaultMutator;
    use serde::{Serialize, Deserialize};

    /// The API of our tree, represented as an enum
    #[derive(Clone, DefaultMutator, Serialize, Deserialize)]
    enum Operation<T: Ord> {
        Insert(T),
        Contains(T),
    }
    /// A test function 
    fn compare_with_btree_set<T: Ord + Clone>(operations: &[Operation<T>]) {
        let mut tree = Tree::empty();
        let mut set = BTreeSet::new();
        for op in operations {
            match op {
                Operation::Insert(x) => {
                    let a = tree.insert(x.clone());
                    let b = set.insert(x.clone());
                    assert_eq!(a, b);
                }
                Operation::Contains(x) => {
                    let a = tree.contains(x);
                    let b = set.contains(x);
                    assert_eq!(a, b);
                }
            }
        }
    }

    // the actual fuzz target, which launches the fuzzer
    #[test]
    fn fuzz_test_tree() {
        let result = fuzzcheck::fuzz_test(compare_with_btree_set::<u16>)
            .default_options()
            .launch();
        assert!(!result.found_test_failure);
    }
}
```

</details>

The next section discusses how to run `fuzz_test_tree` using `cargo fuzzcheck`.
