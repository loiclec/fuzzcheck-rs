# Test Function

Currently, a test function can be of two different types:
* `fn foo(input: &T)` always succeeds unless it crashes
* `fn foo(input: &T) -> bool` fails if it return false

For this example, we will write a function of the first kind.

Since our tree acts like a set, we want to compare its behaviour to a correct set implementation, such
as `BTreeSet`. The idea is to start with two empty sets of type `Tree` and `BTreeSet`, apply some 
operations to them, and check that the result of each operation is the same for both of them. Our tree 
API consists of two methods, which we can model as:
```rust ignore
#[cfg(all(fuzzing, test))]
mod tests {
    use super::*;

    enum Operation<T: Ord> {
        Insert(T),
        Contains(T),
    }
}
```
The test function takes as input an arbitrary list of operations and applies them to both structures.
```rust ignore
#[cfg(all(fuzzing, test))]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    enum Operation<T: Ord> {
        Insert(T),
        Contains(T),
    }
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
}
```
If `compare_with_btree_set` never crashes, no matter the list of operations
that is passed to it, then we can gain _some_ confidence that our 
implementation is correct.

The next few sections will explain the different components that fuzzcheck
needs in order to fuzz test `compare_with_btree_set`.
