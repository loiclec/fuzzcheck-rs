use std::{fmt::Display, marker::PhantomData};

use crate::{
    mutators::either::Either,
    sensor_and_pool::{CorpusDelta, Pool, Sensor, SensorAndPool},
};

pub struct AndPool<P1, P2>
where
    P1: Pool,
    P2: Pool<TestCase = P1::TestCase>,
{
    pub p1: P1,
    pub p2: P2,

    pub percent_choose_first: usize,
    pub rng: fastrand::Rng,
}

impl<P1, P2> Pool for AndPool<P1, P2>
where
    P1: Pool,
    P2: Pool<TestCase = P1::TestCase>,
{
    type TestCase = P1::TestCase;
    type Index = Either<P1::Index, P2::Index>;
    #[no_coverage]
    fn len(&self) -> usize {
        self.p1.len() + self.p2.len()
    }
    #[no_coverage]
    fn get_random_index(&mut self) -> Option<Self::Index> {
        if self.rng.usize(0..100) < self.percent_choose_first {
            if let Some(idx) = self.p1.get_random_index().map(Either::Left) {
                Some(idx)
            } else {
                self.p2.get_random_index().map(Either::Right)
            }
        } else {
            if let Some(idx) = self.p2.get_random_index().map(Either::Right) {
                Some(idx)
            } else {
                self.p1.get_random_index().map(Either::Left)
            }
        }
    }
    #[no_coverage]
    fn get(&self, idx: Self::Index) -> &Self::TestCase {
        match idx {
            Either::Left(idx) => self.p1.get(idx),
            Either::Right(idx) => self.p2.get(idx),
        }
    }
    #[no_coverage]
    fn get_mut(&mut self, idx: Self::Index) -> &mut Self::TestCase {
        match idx {
            Either::Left(idx) => self.p1.get_mut(idx),
            Either::Right(idx) => self.p2.get_mut(idx),
        }
    }
    #[no_coverage]
    fn retrieve_after_processing(&mut self, idx: Self::Index, generation: usize) -> Option<&mut Self::TestCase> {
        match idx {
            Either::Left(idx) => self.p1.retrieve_after_processing(idx, generation),
            Either::Right(idx) => self.p2.retrieve_after_processing(idx, generation),
        }
    }
    #[no_coverage]
    fn mark_test_case_as_dead_end(&mut self, idx: Self::Index) {
        match idx {
            Either::Left(idx) => self.p1.mark_test_case_as_dead_end(idx),
            Either::Right(idx) => self.p2.mark_test_case_as_dead_end(idx),
        }
    }
}

pub struct AndSensor<S1, S2>
where
    S1: Sensor,
    S2: Sensor,
{
    pub s1: S1,
    pub s2: S2,
}
impl<S1, S2> Sensor for AndSensor<S1, S2>
where
    S1: Sensor,
    S2: Sensor,
{
    #[no_coverage]
    fn start_recording(&mut self) {
        self.s1.start_recording();
        self.s2.start_recording();
    }
    #[no_coverage]
    fn stop_recording(&mut self) {
        self.s1.stop_recording();
        self.s2.stop_recording();
    }
}

#[derive(Default, Clone)]
pub struct AndStats<S1: Display, S2: Display> {
    pub stats1: S1,
    pub stats2: S2,
}
impl<S1: Display, S2: Display> Display for AndStats<S1, S2> {
    #[no_coverage]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}\t{}", self.stats1, self.stats2)
    }
}

