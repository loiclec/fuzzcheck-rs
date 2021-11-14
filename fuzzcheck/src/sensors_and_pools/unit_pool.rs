use std::path::PathBuf;

use crate::fuzzer::PoolStorageIndex;
use crate::sensors_and_pools::stats::EmptyStats;
use crate::traits::{CompatibleWithSensor, CorpusDelta, Pool, Sensor};

/// A pool that stores only one given test case.
///
/// Currently, it can only be used by fuzzcheck itself
/// because it requires a `PoolStorageIndex`, which only
/// fuzzcheck can create. This will change at some point.
pub struct UnitPool {
    input_index: PoolStorageIndex,
    dead_end: bool,
}
impl UnitPool {
    #[no_coverage]
    pub(crate) fn new(input_index: PoolStorageIndex) -> Self {
        Self {
            input_index,
            dead_end: false,
        }
    }
}

impl Pool for UnitPool {
    type Stats = EmptyStats;
    #[no_coverage]
    fn stats(&self) -> Self::Stats {
        EmptyStats
    }

    #[no_coverage]
    fn len(&self) -> usize {
        1
    }
    #[no_coverage]
    fn get_random_index(&mut self) -> Option<PoolStorageIndex> {
        if self.dead_end {
            None
        } else {
            Some(self.input_index)
        }
    }
    #[no_coverage]
    fn mark_test_case_as_dead_end(&mut self, _idx: PoolStorageIndex) {
        self.dead_end = true
    }

    #[no_coverage]
    fn serialized(&self) -> Vec<(PathBuf, Vec<u8>)> {
        vec![]
    }
}

impl<S: Sensor> CompatibleWithSensor<S> for UnitPool {
    #[no_coverage]
    fn process(&mut self, _input_id: PoolStorageIndex, _sensor: &mut S, _complexity: f64) -> Vec<CorpusDelta> {
        vec![]
    }
}
