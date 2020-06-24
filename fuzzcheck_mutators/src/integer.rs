
use fuzzcheck_traits::Mutator;

macro_rules! impl_unsigned_mutator {
    ($name:ty,$name_mutator:ident) => {
#[derive(Clone)]
pub struct $name_mutator {}
impl Default for $name_mutator {
    fn default() -> Self {
        $name_mutator {}
    }
}

impl $name_mutator {
    pub fn _arbitrary(low: $name, high: $name, step: u64) -> $name {
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
            $name_mutator::_arbitrary(next.wrapping_add(1), high, step / 2)
        } else {
            // step % 2 == 0
            $name_mutator::_arbitrary(low, next.wrapping_sub(1), (step - 1) / 2)
        }
    }
}

impl Mutator for $name_mutator {
    type Value = $name;
    type Cache = ();
    type MutationStep = u64; // mutation step
    type UnmutateToken = $name; // old value

    fn cache_from_value(&self, _value: &Self::Value) -> Self::Cache {}
    fn mutation_step_from_value(&self, _value: &Self::Value) -> Self::MutationStep {
        0
    }

    fn arbitrary(&mut self, seed: usize, _max_cplx: f64) -> (Self::Value, Self::Cache) {
        let value = $name_mutator::_arbitrary(<$name>::MIN, <$name>::MAX, seed as u64);// (seed % <$name>::MAX as usize) as $name;
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
                let nudge = (tmp_step + 2) as $name;
                if nudge % 2 == 0 {
                    value.wrapping_add(nudge / 2)
                } else {
                    value.wrapping_sub(nudge / 2)
                }
            } else {
                tmp_step -= 7;
                let low = value.wrapping_sub(<$name>::MAX / 2);
                let high = value.wrapping_add(<$name>::MAX / 2 + 1);
                $name_mutator::_arbitrary(low, high, tmp_step)
            }
        };
        *step = step.wrapping_add(1);

        token
    }

    fn unmutate(&self, value: &mut Self::Value, _cache: &mut Self::Cache, t: Self::UnmutateToken) {
        *value = t;
    }
}
    };
}

impl_unsigned_mutator!(u8, U8Mutator);
impl_unsigned_mutator!(u16, U16Mutator);
impl_unsigned_mutator!(u32, U32Mutator);
impl_unsigned_mutator!(u64, U64Mutator);


macro_rules! impl_signed_mutator {
    ($name:ty,$name_unsigned:ty,$name_mutator:ident) => {
#[derive(Clone)]
pub struct $name_mutator {}
impl Default for $name_mutator {
    fn default() -> Self {
        $name_mutator {}
    }
}

impl $name_mutator {
    pub fn _arbitrary(low: $name_unsigned, high: $name_unsigned, step: u64) -> $name_unsigned {
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
            $name_mutator::_arbitrary(next.wrapping_add(1), high, step / 2)
        } else {
            // step % 2 == 0
            $name_mutator::_arbitrary(low, next.wrapping_sub(1), (step - 1) / 2)
        }
    }
}

impl Mutator for $name_mutator {
    type Value = $name;
    type Cache = ();
    type MutationStep = u64; // mutation step
    type UnmutateToken = $name; // old value

    fn cache_from_value(&self, _value: &Self::Value) -> Self::Cache {}
    fn mutation_step_from_value(&self, _value: &Self::Value) -> Self::MutationStep {
        0
    }

    fn arbitrary(&mut self, seed: usize, _max_cplx: f64) -> (Self::Value, Self::Cache) {
        let value = $name_mutator::_arbitrary(<$name_unsigned>::MIN, <$name_unsigned>::MAX, seed as u64) as $name;
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
                let nudge = (tmp_step + 2) as $name;
                if nudge % 2 == 0 {
                    value.wrapping_add(nudge / 2)
                } else {
                    value.wrapping_sub(nudge / 2)
                }
            } else {
                tmp_step -= 7;
                let low = (*value as $name_unsigned).wrapping_sub(<$name_unsigned>::MAX / 2);
                let high = (*value as $name_unsigned).wrapping_add(<$name_unsigned>::MAX / 2 + 1);
                $name_mutator::_arbitrary(low, high, tmp_step) as $name
            }
        };
        *step = step.wrapping_add(1);

        token
    }

    fn unmutate(&self, value: &mut Self::Value, _cache: &mut Self::Cache, t: Self::UnmutateToken) {
        *value = t;
    }
}
    };
}

impl_signed_mutator!(i8, u8, I8Mutator);
impl_signed_mutator!(i16, u16, I16Mutator);
impl_signed_mutator!(i32, u32, I32Mutator);
impl_signed_mutator!(i64, u64, I64Mutator);
