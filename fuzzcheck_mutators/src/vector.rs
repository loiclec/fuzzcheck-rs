extern crate fuzzcheck;
use fuzzcheck::Mutator;

extern crate rand;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

pub struct VecMutator<M: Mutator> {
    m: M,
}
impl<M: Mutator> VecMutator<M> {
    pub fn new(m: M) -> Self {
        Self { m }
    }
}
impl<M: Mutator> Default for VecMutator<M>
where
    M: Default,
{
    fn default() -> Self {
        Self::new(M::default())
    }
}

struct VecMutatorArbitrarySeed {
    complexity_step: usize,
    len_step: usize,
    rng: SmallRng,
}

impl VecMutatorArbitrarySeed {
    fn new(step: usize) -> Self {
        let mut rng = SmallRng::seed_from_u64(step as u64);
        if step == 0 {
            Self {
                complexity_step: 0,
                len_step: 0,
                rng,
            }
        } else {
            let (complexity_step, len_step) = if step < 100 {
                // deterministic phase for 100 first steps
                (step % 10, step / 10)
            } else {
                // default
                (step, rng.gen())
            };
            Self {
                complexity_step,
                len_step,
                rng,
            }
        }
    }
}

#[derive(Clone, Debug)]
struct MutationStep {
    category: MutationCategory,
    remove_idx: usize,
    insert_idx: usize,
    vec_operations: Vec<VecOperation>,
    cycle: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VecOperation {
    Remove,
    Insert,
}

impl MutationStep {
    fn new(len: usize) -> Self {
        let (category, vec_operations) = if len > 0 {
            (
                MutationCategory::Element(0),
                vec![VecOperation::Insert, VecOperation::Remove],
            )
        } else {
            (MutationCategory::Empty, vec![VecOperation::Insert])
        };
        Self {
            category,
            remove_idx: len.saturating_sub(1),
            insert_idx: 0,
            vec_operations,
            cycle: 0,
        }
    }
}

#[derive(Debug, Clone)]
enum MutationCategory {
    Empty,
    Element(usize),
    Vector(usize),
}
use crate::vector::MutationCategory::{Element, Empty, Vector};

#[derive(Clone)]
pub struct VecMutatorCache<C> {
    inner: Vec<C>,
    sum_cplx: f64,
}
impl<C> Default for VecMutatorCache<C> {
    fn default() -> Self {
        Self {
            inner: Vec::new(),
            sum_cplx: 0.0,
        }
    }
}

pub struct VecMutatorStep<S> {
    inner: Vec<S>,
    // TODO: rename that
    pick_step: MutationStep,
}

impl<S> VecMutatorStep<S> {
    fn increment_mutation_step_category(&mut self) {
        match self.pick_step.category {
            Empty => {
                if self.inner.is_empty() {
                    self.pick_step.category = MutationCategory::Vector(0)
                } else {
                    self.pick_step.category = MutationCategory::Element(0)
                }
            }
            Element(idx) => {
                let new_idx = idx + 1;
                if new_idx < self.inner.len() {
                    self.pick_step.category = MutationCategory::Element(new_idx)
                } else {
                    self.pick_step.category = MutationCategory::Vector(0)
                }
            }
            Vector(step) => {
                let new_step = step + 1;
                if new_step < self.pick_step.vec_operations.len() {
                    self.pick_step.category = MutationCategory::Vector(new_step)
                } else {
                    self.pick_step.cycle += 1;
                    if self.inner.is_empty() {
                        self.pick_step.category = MutationCategory::Vector(0)
                    } else {
                        self.pick_step.category = MutationCategory::Element(0)
                    }
                }
            }
        }
    }
}

pub enum UnmutateVecToken<M: Mutator> {
    Element(usize, M::UnmutateToken, f64),
    Remove(usize, f64),
    Insert(usize, M::Value, M::Cache),
    Replace(<VecMutator<M> as Mutator>::Value, <VecMutator<M> as Mutator>::Cache),
}

impl<M: Mutator> VecMutator<M> {
    fn mutate_element(
        &self,
        value: &mut Vec<M::Value>,
        cache: &mut VecMutatorCache<M::Cache>,
        step: &mut VecMutatorStep<M::MutationStep>,
        idx: usize,
        spare_cplx: f64,
    ) -> UnmutateVecToken<M> {
        let el = &mut value[idx];
        let el_cache = &mut cache.inner[idx];
        let el_step = &mut step.inner[idx];

        let old_cplx = self.m.complexity(el, el_cache);

        let token = self.m.mutate(el, el_cache, el_step, spare_cplx);

        let new_cplx = self.m.complexity(el, el_cache);

        cache.sum_cplx += new_cplx - old_cplx;
        step.increment_mutation_step_category();

        UnmutateVecToken::Element(idx, token, old_cplx - new_cplx)
    }

