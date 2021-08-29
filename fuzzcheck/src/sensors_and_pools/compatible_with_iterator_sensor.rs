use crate::{
    mutators::either::Either,
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
        data: Self::TestCase,
        complexity: f64,
        observation_state: Self::ObservationState,
        event_handler: impl FnMut(CorpusDelta<&Self::TestCase, Self::Index>, Self::Stats) -> Result<(), std::io::Error>,
    ) -> Result<(), std::io::Error>;
}

impl<S, P> CompatibleWithSensor<S> for P
where
    S: for<'a> Sensor<ObservationHandler<'a> = &'a mut dyn FnMut(P::Observation)>,
    P: CompatibleWithIteratorSensor,
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
        let mut observation_state = <Self as CompatibleWithIteratorSensor>::ObservationState::default();
        sensor.iterate_over_observations(
            #[no_coverage]
            &mut |o| {
                self.observe(&o, complexity, &mut observation_state);
            },
        );
        self.finish_observing(&mut observation_state, complexity);
        if self.is_interesting(&observation_state, complexity) {
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
