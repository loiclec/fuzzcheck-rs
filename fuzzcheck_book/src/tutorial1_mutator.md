# Mutator

A mutator is a value whose type implements the `Mutator<T>` trait, where 
`T` is any type that is `Clone + 'static`.

While it is possible to hand-write mutators, the easiest way to get one 
is to use the `DefaultMutator` trait and procedural macro.

```rust ignore
use fuzzcheck::DefaultMutator;
#[derive(Clone, DefaultMutator)]
enum Operation<T: Ord> {
    Insert(T),
    Contains(T),
}
```
Now, we can get a mutator of `Operation<T>` by writing, for example:
```rust ignore
fn example() {
    let mutator = Operation::<bool>::default_mutator(); // impl Mutator<Operation<bool>>
    // or for a mutator of `Vec<Operation<bool>>`
    let mutator = Vec::<Operation<bool>>::default_mutator(); // impl Mutator<Vec<Operation<bool>>>
    // ...
}
```
If we have a `Mutator<Vec<bool>>`, then we can use it for any function whose input type
can be obtained by borrowing `Vec<bool>`. So it is valid for:
* `fn test(xs: &Vec<bool>)`
* but also `fn test(xs: &[bool])` because `Vec<bool>` can be borrowed as `[bool]`

