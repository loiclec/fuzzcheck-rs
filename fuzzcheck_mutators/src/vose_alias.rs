use fastrand::Rng;
use std::fmt::Debug;

#[derive(Debug, Clone)]
pub struct VoseAlias {
    pub original_probabilities: Vec<f64>,
    alias: Vec<usize>,
    prob: Vec<f64>,
    rng: Rng,
}
impl PartialEq for VoseAlias {
    #[no_coverage]
    fn eq(&self, other: &Self) -> bool {
        self.alias.eq(&other.alias) && self.prob.eq(&other.prob)
    }
}

// implementation from https://www.keithschwarz.com/darts-dice-coins/
impl VoseAlias {
    /// Note: the probabilities must sum up to 1.0
    #[no_coverage]
    pub fn new(mut probabilities: Vec<f64>) -> VoseAlias {
        let original_probabilities = probabilities.clone();
        // Step 0: ensure sum of probabilities is equal to 1
        assert!(!probabilities.is_empty());
        let sum = probabilities.iter().fold(0.0, |sum, p| sum + p);
        #[allow(clippy::float_cmp)]
        if sum != 1.0 {
            // hack, the whole of the extra probability is added to the first element
            // if it happened due to numerical instability, it's fine, it doesn't
            // bias the odds that much
            // otherwise, it is a mistake from the caller.
            // I check that the difference between sum and 1.0 is less than
            // 0.1 to try and distinguish between the two cases
            assert!((sum - 1.0).abs() < 0.1);
            let add = 1.0 - sum;
            probabilities[0] += add;
        }

        // Step 1 and 2
        let size = probabilities.len();
        let mut small = Vec::new();
        let mut large = Vec::new();
        let mut alias: Vec<usize> = std::iter::repeat(0).take(size).collect();
        let mut prob: Vec<f64> = std::iter::repeat(0.0).take(size).collect();

        // Step 3 and 4
        for (i, p) in probabilities.iter_mut().enumerate() {
            *p *= size as f64;
            if *p < 1.0 {
                small.push(i);
            } else {
                large.push(i);
            }
        }
        // Step 5, 6, 7
        loop {
            match (small.pop(), large.pop()) {
                // Step 5
                (Some(l), Some(g)) => {
                    let p_l = probabilities[l];
                    prob[l] = p_l; // 5.3
                    alias[l] = g; // 5.4

                    let p_g = probabilities[g];
                    let p_g = (p_g + p_l) - 1.0;
                    probabilities[g] = p_g; // 5.5
                    if p_g < 1.0 {
                        small.push(g); // 5.6
                    } else {
                        large.push(g); // 5.7
                    }
                }
                // Step 7
                (Some(l), None) => {
                    prob[l] = 1.0;
                }
                // Step 6
                (None, Some(g)) => {
                    prob[g] = 1.0;
                }
                (None, None) => break,
            }
        }

        VoseAlias {
            original_probabilities,
            alias,
            prob,
            rng: Rng::default(),
        }
    }

    #[no_coverage]
    pub fn sample(&self) -> usize {
        // Step 1
        let i = self.rng.usize(..self.prob.len());
        // Step 2
        if self.rng.f64() <= unsafe { *self.prob.get_unchecked(i) } {
            // Step 3
            i
        } else {
            // Step 4
            unsafe { *self.alias.get_unchecked(i) }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::VoseAlias;
    #[test]
    #[no_coverage]
    fn test_probabilities_1() {
        let alias = VoseAlias::new(vec![0.1, 0.4, 0.2, 0.3]);
        let mut choices = vec![0, 0, 0, 0];
        for _ in 0..100_000 {
            let i = alias.sample();
            choices[i] += 1;
        }
        println!("{:?}", choices);
    }
    #[test]
    #[no_coverage]
    fn test_probabilities_2() {
        let alias = VoseAlias::new(vec![0.1, 0.4, 0.2, 0.3]);
        let mut choices = vec![0, 0, 0, 0];
        for _ in 0..100_000 {
            let i = alias.sample();
            choices[i] += 1;
        }
        println!("{:?}", choices);
    }
}
