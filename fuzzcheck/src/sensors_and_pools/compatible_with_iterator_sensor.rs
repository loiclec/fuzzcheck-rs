use crate::{
    fuzzer::PoolStorageIndex,
    traits::{CompatibleWithSensor, CorpusDelta, Pool, Sensor},
};

pub trait CompatibleWithIteratorSensor: Pool {
    type Observation;
    type ObservationState: Default;

    fn observe(&mut self, observation: &Self::Observation, input_complexity: f64, state: &mut Self::ObservationState);
    fn finish_observing(&mut self, state: &mut Self::ObservationState, input_complexity: f64);
    fn is_interesting(&self, observation_state: &Self::ObservationState, input_complexity: f64) -> bool;
    fn add(
        &mut self,
        data: PoolStorageIndex,
        complexity: f64,
        observation_state: Self::ObservationState,
    ) -> Vec<CorpusDelta>;
}

impl<S, P> CompatibleWithSensor<S> for P
where
    S: for<'a> Sensor<ObservationHandler<'a> = &'a mut dyn FnMut(P::Observation)>,
    P: CompatibleWithIteratorSensor,
{
    #[no_coverage]
    fn process(&mut self, input_id: PoolStorageIndex, sensor: &mut S, complexity: f64) -> Vec<CorpusDelta> {
        let mut observation_state = <Self as CompatibleWithIteratorSensor>::ObservationState::default();
        sensor.iterate_over_observations(
            #[no_coverage]
            &mut |o| {
                self.observe(&o, complexity, &mut observation_state);
            },
        );
        self.finish_observing(&mut observation_state, complexity);
        if self.is_interesting(&observation_state, complexity) {
            self.add(input_id, complexity, observation_state)
        } else {
            vec![]
        }
    }
}
