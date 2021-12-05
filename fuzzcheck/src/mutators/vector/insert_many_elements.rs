use std::ops::Range;

use super::VecMutator;
use crate::mutators::gen_f64;
use crate::mutators::mutations::{Mutation, RevertMutation};
use crate::Mutator;

pub struct InsertManyElements {
    pub nbr_added_elements: usize,
    pub repeated: bool,
}

// for now, everything random
#[derive(Clone)]
pub struct InsertManyElementsStep {
    nbr_added_elements: usize,
    repeated: bool,
}

pub struct ConcreteInsertManyElements<T> {
    els: Vec<T>,
    added_cplx: f64,
    idx: usize,
}
pub struct RevertInsertManyElements {
    idcs: Range<usize>,
}

impl<T, M> RevertMutation<Vec<T>, VecMutator<T, M>> for RevertInsertManyElements
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    fn revert(
        self,
        _mutator: &VecMutator<T, M>,
        value: &mut Vec<T>,
        _cache: &mut <VecMutator<T, M> as Mutator<Vec<T>>>::Cache,
    ) {
        let _ = value.drain(self.idcs);
    }
}

impl<T, M> Mutation<Vec<T>, VecMutator<T, M>> for InsertManyElements
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    type RandomStep = InsertManyElementsStep;
    type Step = InsertManyElementsStep;
    type Concrete<'a> = ConcreteInsertManyElements<T>;
    type Revert = RevertInsertManyElements;

    fn default_random_step(&self, mutator: &VecMutator<T, M>, value: &Vec<T>) -> Option<Self::RandomStep> {
        if mutator.m.max_complexity() == 0. {
            return None;
        }
        // e.g. value.len() == 3, mutator.len_range == 2 ..= 4
        // then 3+2 > 4 is true
        // so we can't add two more elements,
        // return None
        if value.len() + self.nbr_added_elements > *mutator.len_range.end() {
            None
        } else {
            Some(InsertManyElementsStep {
                nbr_added_elements: self.nbr_added_elements,
                repeated: self.repeated,
            })
        }
    }

    fn random<'a>(
        mutator: &VecMutator<T, M>,
        value: &Vec<T>,
        cache: &<VecMutator<T, M> as Mutator<Vec<T>>>::Cache,
        random_step: &Self::RandomStep,
        max_cplx: f64,
    ) -> Self::Concrete<'a> {
        let value_cplx = mutator.complexity(value, cache);
        let min_added_cplx = mutator.m.min_complexity() * random_step.nbr_added_elements as f64;
        let min_new_cplx = mutator.complexity_from_inner(
            cache.sum_cplx + min_added_cplx,
            value.len() + random_step.nbr_added_elements,
        );
        if min_new_cplx > max_cplx {
            ConcreteInsertManyElements {
                els: vec![],
                added_cplx: 0.,
                idx: 0,
            }
        } else {
            let start_idx = mutator.rng.usize(..=value.len());
            let spare_cplx = max_cplx - value_cplx;

            let (els, added_cplx) = if random_step.repeated {
                let (el, el_cplx) = mutator
                    .m
                    .random_arbitrary(spare_cplx / random_step.nbr_added_elements as f64);
                let els = std::iter::repeat(el).take(random_step.nbr_added_elements).collect();
                let cplx = el_cplx * random_step.nbr_added_elements as f64;
                (els, cplx)
            } else {
                let target_cplx = gen_f64(&mutator.rng, min_new_cplx..spare_cplx);

                let mut v = Vec::with_capacity(random_step.nbr_added_elements);
                let mut sum_cplx = 0.0;

                let mut remaining_cplx = target_cplx;
                for i in 0..random_step.nbr_added_elements {
                    let max_cplx_element = remaining_cplx / ((random_step.nbr_added_elements - i) as f64);
                    let min_cplx_el = mutator.m.min_complexity();

                    if min_cplx_el >= max_cplx_element {
                        break;
                    }
                    let (x, x_cplx) = mutator.m.random_arbitrary(max_cplx_element);
                    sum_cplx += x_cplx;
                    v.push(x);
                    remaining_cplx -= x_cplx;
                }
                if v.len() < random_step.nbr_added_elements {
                    // at this point it is smaller than it must be, so we add new, minimal, elements
                    let remaining = random_step.nbr_added_elements - v.len();
                    for _ in 0..remaining {
                        let (x, x_cplx) = mutator.m.random_arbitrary(0.0);
                        v.push(x);
                        sum_cplx += x_cplx;
                    }
                }
                mutator.rng.shuffle(&mut v);
                (v, sum_cplx)
            };
            ConcreteInsertManyElements {
                els,
                added_cplx,
                idx: start_idx,
            }
        }
    }

    fn default_step(
        &self,
        mutator: &VecMutator<T, M>,
        value: &Vec<T>,
        _cache: &<VecMutator<T, M> as Mutator<Vec<T>>>::Cache,
    ) -> Option<Self::Step> {
        if mutator.m.max_complexity() == 0. {
            return None;
        }
        self.default_random_step(mutator, value)
    }

    fn from_step<'a>(
        mutator: &VecMutator<T, M>,
        value: &Vec<T>,
        cache: &<VecMutator<T, M> as Mutator<Vec<T>>>::Cache,
        step: &'a mut Self::Step,
        max_cplx: f64,
    ) -> Option<Self::Concrete<'a>> {
        let concrete = Self::random(mutator, value, cache, step, max_cplx);
        if concrete.els.is_empty() {
            None
        } else {
            Some(concrete)
        }
    }

    fn apply<'a>(
        mutation: Self::Concrete<'a>,
        mutator: &VecMutator<T, M>,
        value: &mut Vec<T>,
        cache: &mut <VecMutator<T, M> as Mutator<Vec<T>>>::Cache,
        _max_cplx: f64,
    ) -> (Self::Revert, f64) {
        let idcs = mutation.idx..mutation.idx + mutation.els.len();
        insert_many(value, mutation.idx, mutation.els.into_iter());
        let cplx = mutator.complexity_from_inner(cache.sum_cplx + mutation.added_cplx, value.len());
        let revert = RevertInsertManyElements { idcs };
        (revert, cplx)
    }
}
#[no_coverage]
pub fn insert_many<T>(v: &mut Vec<T>, idx: usize, iter: impl Iterator<Item = T>) {
    let moved_slice = v.drain(idx..).collect::<Vec<T>>().into_iter();
    v.extend(iter);
    v.extend(moved_slice);
}

