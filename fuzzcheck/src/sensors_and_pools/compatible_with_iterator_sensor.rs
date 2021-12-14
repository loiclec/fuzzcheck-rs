use crate::{
    fuzzer::PoolStorageIndex,
    traits::{CompatibleWithSensor, CorpusDelta, Pool, Sensor},
};

/// A trait for [pools](Pool) that helps to implement [CompatibleWithSensor] for sensors whose
/// [observation handler](Sensor::ObservationHandler) is a closure of type `&'a mut dyn FnMut(P::Observation)>`.
///
/// It splits the [`self.process(..)`](CompatibleWithSensor::process) method into
/// * [`self.start_observing(..)`](CompatibleWithIteratorSensor::start_observing) called once at the beginning
/// * [`self.observe(..)`](CompatibleWithIteratorSensor::observe) called repeatedly for each observation
/// * [`self.finish_observing(..)`](CompatibleWithIteratorSensor::finish_observing) called once at the end
/// * [`self.is_interesting(..)`](CompatibleWithIteratorSensor::is_interesting) to evaluate whether the input should be added to the pool
/// * [`self.add(..)`](CompatibleWithIteratorSensor::add) to add the input to the pool, if necessary\
///
/// [`AndPool<P1, P2>`](crate::sensors_and_pools::AndPool) automatically implements `CompatibleWithIteratorSensor` if both
/// `P1` and `P2` implement it too and their [`Observation`](CompatibleWithIteratorSensor::Observation) associated type is the same.
/// Thus, indirectly, it also automatically implements [`CompatibleWithSensor<S>`](CompatibleWithSensor) for `S` compatible with `P1` and `P2`
pub trait CompatibleWithIteratorSensor: Pool {
    type Observation;
    type ObservationState;

    fn start_observing(&mut self) -> Self::ObservationState;
    fn observe(&mut self, observation: &Self::Observation, input_complexity: f64, state: &mut Self::ObservationState);
    // fn finish_observing(&mut self, state: &mut Self::ObservationState, input_complexity: f64);
    // fn is_interesting(&self, observation_state: &Self::ObservationState, input_complexity: f64) -> bool;

    fn add_if_interesting(
        &mut self,
        data: PoolStorageIndex,
        complexity: f64,
        observation_state: Self::ObservationState,
        observations: &[Self::Observation],
    ) -> Vec<CorpusDelta>;
}

impl<S, P> CompatibleWithSensor<S> for P
where
    S: for<'a> Sensor<Observations<'a> = &'a [P::Observation]>,
    P: CompatibleWithIteratorSensor,
{
    #[no_coverage]
    fn process(&mut self, input_id: PoolStorageIndex, sensor: &mut S, complexity: f64) -> Vec<CorpusDelta> {
        let mut observation_state = self.start_observing();
        let observations = sensor.get_observations();
        for o in observations {
            self.observe(&o, complexity, &mut observation_state);
        }

        self.add_if_interesting(input_id, complexity, observation_state, observations)
    }
}
