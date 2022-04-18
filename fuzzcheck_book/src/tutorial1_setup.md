# Setup
First, create a new empty crate using a library template:

```
cargo new --lib example_1
cd example_1
```

And set the default version of the rust compiler for this crate to nightly;
```
rustup override set nightly
```

In `Cargo.toml`, add the following dependencies:
```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }

[dev-dependencies]
fuzzcheck = "0.12"
```

At the top of `src/lib.rs` add the following line:
```rust ignore
#![cfg_attr(fuzzing, feature(no_coverage))]
```
This is a rust-nightly feature that fuzzcheck’s procedural macros use.

Now let's write some code to test. In `src/lib.rs`, we will add a simple implementation of a binary search tree. 

```rust ignore
pub struct Node<T: Ord> {
    key: T,
    left: Box<Tree<T>>,
    right: Box<Tree<T>>,
}
pub enum Tree<T: Ord> {
    Node(Node<T>),
    Empty,
}
```

And we implement the methods `empty`, `insert`, and `contains`.

```rust ignore
use std::cmp::{self, Ordering};

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
```

We already have enough to start fuzz-testing our tree. To do so, we first add a `tests` module
within the same `src/lib.rs` file, which we conditionally enable only for test builds with `#[cfg(test)]`.
```rust ignore
#[cfg(test)]
mod tests {
    use super::*;
    // Note that fuzzcheck is accessible here because it is a dev-dependency.
}
```