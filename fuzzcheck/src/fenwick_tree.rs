pub struct FenwickTree {
    storage: Vec<f64>,
}

#[inline(always)]
fn least_significant_bit(i: usize) -> usize {
    i & (1_usize.wrapping_add(!i))
}
#[inline(always)]
fn get_parent(i: usize) -> usize {
    i - least_significant_bit(i)
}
#[inline(always)]
fn get_next(i: usize) -> usize {
    i + least_significant_bit(i)
}

impl FenwickTree {
    pub fn new(mut xs: Vec<f64>) -> Self {
        let mut i = 1;
        while i < xs.len() {
            let j = get_next(i);
            if j < xs.len() {
                xs[j] += xs[i];
            }
            i += 1;
        }
        Self { storage: xs }
    }
    pub fn len(&self) -> usize {
        self.storage.len()
    }
    pub fn prefix_sum(&self, mut idx: usize) -> f64 {
        assert!(!self.storage.is_empty());
        let mut sum = self.storage[0];
        while idx != 0 {
            sum += self.storage[idx];
            idx = get_parent(idx);
        }
        sum
    }
    pub fn update(&mut self, mut idx: usize, delta: f64) {
        if idx == 0 {
            self.storage[idx] += delta;
            return;
        }
        while idx < self.storage.len() {
            self.storage[idx] += delta;
            idx = get_next(idx);
        }
    }
}

/*
Note: I can pad the tree with zeros if its size is not a power of two
this will make it possible to use this method below to query it
is it worth it?

// Find the largest i with prefix_sum(i) <= value.
// NOTE: Requires that all values are non-negative!
unsigned int rank_query(int value) {
    int i = 0, j = SIZE - 1;
    // j is a power of 2.

    for (; j > 0;  j >>= 1) {
        if (i + j < SIZE && A[i + j] <= value) {
            value -= A[i + j];
            i += j;
        }
    }
    return i;
}
*/

#[cfg(test)]
mod tests {
    use super::FenwickTree;

    #[test]
    fn test_basic_1() {
        let cumulative_probabilities = vec![2.0, 4.0, 1.0, 0.0, 1.2];
        let mut tree = FenwickTree::new(cumulative_probabilities);
        for i in 0..tree.storage.len() {
            println!("{}", tree.prefix_sum(i));
        }
        println!("===");
        tree.update(0, -0.5);
        for i in 0..tree.storage.len() {
            println!("{}", tree.prefix_sum(i));
        }
        println!("===");
        tree.update(1, 0.5);
        for i in 0..tree.storage.len() {
            println!("{}", tree.prefix_sum(i));
        }
        println!("===");
        tree.update(3, 1.);
        for i in 0..tree.storage.len() {
            println!("{}", tree.prefix_sum(i));
        }
    }
}
