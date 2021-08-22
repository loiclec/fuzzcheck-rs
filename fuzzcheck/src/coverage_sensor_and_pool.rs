use std::fmt::Display;
use std::marker::PhantomData;
use std::ops::Range;

use crate::code_coverage_sensor::CodeCoverageSensor;
use crate::data_structures::SlabKey;
use crate::mutators::either::Either;
use crate::sensor_and_pool::{CorpusDelta, Pool, SensorAndPool, TestCase};
use crate::unique_coverage_pool::{AnalyzedFeature, AnalyzedFeatureRef, UniqueCoveragePool, UniqueCoveragePoolEvent};
use crate::Feature;

pub struct CodeCoverageSensorAndPool<T> {
    _phantom: PhantomData<T>,
}

impl<T: TestCase> SensorAndPool for CodeCoverageSensorAndPool<T> {
    type Sensor = CodeCoverageSensor;
    type Pool = UniqueCoveragePool<T>;
    type TestCase = T;
    type Event = UniqueCoveragePoolEvent<T>;
    type Stats = FuzzerStats;

    #[no_coverage]
    fn process(
        sensor: &mut Self::Sensor,
        pool: &mut Self::Pool,
        stats: &mut Self::Stats,
        get_input_ref: Either<<Self::Pool as Pool>::Index, &Self::TestCase>,
        clone_input: &impl Fn(&Self::TestCase) -> Self::TestCase,
        complexity: f64,
        mut event_handler: impl FnMut(
            CorpusDelta<&Self::TestCase, <Self::Pool as Pool>::Index>,
            &Self::Stats,
        ) -> Result<(), std::io::Error>,
    ) -> Result<(), std::io::Error> {
        if let Some(result) = Self::analyze(sensor, pool, complexity) {
            let input_cloned = {
                let input_ref = match get_input_ref {
                    Either::Left(idx) => pool.get(idx),
                    Either::Right(input_ref) => input_ref,
                };
                clone_input(input_ref)
            };
            if let (Some(event), _) = pool.add(input_cloned, complexity, result, &sensor.index_ranges) {
                Self::update_stats(stats, pool, sensor);
                let corpus_delta = Self::get_corpus_delta_from_event(pool, event);
                event_handler(corpus_delta, stats)?;
            }
        }
        Ok(())
    }
    #[no_coverage]
    fn minify(
        sensor: &mut Self::Sensor,
        pool: &mut Self::Pool,
        stats: &mut Self::Stats,
        target_len: usize,
        mut event_handler: impl FnMut(CorpusDelta<&Self::TestCase, <Self::Pool as Pool>::Index>, &Self::Stats),
    ) {
        while pool.len() > target_len {
            let event = pool.remove_lowest_scoring_input();
            if let Some(event) = event {
                Self::update_stats(stats, pool, sensor);
                let corpus_delta = Self::get_corpus_delta_from_event(pool, event);
                event_handler(corpus_delta, stats);
            } else {
                break;
            }
        }
    }
    #[no_coverage]
    fn get_corpus_delta_from_event<'a>(
        pool: &'a Self::Pool,
        event: Self::Event,
    ) -> CorpusDelta<&'a Self::TestCase, <Self::Pool as Pool>::Index> {
        let UniqueCoveragePoolEvent {
            added_key,
            removed_keys,
        } = event;

        CorpusDelta {
            add: added_key.map(|key| (pool.get(key), key)),
            remove: removed_keys,
        }
    }
}

#[derive(Clone, Copy, Default)]
pub struct FuzzerStats {
    pub score: f64,
    pub pool_size: usize,
    pub avg_cplx: f64,
    pub percent_coverage: f64,
}
impl Display for FuzzerStats {
    #[no_coverage]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "score: {:.2}\tcov:{:.2}%\tpool:{}\tcplx:{:.2}",
            self.score,
            self.percent_coverage * 100.0,
            self.pool_size,
            self.avg_cplx
        )
    }
}

pub(crate) struct AnalysisResult<T> {
    pub existing_features: Vec<SlabKey<AnalyzedFeature<T>>>,
    pub new_features: Vec<Feature>,
}

impl<T: TestCase> CodeCoverageSensorAndPool<T> {
    #[no_coverage]
    fn update_stats(stats: &mut FuzzerStats, pool: &mut UniqueCoveragePool<T>, sensor: &mut CodeCoverageSensor) {
        stats.pool_size = pool.len();
        stats.score = pool.score();
        stats.avg_cplx = pool.average_complexity as f64;
        stats.percent_coverage = pool.feature_groups.len() as f64 / sensor.count_instrumented as f64;
    }
    #[no_coverage]
    fn analyze(
        sensor: &mut CodeCoverageSensor,
        pool: &mut UniqueCoveragePool<T>,
        cur_input_cplx: f64,
    ) -> Option<AnalysisResult<T>> {
        let mut best_input_for_a_feature = false;

        let pool_slab_features = &pool.slab_features;

        unsafe {
            for i in 0..sensor.coverage.len() {
                let Range { start, end } = pool.features_range_for_coverage_index.get_unchecked(i).clone();
                let features = &pool.features;
                let mut idx = start;
                sensor.iterate_over_collected_features(
                    i,
                    #[no_coverage]
                    |collected_feature| loop {
                        if idx < end {
                            let AnalyzedFeatureRef {
                                feature: pool_feature,
                                key,
                            } = *features.get_unchecked(idx);
                            if pool_feature < collected_feature {
                                idx += 1;
                                continue;
                            } else if pool_feature == collected_feature {
                                if cur_input_cplx < pool_slab_features[key].least_complexity {
                                    best_input_for_a_feature = true;
                                }
                                break;
                            } else {
                                best_input_for_a_feature = true;
                                break;
                            }
                        } else {
                            best_input_for_a_feature = true;
                            break;
                        }
                    },
                );
            }
        }
        if best_input_for_a_feature {
            let mut existing_features = Vec::new();
            let mut new_features = Vec::new();

            unsafe {
                for i in 0..sensor.coverage.len() {
                    let Range { start, end } = pool.features_range_for_coverage_index.get_unchecked(i);
                    let features = &pool.features;
                    let mut idx = *start;
                    let end = *end;
                    sensor.iterate_over_collected_features(
                        i,
                        #[no_coverage]
                        |feature| loop {
                            if idx < end {
                                let f_iter = features.get_unchecked(idx);
                                if f_iter.feature < feature {
                                    idx += 1;
                                    continue;
                                } else if f_iter.feature == feature {
                                    existing_features.push(f_iter.key);
                                    break;
                                } else {
                                    new_features.push(feature);
                                    break;
                                }
                            } else {
                                new_features.push(feature);
                                break;
                            }
                        },
                    );
                }
                Some(AnalysisResult {
                    existing_features,
                    new_features,
                })
            }
        } else {
            None
        }
    }
}