pub struct AndSensorAndPool<SP1, SP2>
where
    SP1: SensorAndPool,
    SP2: SensorAndPool<TestCase = SP1::TestCase>,
{
    _phantom: PhantomData<(SP1, SP2)>,
}
impl<SP1, SP2> SensorAndPool for AndSensorAndPool<SP1, SP2>
where
    SP1: SensorAndPool,
    SP2: SensorAndPool<TestCase = SP1::TestCase>,
{
    type Sensor = AndSensor<SP1::Sensor, SP2::Sensor>;
    type Pool = AndPool<SP1::Pool, SP2::Pool>;
    type TestCase = SP1::TestCase;
    type Event = Either<SP1::Event, SP2::Event>;
    type Stats = AndStats<SP1::Stats, SP2::Stats>;

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
        {
            let AndPool { p1, p2, .. } = pool;
            let AndStats { stats2, .. } = stats;

            let get_input_1 = match get_input_ref {
                Either::Left(Either::Right(idx)) => Either::Right(p2.get(idx)),
                Either::Left(Either::Left(idx)) => Either::Left(idx),
                Either::Right(input_ref) => Either::Right(input_ref),
            };

            SP1::process(
                &mut sensor.s1,
                p1,
                &mut stats.stats1,
                get_input_1,
                clone_input,
                complexity,
                #[no_coverage]
                |corpus_delta, stats1| {
                    event_handler(
                        Self::lift_corpus_delta_1(corpus_delta),
                        &AndStats {
                            stats1: stats1.clone(),
                            stats2: stats2.clone(),
                        },
                    )
                },
            )?;
        }
        {
            let AndPool { p1, p2, .. } = pool;
            let AndStats { stats1, .. } = stats;

            let get_input_2 = match get_input_ref {
                Either::Left(Either::Left(idx)) => Either::Right(p1.get(idx)),
                Either::Left(Either::Right(idx)) => Either::Left(idx),
                Either::Right(input_ref) => Either::Right(input_ref),
            };

            SP2::process(
                &mut sensor.s2,
                p2,
                &mut stats.stats2,
                get_input_2,
                clone_input,
                complexity,
                #[no_coverage]
                |corpus_delta, stats2| {
                    event_handler(
                        Self::lift_corpus_delta_2(corpus_delta),
                        &AndStats {
                            stats1: stats1.clone(),
                            stats2: stats2.clone(),
                        },
                    )
                },
            )?
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
        {
            let AndStats { stats1, stats2 } = stats;

            SP1::minify(
                &mut sensor.s1,
                &mut pool.p1,
                stats1,
                target_len,
                #[no_coverage]
                |corpus_delta, stats1| {
                    event_handler(
                        Self::lift_corpus_delta_1(corpus_delta),
                        &AndStats {
                            stats1: stats1.clone(),
                            stats2: stats2.clone(),
                        },
                    )
                },
            )
        }
        {
            let AndStats { stats1, stats2 } = stats;

            SP2::minify(
                &mut sensor.s2,
                &mut pool.p2,
                stats2,
                target_len,
                #[no_coverage]
                |corpus_delta, stats2| {
                    event_handler(
                        Self::lift_corpus_delta_2(corpus_delta),
                        &AndStats {
                            stats1: stats1.clone(),
                            stats2: stats2.clone(),
                        },
                    )
                },
            )
        }
    }
    #[no_coverage]
    fn get_corpus_delta_from_event(
        pool: &Self::Pool,
        event: Self::Event,
    ) -> CorpusDelta<&Self::TestCase, <Self::Pool as Pool>::Index> {
        match event {
            Either::Left(event) => Self::lift_corpus_delta_1(SP1::get_corpus_delta_from_event(&pool.p1, event)),
            Either::Right(event) => Self::lift_corpus_delta_2(SP2::get_corpus_delta_from_event(&pool.p2, event)),
        }
    }
}
impl<SP1, SP2> AndSensorAndPool<SP1, SP2>
where
    SP1: SensorAndPool,
    SP2: SensorAndPool<TestCase = SP1::TestCase>,
{
    #[no_coverage]
    fn lift_corpus_delta_1(
        corpus_delta: CorpusDelta<&SP1::TestCase, <SP1::Pool as Pool>::Index>,
    ) -> CorpusDelta<&<Self as SensorAndPool>::TestCase, <<Self as SensorAndPool>::Pool as Pool>::Index> {
        CorpusDelta {
            add: corpus_delta.add.map(
                #[no_coverage]
                |(content, idx)| (content, Either::Left(idx)),
            ),
            remove: corpus_delta
                .remove
                .into_iter()
                .map(
                    #[no_coverage]
                    |idx| Either::Left(idx),
                )
                .collect(),
        }
    }
    #[no_coverage]
    fn lift_corpus_delta_2(
        corpus_delta: CorpusDelta<&SP2::TestCase, <SP2::Pool as Pool>::Index>,
    ) -> CorpusDelta<&<Self as SensorAndPool>::TestCase, <<Self as SensorAndPool>::Pool as Pool>::Index> {
        CorpusDelta {
            add: corpus_delta.add.map(
                #[no_coverage]
                |(content, idx)| (content, Either::Right(idx)),
            ),
            remove: corpus_delta
                .remove
                .into_iter()
                .map(
                    #[no_coverage]
                    |idx| Either::Right(idx),
                )
                .collect(),
        }
    }
}