    fn insert_element(
        &self,
        value: &mut Vec<M::Value>,
        cache: &mut VecMutatorCache<M::Cache>,
        step: &mut VecMutatorStep<M::MutationStep>,
        spare_cplx: f64,
    ) -> UnmutateVecToken<M> {
        let (idx, cycle) = (step.pick_step.insert_idx, step.pick_step.cycle);

        // TODO: For now I assume that the complexity given by the length of the vector does not change
        // Should I take it into account instead?
        let (el, el_cache) = self.m.arbitrary(cycle, spare_cplx);
        let el_cplx = self.m.complexity(&el, &el_cache);

        value.insert(idx, el);

        // TODO: updating the cache is not *really* needed unless I start cloning the cache
        // in the fuzzer itself, so should I? maybe it will make things clearer and more consistent
        // maybe it's more surface to introduce bugs
        cache.inner.insert(idx, el_cache);
        // Don't do the following! It is not possible to unmutate mutation steps
        // step.inner.insert(idx, el_step);

        let token = UnmutateVecToken::Remove(idx, el_cplx); // TODO: is that always right?

        cache.sum_cplx += el_cplx;

        // TODO: have only one function for the len() of the vector, that stays consistent
        step.pick_step.insert_idx = (step.pick_step.insert_idx + 1) % (step.inner.len() + 1);
        step.increment_mutation_step_category();

        token
    }

    fn remove_element(
        &self,
        value: &mut Vec<M::Value>,
        cache: &mut VecMutatorCache<M::Cache>,
        step: &mut VecMutatorStep<M::MutationStep>,
    ) -> UnmutateVecToken<M> {
        let idx = step.pick_step.remove_idx;

        let el = &value[idx];
        let el_cplx = self.m.complexity(&el, &cache.inner[idx]);

        let removed_el = value.remove(idx);
        // TODO: again, that's not really necessary
        let removed_el_cache = cache.inner.remove(idx);
        // Don't do the following! It is not possible to unmutate mutation steps
        // let removed_el_step = step.inner.remove(idx);

        // TODO: restore cache and step too
        let token = UnmutateVecToken::Insert(idx, removed_el, removed_el_cache);

        cache.sum_cplx -= el_cplx;

        if step.pick_step.remove_idx == 0 {
            step.pick_step.vec_operations.remove_item(&VecOperation::Remove);
        } else {
            step.pick_step.remove_idx -= 1;
        }

        step.increment_mutation_step_category();

        token
    }
}

impl<M: Mutator> Mutator for VecMutator<M> {
    type Value = Vec<M::Value>;
    type Cache = VecMutatorCache<M::Cache>;
    type MutationStep = VecMutatorStep<M::MutationStep>;
    type UnmutateToken = UnmutateVecToken<M>;

    fn max_complexity(&self) -> f64 {
        std::f64::INFINITY
    }

    fn min_complexity(&self) -> f64 {
        1.0
    }

    fn complexity(&self, value: &Self::Value, cache: &Self::Cache) -> f64 {
        1.0 + cache.sum_cplx + crate::size_to_cplxity(value.len() + 1)
    }

    fn cache_from_value(&self, value: &Self::Value) -> Self::Cache {
        let inner: Vec<_> = value.iter().map(|x| self.m.cache_from_value(x)).collect();

        let sum_cplx = value
            .iter()
            .zip(inner.iter())
            .fold(0.0, |cplx, (v, cache)| cplx + self.m.complexity(v, cache));

        VecMutatorCache { inner, sum_cplx }
    }
    fn mutation_step_from_value(&self, value: &Self::Value) -> Self::MutationStep {
        let inner: Vec<_> = value.iter().map(|x| self.m.mutation_step_from_value(x)).collect();
        VecMutatorStep {
            inner,
            pick_step: MutationStep::new(value.len()),
        }
    }

