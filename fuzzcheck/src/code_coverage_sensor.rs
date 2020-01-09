use crate::input_pool::*;

use std::mem::MaybeUninit;

use ahash::AHashSet;

type PC = usize;

pub static mut SHARED_SENSOR: MaybeUninit<CodeCoverageSensor> = MaybeUninit::<CodeCoverageSensor>::uninit();

pub fn shared_sensor() -> &'static mut CodeCoverageSensor {
    unsafe { &mut *SHARED_SENSOR.as_mut_ptr() }
}

pub struct CodeCoverageSensor {
    pub num_guards: isize,
    pub is_recording: bool,
    pub eight_bit_counters: &'static mut [u8],
    pub features: AHashSet<Feature>,
}

impl CodeCoverageSensor {
    pub fn handle_trace_cmp(&mut self, pc: PC, arg1: u64, arg2: u64) {
        let f = Feature::instruction(pc, arg1, arg2);
        self.features.insert(f);
    }
    pub fn handle_trace_indir(&mut self, caller: PC, callee: PC) {
        let f = Feature::indir(caller ^ callee);
        self.features.insert(f);
    }

    pub fn iterate_over_collected_features<F>(&mut self, mut handle: F)
    where
        F: FnMut(Feature) -> (),
    {
        for (i, x) in self.eight_bit_counters.iter().enumerate() {
            if *x == 0 {
                continue;
            } else {
                let f = Feature::edge(i, *x as u16);
                handle(f);
            }
        }
        for f in self.features.iter() {
            handle(*f);
        }
    }

    pub fn clear(&mut self) {
        for x in self.eight_bit_counters.iter_mut() {
            *x = 0;
        }
        self.features.clear();
    }
}
