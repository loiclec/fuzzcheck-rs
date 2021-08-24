use crate::and_sensor_and_pool::{AndPool, AndStats};
use crate::mutators::either::Either;
use crate::sensor_and_pool::{CompatibleWithSensor, CorpusDelta, Pool, Sensor, TestCase};
use crate::unique_coverage_pool::FeatureIdx;
use crate::unique_coverage_pool::{AnalyzedFeatureRef, UniqueCoveragePool};
use std::fmt::Display;

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

#[derive(Default)]
pub struct UniqueCoveragePoolObservationState {
    is_interesting: bool,
    analysis_result: AnalysisResult,
}
#[derive(Default)]
pub struct AnalysisResult {
    pub(crate) existing_features: Vec<FeatureIdx>,
    pub(crate) new_features: Vec<FeatureIdx>,
}

// could this be the CompatibleWithSensor trait?
pub trait HandleCoveragePointFromCodeCoverageSensor: Pool {
    type Observation;
    type ObservationState: Default;

    fn observe(&mut self, observation: &Self::Observation, input_complexity: f64, state: &mut Self::ObservationState);
    fn finish_observing(&mut self, state: &mut Self::ObservationState);
    fn is_interesting(&self, observation_state: &Self::ObservationState) -> bool;
    fn add(
        &mut self,
        data: Self::TestCase,
        complexity: f64,
        observation_state: Self::ObservationState,
        event_handler: impl FnMut(CorpusDelta<&Self::TestCase, Self::Index>, Self::Stats) -> Result<(), std::io::Error>,
    ) -> Result<(), std::io::Error>;
}

impl<T: TestCase> HandleCoveragePointFromCodeCoverageSensor for UniqueCoveragePool<T> {
    type Observation = (usize, u64);
    type ObservationState = UniqueCoveragePoolObservationState;

    #[no_coverage]
    fn observe(
        &mut self,
        &(index, counter): &Self::Observation,
        input_complexity: f64,
        state: &mut Self::ObservationState,
    ) {
        let feature_index = FeatureIdx::new(index, counter);
        let AnalyzedFeatureRef { least_complexity } = unsafe { self.features.get_unchecked(feature_index.0) };
        if let Some(prev_least_complexity) = least_complexity {
            self.existing_features.push(feature_index);
            if input_complexity < *prev_least_complexity {
                state.is_interesting = true;
            }
        } else {
            self.new_features.push(feature_index);
            state.is_interesting = true;
        }
    }
    #[no_coverage]
    fn finish_observing(&mut self, state: &mut Self::ObservationState) {
        if state.is_interesting {
            state.analysis_result.new_features = self.new_features.clone();
            state.analysis_result.existing_features = self.existing_features.clone();
        }
        self.new_features.clear();
        self.existing_features.clear();
    }
    #[no_coverage]
    fn is_interesting(&self, observation_state: &Self::ObservationState) -> bool {
        observation_state.is_interesting
    }
    #[no_coverage]
    fn add(
        &mut self,
        data: Self::TestCase,
        complexity: f64,
        observation_state: Self::ObservationState,
        mut event_handler: impl FnMut(CorpusDelta<&Self::TestCase, Self::Index>, Self::Stats) -> Result<(), std::io::Error>,
    ) -> Result<(), std::io::Error> {
        let result = observation_state.analysis_result;
        let delta = self.add(data, complexity, result);
        if let Some((delta, stats)) = delta {
            event_handler(delta, stats)?;
        }
        Ok(())
    }
}