    fn arbitrary(&self, seed: usize, max_cplx: f64) -> (Self::Value, Self::Cache) {
        let VecMutatorArbitrarySeed {
            complexity_step,
            len_step,
            mut rng,
        } = VecMutatorArbitrarySeed::new(seed);

        if seed == 0 || max_cplx <= 1.0 {
            return (Self::Value::default(), Self::Cache::default());
        }

        let target_cplx = {
            let increments_target_cplx = (max_cplx * 100.0).round() as usize;
            let multiplied_target_cplx = crate::arbitrary_binary(0, increments_target_cplx, complexity_step) as f64;
            multiplied_target_cplx / 100.0
        };
        let min_cplx_el = self.m.min_complexity();

        // slight underestimate of the maximum number of elements required to produce an input of max_cplx
        let max_len_most_complex = {
            let overestimated_max_len: f64 = target_cplx / min_cplx_el;
            let max_len = if overestimated_max_len.is_infinite() {
                // min_cplx_el is 0, so the max length is the maximum complexity of the length component of the vector
                crate::cplxity_to_size(target_cplx)
            } else {
                // an underestimate of the true max_length, but not by much
                (overestimated_max_len - overestimated_max_len.log2()) as usize
            };
            if max_len > 10_000 {
                /* TODO */
                // 10_000?
                target_cplx.trunc() as usize
            } else {
                max_len
            }
        };
        let max_cplx_el = self.m.max_complexity();
        // slight underestimate of the minimum number of elements required to produce an input of max_cplx
        let min_len_most_complex = target_cplx / max_cplx_el - (target_cplx / max_cplx_el).log2();
        if !min_len_most_complex.is_finite() {
            // in this case, the elements are always of cplx 0, so we can only vary the length of the vector
            let len = crate::arbitrary_binary(0, max_len_most_complex, len_step);
            let mut v = Self::Value::default();
            let mut cache = Self::Cache::default();
            for _ in 0..len {
                // no point in adding valid step and max_cplx argument, the elements have only one possible value
                let (el, el_cache) = self.m.arbitrary(0, 0.0);
                v.push(el);
                cache.inner.push(el_cache); // I don't update sum_cplx because it is 0
            }
            return (v, cache);
        }
        let min_len_most_complex = min_len_most_complex.trunc() as usize;
        // arbitrary restriction on the length of the generated number, to avoid creating absurdly large vectors
        // of very simple elements, that take up too much memory
        let max_len_most_complex = if max_len_most_complex > 10_000 {
            /* TODO */
            // 10_000?
            target_cplx.trunc() as usize
        } else {
            max_len_most_complex
        };

        // choose a length between min_len_most_complex and max_len_most_complex
        let target_len = crate::arbitrary_binary(min_len_most_complex, max_len_most_complex, len_step);
        // TODO: create a new_input_with_complexity method
        let mut v = Self::Value::default();
        let mut cache = Self::Cache::default();
        let mut remaining_cplx = target_cplx;
        for i in 0..target_len {
            let max_cplx_element = remaining_cplx / ((target_len - i) as f64);
            if max_cplx_element <= min_cplx_el {
                break;
            }
            let cplx_element = rng.gen_range(min_cplx_el, max_cplx_element);
            let (x, x_cache) = self.m.arbitrary(rng.gen(), cplx_element);
            let x_cplx = self.m.complexity(&x, &x_cache);
            v.push(x);
            cache.inner.push(x_cache);
            cache.sum_cplx += x_cplx;
            remaining_cplx -= x_cplx;
        }
        (v, cache)
    }

    fn mutate(
        &self,
        value: &mut Self::Value,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        max_cplx: f64,
    ) -> Self::UnmutateToken {
        let spare_cplx = max_cplx - self.complexity(value, cache);

        match step.pick_step.category {
            MutationCategory::Empty => {
                step.increment_mutation_step_category();

                let mut old_value = Self::Value::default();
                let mut old_cache = Self::Cache::default();

                std::mem::swap(value, &mut old_value);
                std::mem::swap(cache, &mut old_cache);

                UnmutateVecToken::Replace(old_value, old_cache)
            }
            MutationCategory::Element(idx) => self.mutate_element(value, cache, step, idx, spare_cplx),
            MutationCategory::Vector(vector_step) => {
                let operation_idx = vector_step % step.pick_step.vec_operations.len();
                let operation = step.pick_step.vec_operations[operation_idx];
                match operation {
                    VecOperation::Insert => self.insert_element(value, cache, step, spare_cplx),
                    VecOperation::Remove => self.remove_element(value, cache, step),
                }
            }
        }
    }

    fn unmutate(&self, value: &mut Self::Value, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        match t {
            UnmutateVecToken::Element(idx, inner_t, diff_cplx) => {
                let el = &mut value[idx];
                let el_cache = &mut cache.inner[idx];
                self.m.unmutate(el, el_cache, inner_t);
                cache.sum_cplx += diff_cplx;
            }
            UnmutateVecToken::Insert(idx, el, el_cache) => {
                cache.sum_cplx += self.m.complexity(&el, &el_cache);

                value.insert(idx, el);
                cache.inner.insert(idx, el_cache);
            }
            UnmutateVecToken::Remove(idx, el_cplx) => {
                value.remove(idx);
                cache.inner.remove(idx);
                cache.sum_cplx -= el_cplx;
            }
            UnmutateVecToken::Replace(new_value, new_cache) => {
                let _ = std::mem::replace(value, new_value);
                let _ = std::mem::replace(cache, new_cache);
            }
        }
    }
}
