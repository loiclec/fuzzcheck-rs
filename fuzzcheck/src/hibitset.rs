
const SIZE: usize = 0b1 << 30;
const L0_SIZE: usize = 0b1 << 24;
const L1_SIZE: usize = 0b1 << 18;
const L2_SIZE: usize = 0b1 << 12;
const L3_SIZE: usize = 0b1 << 6;

pub struct HBitSet {
    l0: Vec<u64>,
    l1: Vec<u64>, 
    l2: Vec<u64>,
    l3: Vec<u64>, 
}

impl HBitSet {
    pub fn new() -> Self {
        Self {
            l0: std::iter::repeat(0).take(L0_SIZE).collect(),
            l1: std::iter::repeat(0).take(L1_SIZE).collect(),
            l2: std::iter::repeat(0).take(L2_SIZE).collect(),
            l3: std::iter::repeat(0).take(L3_SIZE).collect(),
        }
    }

    #[inline]
    pub fn set(&mut self, mut idx: usize) {
        // assert!(idx < SIZE);

        let bit = 0b1 << (idx % 64);
        idx /= 64;
        unsafe { *self.l0.get_unchecked_mut(idx) |= bit; }

        let bit = 0b1 << (idx % 64);
        idx /= 64;
        unsafe { *self.l1.get_unchecked_mut(idx) |= bit; }

        let bit = 0b1 << (idx % 64);
        idx /= 64;
        unsafe { *self.l2.get_unchecked_mut(idx) |= bit; }

        let bit = 0b1 << (idx % 64);
        idx /= 64;
        unsafe { *self.l3.get_unchecked_mut(idx) |= bit; }
    }

    pub fn test(&self, el: usize) -> bool {
        let (idx, bit) = (el / 64, el % 64);

        self.l0[idx] & (0b1 << bit) != 0
    }

    pub fn drain(&mut self, mut f: impl FnMut(u64)) {

        for (idx, map) in self.l3.iter_mut().enumerate() {
            if *map == 0 { 
                continue 
            }
            for bit in 0 .. 64 {
                if *map & (0b1 << bit) == 0 { continue }

                let inner_idx = idx * 64 + bit;

                for (idx, map) in self.l2[inner_idx .. inner_idx+(64- bit)].iter_mut().enumerate() {
                    if *map == 0 { 
                        continue 
                    }
                    for bit in 0 .. 64 {
                        if *map & (0b1 << bit) == 0 { continue }

                        let inner_idx = (inner_idx + idx) * 64 + bit;

                        for (idx, map) in self.l1[inner_idx .. inner_idx+(64-bit)].iter_mut().enumerate() {
                            if *map == 0 { 
                                continue 
                            }
                            for bit in 0 .. 64 {
                                if *map & (0b1 << bit) == 0 { continue }
                                
                                let inner_idx = (inner_idx + idx) * 64 + bit;

                                for (idx, map) in self.l0[inner_idx .. inner_idx+(64-bit)].iter_mut().enumerate() {
                                    if *map == 0 { 
                                        continue 
                                    }
                                    let element = ((inner_idx + idx) as u64) * 64;
                                    for bit in 0 .. 64 {
                                        if *map & (0b1 << bit) != 0 { 
                                            f(element + bit);
                                        }
                                    }

                                    *map = 0;
                                }
                            }

                            *map = 0;
                        }
                    }
        
                    *map = 0;
                }
            }

            *map = 0;
        }

    }
}