// impl<T, M> Mutation<Vec<T>, VecMutator<T, M>> for InsertElement
// where
//     T: Clone + 'static,
//     M: Mutator<T>,
// {
//     type RandomStep = ();
//     type Step = InsertElementStep<M::ArbitraryStep>;
//     type Concrete<'a> = ConcreteInsertElement<T>;
//     type Revert = RevertInsertElement;

//     fn default_random_step(mutator: &VecMutator<T, M>, value: &Vec<T>) -> Option<Self::RandomStep> {
//         if value.len() >= *mutator.len_range.end() {
//             None
//         } else {
//             Some(())
//         }
//     }

//     fn random<'a>(
//         mutator: &VecMutator<T, M>,
//         value: &Vec<T>,
//         cache: &<VecMutator<T, M> as Mutator<Vec<T>>>::Cache,
//         _random_step: &Self::RandomStep,
//         max_cplx: f64,
//     ) -> Self::Concrete<'a> {
//         let value_cplx = mutator.complexity(value, cache);
//         let spare_cplx = max_cplx - value_cplx;

//         let (el, cplx) = mutator.m.random_arbitrary(spare_cplx);
//         ConcreteInsertElement {
//             el,
//             cplx,
//             idx: mutator.rng.usize(..=value.len()),
//         }
//     }

//     fn default_step(
//         mutator: &VecMutator<T, M>,
//         value: &Vec<T>,
//         _cache: &<VecMutator<T, M> as Mutator<Vec<T>>>::Cache,
//     ) -> Option<Self::Step> {
//         if value.len() >= *mutator.len_range.end() {
//             None
//         } else {
//             Some(InsertElementStep {
//                 arbitrary_steps: (0..=value.len())
//                     .map(|i| (i, mutator.m.default_arbitrary_step()))
//                     .collect(),
//             })
//         }
//     }

//     fn from_step<'a>(
//         mutator: &VecMutator<T, M>,
//         value: &Vec<T>,
//         cache: &<VecMutator<T, M> as Mutator<Vec<T>>>::Cache,
//         step: &'a mut Self::Step,
//         max_cplx: f64,
//     ) -> Option<Self::Concrete<'a>> {
//         if step.arbitrary_steps.is_empty() {
//             return None;
//         }
//         let value_cplx = mutator.complexity(value, cache);
//         let spare_cplx = max_cplx - value_cplx;
//         let choice = mutator.rng.usize(..step.arbitrary_steps.len());
//         let (idx, arbitrary_step) = &mut step.arbitrary_steps[choice];

//         if let Some((el, cplx)) = mutator.m.ordered_arbitrary(arbitrary_step, spare_cplx) {
//             Some(ConcreteInsertElement { el, cplx, idx: *idx })
//         } else {
//             step.arbitrary_steps.remove(choice);
//             Self::from_step(mutator, value, cache, step, max_cplx)
//         }
//     }

//     fn apply<'a>(
//         mutation: Self::Concrete<'a>,
//         mutator: &VecMutator<T, M>,
//         value: &mut Vec<T>,
//         cache: &mut <VecMutator<T, M> as Mutator<Vec<T>>>::Cache,
//         _max_cplx: f64,
//     ) -> (Self::Revert, f64) {
//         value.insert(mutation.idx, mutation.el);
//         let new_cplx = mutator.complexity_from_inner(cache.sum_cplx + mutation.cplx, value.len());
//         (RevertInsertElement { idx: mutation.idx }, new_cplx)
//     }
// }
