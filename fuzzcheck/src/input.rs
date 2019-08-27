use std::hash::Hasher;

pub trait InputGenerator {
    type Input: Clone;

    /**
     * Returns the complexity of the given input.
     *
     * Fuzzcheck will prefer using inputs with a smaller complexity.
     * Important: The return value must be >= 0.0
     *
     * ## Examples
     * - an array might have a complexity equal to the sum of complexities of its elements
     * - an integer might have a complexity equal to the number of bytes used to represent it
     */
    fn complexity(input: &Self::Input) -> f64;

    /**
     * Feeds the input into the Hasher.
     */
    fn hash<H>(input: &Self::Input, state: &mut H)
    where
        H: Hasher;

    fn base_input() -> Self::Input;

    /**
     * Return a new input to test.
     *
     * It can be completely random or drawn from a corpus of “special” inputs
     * or generated in any other way that yields a wide variety of inputs.
     */
    fn new_input(&mut self, max_cplx: f64) -> Self::Input;

    fn initial_inputs(&mut self, max_cplx: f64) -> Vec<Self::Input> {
        (0..10).map(|_| self.new_input(max_cplx)).collect()
    }

    /**
     * Mutate the given input.
     *
     * Fuzzcheck will call this method repeatedly in order to explore all the
     * possible values of Input. It is therefore important that it is implemented
     * efficiently.
     *
     * It should be theoretically possible to mutate any arbitrary input `u1` into any
     * other arbitrary input `u2` by calling `mutate` repeatedly.
     *
     * Moreover, the result of `mutate` should try to be “interesting” to Fuzzcheck.
     * That is, it should be likely to trigger new code paths when passed to the
     * test function.
     *
     * ## Examples
     * - append a random element to an array
     * - mutate a random element in an array
     * - subtract a small constant from an integer
     * - change an integer to Int.min or Int.max or 0
     * - replace a substring by a keyword relevant to the test function
     */
    fn mutate(&mut self, input: &mut Self::Input, spare_cplx: f64) -> bool;

    fn from_data(data: &[u8]) -> Option<Self::Input>;
    fn to_data(input: &Self::Input) -> Vec<u8>;
}
