
use fuzzcheck_traits::Mutator;

// TODO: explanation
pub fn binary_search_arbitrary(low: u8, high: u8, step: u64) -> u8 {
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
        binary_search_arbitrary(next.wrapping_add(1), high, step / 2)
    } else {
        // step % 2 == 0
        binary_search_arbitrary(low, next.wrapping_sub(1), (step - 1) / 2)
    }
}

macro_rules! impl_unsigned_mutator {
    ($name:ty,$name_mutator:ident,$size:expr) => {
#[derive(Clone)]
pub struct $name_mutator { 
    shuffled_integers: [u8; 256],
}
impl Default for $name_mutator {
    fn default() -> Self {
        let mut shuffled_integers = [0; 256];
        for (i, x) in shuffled_integers.iter_mut().enumerate() {
            *x = binary_search_arbitrary(0, u8::MAX, i as u64);
        }
        $name_mutator {
            shuffled_integers
        }
    }
}

impl $name_mutator {
    // TODO: explanation
    pub fn uniform_permutation(&self, step: u64) -> $name {

        let size = $size as u64;
        let granularity = ((std::mem::size_of::<usize>() * 8) - (self.shuffled_integers.len().leading_zeros() as usize) - 1) as u64;
        let step_mask = ((u8::MAX as usize) >> (8 - granularity)) as u64;

        let step_i = (step & step_mask) as usize;
        let mut prev = unsafe { *self.shuffled_integers.get_unchecked(step_i) as $name };
        
        let mut result = (prev << (size - granularity)) as $name;

        for i in 1 .. (size / granularity) {
            let step_i = (((step >> (i*granularity)) ^ prev as u64) & step_mask) as usize;
            prev = unsafe { *self.shuffled_integers.get_unchecked(step_i) as $name };
            result |= prev << (size - (i+1)*granularity);
        }

        result
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
        let value = self.uniform_permutation(seed as u64);
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
                self.uniform_permutation(tmp_step)
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

impl_unsigned_mutator!(u8, U8Mutator, 8);
impl_unsigned_mutator!(u16, U16Mutator, 16);
impl_unsigned_mutator!(u32, U32Mutator, 32);
impl_unsigned_mutator!(u64, U64Mutator, 64);


macro_rules! impl_signed_mutator {
    ($name:ty,$name_unsigned:ty,$name_mutator:ident,$size:expr) => {
#[derive(Clone)]
pub struct $name_mutator { 
    shuffled_integers: [u8; 256],
}
impl Default for $name_mutator {
    fn default() -> Self {
        let mut shuffled_integers = [0; 256];
        for (i, x) in shuffled_integers.iter_mut().enumerate() {
            *x = binary_search_arbitrary(0, u8::MAX, i as u64);
        }
        $name_mutator {
            shuffled_integers
        }
    }
}

impl $name_mutator {
    // TODO: explanation
    pub fn uniform_permutation(&self, step: u64) -> $name_unsigned {
        let size = $size as u64;
        let granularity = ((std::mem::size_of::<usize>() * 8) - (self.shuffled_integers.len().leading_zeros() as usize) - 1) as u64;
        let step_mask = ((u8::MAX as usize) >> (8 - granularity)) as u64;

        let step_i = (step & step_mask) as usize;
        let mut prev = unsafe { *self.shuffled_integers.get_unchecked(step_i) as $name_unsigned };
        
        let mut result = (prev << (size - granularity)) as $name_unsigned;

        for i in 1 .. (size / granularity) {
            let step_i = (((step >> (i*granularity)) ^ prev as u64) & step_mask) as usize;
            prev = unsafe { *self.shuffled_integers.get_unchecked(step_i) as $name_unsigned };
            result |= prev << (size - (i+1)*granularity);
        }

        result as $name_unsigned
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
        let value = self.uniform_permutation(seed as u64) as $name;
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
                self.uniform_permutation(tmp_step) as $name
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

impl_signed_mutator!(i8, u8, I8Mutator, 8);
impl_signed_mutator!(i16, u16, I16Mutator, 16);
impl_signed_mutator!(i32, u32, I32Mutator, 32);
impl_signed_mutator!(i64, u64, I64Mutator, 64);