impl<P1, P2> HandleCoveragePointFromCodeCoverageSensor for AndPool<P1, P2>
where
    P1: HandleCoveragePointFromCodeCoverageSensor,
    P2: HandleCoveragePointFromCodeCoverageSensor<Observation = P1::Observation, TestCase = P1::TestCase>,
{
    type Observation = P1::Observation;
    type ObservationState = (P1::ObservationState, P2::ObservationState);
    #[no_coverage]
    fn observe(&mut self, observation: &Self::Observation, input_complexity: f64, state: &mut Self::ObservationState) {
        self.p1.observe(observation, input_complexity, &mut state.0);
        self.p2.observe(observation, input_complexity, &mut state.1);
    }
    #[no_coverage]
    fn is_interesting(&self, observation_state: &Self::ObservationState) -> bool {
        self.p1.is_interesting(&observation_state.0) || self.p2.is_interesting(&observation_state.1)
    }
    #[no_coverage]
    fn finish_observing(&mut self, state: &mut Self::ObservationState) {
        self.p1.finish_observing(&mut state.0);
        self.p2.finish_observing(&mut state.1);
    }
    #[no_coverage]
    fn add(
        &mut self,
        data: Self::TestCase,
        complexity: f64,
        observation_state: Self::ObservationState,
        mut event_handler: impl FnMut(CorpusDelta<&Self::TestCase, Self::Index>, Self::Stats) -> Result<(), std::io::Error>,
    ) -> Result<(), std::io::Error> {
        let AndStats { stats1, stats2 } = self.stats();
        let (o1, o2) = observation_state;
        if self.p1.is_interesting(&o1) {
            self.p1.add(
                data.clone(),
                complexity,
                o1,
                #[no_coverage]
                |delta, stats1| {
                    let mut delta = Self::lift_corpus_delta_1(delta);
                    delta.path.push("a");
                    event_handler(
                        delta,
                        AndStats {
                            stats1,
                            stats2: stats2.clone(),
                        },
                    )?;
                    Ok(())
                },
            )?;
        }
        if self.p2.is_interesting(&o2) {
            self.p2.add(
                data,
                complexity,
                o2,
                #[no_coverage]
                |delta, stats2| {
                    let mut delta = Self::lift_corpus_delta_2(delta);
                    delta.path.push("b");
                    event_handler(
                        delta,
                        AndStats {
                            stats1: stats1.clone(),
                            stats2,
                        },
                    )?;
                    Ok(())
                },
            )?;
        }
        Ok(())
    }
}

impl<S, P> CompatibleWithSensor<S> for P
where
    S: for<'a> Sensor<ObservationHandler<'a> = &'a mut dyn FnMut(P::Observation)>,
    P: HandleCoveragePointFromCodeCoverageSensor,
{
    #[no_coverage]
    fn process(
        &mut self,
        sensor: &mut S,
        get_input_ref: Either<Self::Index, &Self::TestCase>,
        clone_input: &impl Fn(&Self::TestCase) -> Self::TestCase,
        complexity: f64,
        mut event_handler: impl FnMut(CorpusDelta<&Self::TestCase, Self::Index>, Self::Stats) -> Result<(), std::io::Error>,
    ) -> Result<(), std::io::Error> {
        let mut observation_state = <Self as HandleCoveragePointFromCodeCoverageSensor>::ObservationState::default();
        sensor.iterate_over_observations(
            #[no_coverage]
            &mut |o| {
                self.observe(&o, complexity, &mut observation_state);
            },
        );
        self.finish_observing(&mut observation_state);
        if self.is_interesting(&observation_state) {
            let input_cloned = {
                let input_ref = match get_input_ref {
                    Either::Left(idx) => self.get(idx),
                    Either::Right(input_ref) => input_ref,
                };
                clone_input(input_ref)
            };
            self.add(
                input_cloned,
                complexity,
                observation_state,
                #[no_coverage]
                |delta, stats| {
                    event_handler(delta, stats)?;
                    Ok(())
                },
            )?;
        }

        Ok(())
    }

    // TODO: minify shouldn't depend on the sensor, should only be part of the pool
    #[no_coverage]
    fn minify(
        &mut self,
        sensor: &mut S,
        target_len: usize,
        event_handler: impl FnMut(CorpusDelta<&Self::TestCase, Self::Index>, Self::Stats) -> Result<(), std::io::Error>,
    ) -> Result<(), std::io::Error> {
        todo!()
    }
}
