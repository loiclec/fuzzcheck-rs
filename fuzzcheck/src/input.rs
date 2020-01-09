use std::hash::Hasher;

pub trait FuzzedInput {
    type Value: Clone;
    type State: Clone;
    type UnmutateToken;
    // TODO: separate state into mutation_step and cache
    // give mutation step and cache to mutate()
    // give cache but no mutation step to new_source()
    // review every trait function accordingly

    /// default constructor
    fn default() -> Self::Value;

    fn state_from_value(value: &Self::Value) -> Self::State;

    /// This is meant to create arbitrary values of Input.
    fn arbitrary(seed: usize, max_cplx: f64) -> Self::Value;

    /// The maximum complexity of an input of this type
    fn max_complexity() -> f64;
    /// The minimum complexity of an input of this type
    fn min_complexity() -> f64;

    /// Feeds the input into the Hasher.
    fn hash_value<H: Hasher>(value: &Self::Value, state: &mut H);

    /// The complexity of the current input
    fn complexity(value: &Self::Value, state: &Self::State) -> f64;

    fn mutate(value: &mut Self::Value, state: &mut Self::State, max_cplx: f64) -> Self::UnmutateToken;

    fn unmutate(value: &mut Self::Value, state: &mut Self::State, t: Self::UnmutateToken);

    fn from_data(data: &[u8]) -> Option<Self::Value>;
    fn to_data(value: &Self::Value) -> Vec<u8>;
}

pub struct UnifiedFuzzedInput<I: FuzzedInput> {
    pub value: I::Value,
    pub state: I::State,
}

impl<I: FuzzedInput> Clone for UnifiedFuzzedInput<I> {
    fn clone(&self) -> Self {
        UnifiedFuzzedInput {
            value: self.value.clone(),
            state: self.state.clone(),
        }
    }
}

impl<I: FuzzedInput> UnifiedFuzzedInput<I> {
    pub fn new(data: (I::Value, I::State)) -> Self {
        Self {
            value: data.0,
            state: data.1,
        }
    }
    pub fn default() -> Self {
        let value = I::default();
        let state = I::state_from_value(&value);
        Self { value, state }
    }
    pub fn complexity(&self) -> f64 {
        I::complexity(&self.value, &self.state)
    }
    pub fn mutate(&mut self, max_cplx: f64) -> I::UnmutateToken {
        I::mutate(&mut self.value, &mut self.state, max_cplx)
    }
    pub fn new_source(&self) -> Self {
        let (value, state) = (self.value.clone(), I::state_from_value(&self.value));
        Self { value, state }
    }
    pub fn unmutate(&mut self, t: I::UnmutateToken) {
        I::unmutate(&mut self.value, &mut self.state, t)
    }
}
