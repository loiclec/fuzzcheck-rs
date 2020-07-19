/**
 A [Mutator] is an object capable of mutating a value for the purpose of
 fuzz-testing.

 For example, a mutator could change the value
 `v1 = [1, 4, 2, 1]` to `v1' = [1, 5, 2, 1]`.
 The idea is that if v1 is an “interesting” value to test, then v1' also
 has a high chance of being “interesting” to test.

 ## Complexity

 A mutator is also responsible for keeping track of the
 [complexity](crate::Mutator::complexity) of a value. The complexity is,
 roughly speaking, how large the value is.

 For example, the complexity of a vector is the complexity of its length,
 plus  the sum of the complexities of its elements. So `vec![]` would have a
 complexity of `0.0` and `vec![76]` would have a complexity of `9.0`: `1.0`
 for  its short length and `8.0` for the 8-bit integer “76”. But there is no
 fixed rule for how to compute the complexity of a value, and it is up to you
 to judge how “large” something is.

  ## Cache

 In order to mutate values efficiently, the mutator is able to make use of a
 per-value *cache*. The Cache contains information associated with the value
 that will make it faster to compute its complexity or apply a mutation to
 it. For a vector, its cache is its total complexity, along with a vector of
 the cache of each of its element.

  ## MutationStep

 The same values will be passed to the mutator many times, so that it is
 mutated in many different ways. There are different strategies to choose
 what mutation to apply to a value. The first one is to create a list of
 mutation operations, and choose one to apply randomly from this list.

 However, one may want to have better control over which mutation operation
 is used. For example, if the value to be mutated is of type `Option<T>`,
 then you may want to first mutate it to `None`, and then always mutate it
 to another `Some(t)`. This is where `MutationStep` comes in. The mutation
 step is a type you define to allow you to keep track of which mutation
 operation has already been tried. This allows you to deterministically
 apply mutations to a value such that better mutations are tried first, and
 duplicate mutations are avoided.

 ## Unmutate

 Finally, it is important to note that values and caches are mutated
 *in-place*. The fuzzer does not clone them before handing them to the
 mutator. Therefore, the mutator also needs to know how to reverse each
 mutation it performed. To do so, each mutation needs to return a token
 describing how to reverse it. The [unmutate](crate::Mutator::unmutate)
 method will later be called with that token to get the original value
 and cache back.

 For example, if the value is `[[1, 3], [5], [9, 8]]`, the mutator may
 mutate it to `[[1, 3], [5], [9, 1, 8]]` and return the token:
 `Element(2, Remove(1))`, which means that in order to reverse the
 mutation, the element at index 2 has to be unmutated by removing
 its element at index 1. In pseudocode:

 ```ignore
 value = [[1, 3], [5], [9, 8]];
 cache: c1 (ommitted from example)
 step: s1 (ommitted from example)

 let unmutate_token = self.mutate(&mut value, &mut cache, &mut step, max_cplx);

 // value = [[1, 3], [5], [9, 1, 8]]
 // token = Element(2, Remove(1))
 // cache = c2
 // step = s2

 test(&value);

 self.unmutate(&mut value, &mut cache, unmutate_token);

 // value = [[1, 3], [5], [9, 8]]
 // cache = c1 (back to original cache)
 // step = s2 (step has not been reversed)
 ```

**/
pub trait Mutator: Sized {
    type Value: Clone;
    type Cache: Clone;
    type MutationStep: Clone;
    type UnmutateToken;

    /// Compute the cache for the given value
    fn cache_from_value(&self, value: &Self::Value) -> Self::Cache;
    /// Compute the initial mutation step for the given value
    fn initial_step_from_value(&self, value: &Self::Value) -> Self::MutationStep;
    /// Compute a random mutation step for the given value
    fn random_step_from_value(&self, value: &Self::Value) -> Self::MutationStep;
    /// The maximum complexity of an input of this type
    fn max_complexity(&self) -> f64;
    /// The minimum complexity of an input of this type
    fn min_complexity(&self) -> f64;
    /// The complexity of the current input
    fn complexity(&self, value: &Self::Value, cache: &Self::Cache) -> f64;

    /// Create an arbitrary value
    fn arbitrary(&mut self, seed: usize, max_cplx: f64) -> (Self::Value, Self::Cache);

    fn mutate(
        &mut self,
        value: &mut Self::Value,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        max_cplx: f64,
    ) -> Self::UnmutateToken;

    fn unmutate(&self, value: &mut Self::Value, cache: &mut Self::Cache, t: Self::UnmutateToken);
}

/**
 * A Serializer is used to encode and decode values into bytes.
 *
 * One possible implementation would be to use `serde` to implement
 * both required functions. But we also want to be able to fuzz-test
 * types that are not serializable with `serde`, which is why this
 * Serializer trait exists.
*/
pub trait Serializer {
    type Value;
    fn extension(&self) -> &str;
    fn from_data(&self, data: &[u8]) -> Option<Self::Value>;
    fn to_data(&self, value: &Self::Value) -> Vec<u8>;
}
