use rand::rngs::ThreadRng;

use rand::distributions::Distribution;
use rand::distributions::WeightedIndex;

pub trait WeightedMutators {
    type Input;
    type Mutator;

    fn mutate_with(
        &self,
        input: &mut Self::Input,
        mutator: &Self::Mutator,
        spare_cplx: f64,
        rng: &mut ThreadRng,
    ) -> bool;

    fn mutators(&self) -> &Vec<Self::Mutator>;

    fn weighted_index(&self) -> &WeightedIndex<usize>;

    fn mutate(&self, input: &mut Self::Input, spare_cplx: f64, rng: &mut ThreadRng) -> bool {
        for _ in 0..self.mutators().len() {
            let pick = self.weighted_index().sample(rng);
            if self.mutate_with(input, &self.mutators()[pick], spare_cplx, rng) {
                return true;
            }
        }
        false
    }
}
