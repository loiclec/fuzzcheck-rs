
use fuzzcheck_mutator_trait::Mutator;

#[derive(Clone)]
pub struct U8Mutator {}
impl Default for U8Mutator {
    fn default() -> Self {
        U8Mutator {}
    }
}

impl Mutator for U8Mutator {
    type Value = u8;
    type Cache = ();
    type MutationStep = u16; // mutation step
    type UnmutateToken = u8; // old value

    fn cache_from_value(&self, _value: &Self::Value) -> Self::Cache {}
    fn mutation_step_from_value(&self, _value: &Self::Value) -> Self::MutationStep {
        0
    }

    fn arbitrary(&mut self, seed: usize, _max_cplx: f64) -> (Self::Value, Self::Cache) {
        let value = (seed % std::u8::MAX as usize) as u8;
        (value, ())
    }

    fn max_complexity(&self) -> f64 {
        8.0
    }

    fn min_complexity(&self) -> f64 {
        8.0
    }

    fn complexity(&self, _value: &Self::Value, _cache: &Self::Cache) -> f64 {
        8.0
    }

    fn mutate(
        &mut self,
        value: &mut Self::Value,
        _cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        _max_cplx: f64,
    ) -> Self::UnmutateToken {
        let token = *value;
        *value = {
            let mut tmp_step = *step;
            if tmp_step < 8 {
                let nudge = tmp_step + 2;
                let nudge_u8 = nudge as u8;
                if nudge % 2 == 0 {
                    value.wrapping_add(nudge_u8 / 2)
                } else {
                    value.wrapping_sub(nudge_u8 / 2)
                }
            } else {
                tmp_step -= 7;
                let low = value.wrapping_sub(std::u8::MAX / 2);
                let high = value.wrapping_add(std::u8::MAX / 2 + 1);
                arbitrary_u8(low, high, tmp_step)
            }
        };
        *step = step.wrapping_add(1);

        token
    }

    fn unmutate(&self, value: &mut Self::Value, _cache: &mut Self::Cache, t: Self::UnmutateToken) {
        *value = t;
    }
}

pub fn arbitrary_u8(low: u8, high: u8, step: u16) -> u8 {
    let next = low.wrapping_add(high.wrapping_sub(low) / 2);
    if low.wrapping_add(1) == high {
        if step % 2 == 0 {
            high
        } else {
            low
        }
    } else if step == 0 {
        next
    } else if step % 2 == 1 {
        arbitrary_u8(next.wrapping_add(1), high, step / 2)
    } else {
        // step % 2 == 0
        arbitrary_u8(low, next.wrapping_sub(1), (step - 1) / 2)
    }
}
