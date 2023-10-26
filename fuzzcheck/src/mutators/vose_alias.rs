//! An efficient data structure to sample from a discrete, fixed distribution.
//!
//! See: <https://www.keithschwarz.com/darts-dice-coins/> for an explanation.

use std::fmt::Debug;

use fastrand::Rng;

/// An efficient data structure to sample from a discrete, fixed distribution.
///
/// ```
/// use fuzzcheck::mutators::vose_alias::VoseAlias;
///
/// // the discrete distribution is a vector of floats which must add up to 1.0
/// let probabilities = vec![0.5, 0.1, 0.2, 0.2];
/// // create the Vose alias. The `probabilities` vector is moved to `alias.original_probabilities`.
/// let alias = VoseAlias::new(probabilities);
///
/// // index has a 50% chance of being 0, 10% chance of being 1, 20% chance of being 2, and 20% chance of being 2
/// let index = alias.sample();
///
/// assert!((0 .. 4).contains(&index));
/// ```
#[derive(Debug, Clone)]
pub struct VoseAlias {
    pub original_probabilities: Vec<f64>,
    alias: Vec<usize>,
    prob: Vec<f64>,
    rng: Rng,
}
impl PartialEq for VoseAlias {
    #[coverage(off)]
    fn eq(&self, other: &Self) -> bool {
        self.alias.eq(&other.alias) && self.prob.eq(&other.prob)
    }
}

// implementation from https://www.keithschwarz.com/darts-dice-coins/
impl VoseAlias {
    /// Create a new Vose alias with the given discrete probability distribution.
    ///
    /// Important: the probabilities must sum up to ~ 1.0
    #[coverage(off)]
    pub fn new(mut probabilities: Vec<f64>) -> VoseAlias {
        let original_probabilities = probabilities.clone();
        // Step 0: ensure sum of probabilities is equal to 1
        assert!(!probabilities.is_empty());
        let sum = probabilities.iter().fold(
            0.0,
            #[coverage(off)]
            |sum, p| sum + p,
        );
        #[allow(clippy::float_cmp)]
        if sum != 1.0 {
            for p in &mut probabilities {
                *p /= sum;
            }
        }
        let sum = probabilities.iter().fold(
            0.0,
            #[coverage(off)]
            |sum, p| sum + p,
        );
        assert!((sum - 1.0).abs() < 0.1);

        // Step 1 and 2
        let size = probabilities.len();
        let mut small = Vec::with_capacity(size);
        let mut large = Vec::with_capacity(size);
        let mut alias: Vec<usize> = vec![0; size];
        let mut prob: Vec<f64> = vec![0.0; size];

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

    /// Sample the Vose alias.
    ///
    /// It returns an index within `0` .. `original_probabilities.len()`.
    #[coverage(off)]
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
    #[coverage(off)]
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
    #[coverage(off)]
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
