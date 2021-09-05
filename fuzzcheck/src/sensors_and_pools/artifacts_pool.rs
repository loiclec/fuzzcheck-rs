use crate::mutators::either::Either;
use crate::traits::{CompatibleWithSensor, CorpusDelta, Pool, Sensor, TestCase};
use owo_colors::OwoColorize;
use std::fmt::Display;
use std::path::PathBuf;

const NBR_ARTIFACTS_PER_ERROR_AND_CPLX: usize = 8;

pub(crate) static mut TEST_FAILURE: Option<TestFailure> = None;

#[derive(Debug, Clone)]
pub struct TestFailure {
    pub display: String,
    pub id: u64,
}

#[derive(Default)]
pub struct TestFailureSensor {
    error: Option<TestFailure>,
}
impl Sensor for TestFailureSensor {
    type ObservationHandler<'a> = &'a mut Option<TestFailure>;

    #[no_coverage]
    fn start_recording(&mut self) {
        self.error = None;
        unsafe {
            TEST_FAILURE = None;
        }
    }

    #[no_coverage]
    fn stop_recording(&mut self) {
        unsafe {
            self.error = TEST_FAILURE.clone();
        }
    }

    #[no_coverage]
    fn iterate_over_observations(&mut self, handler: Self::ObservationHandler<'_>) {
        *handler = std::mem::take(&mut self.error);
    }
}

#[derive(Clone, Copy, Default)]
pub(crate) struct Stats {
    count: usize,
}
impl Display for Stats {
    #[no_coverage]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.count == 0 {
            write!(f, "{}", format!("artifacts({})", self.count))
        } else {
            write!(f, "{}", format!("artifacts({})", self.count).red())
        }
    }
}

#[derive(Clone)]
pub(crate) struct Input<T> {
    generation: usize,
    data: T,
}

struct ArftifactList<T> {
    error: TestFailure,
    inputs: Vec<ArtifactListForError<T>>,
}

struct ArtifactListForError<T> {
    cplx: f64,
    inputs: Vec<Input<T>>,
}

pub(crate) struct ArtifactsPool<T> {
    name: String,
    inputs: Vec<ArftifactList<T>>,
    rng: fastrand::Rng,
}

impl<T> ArtifactsPool<T> {
    #[no_coverage]
    pub(crate) fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            inputs: vec![],
            rng: fastrand::Rng::new(),
        }
    }
}

impl<T: TestCase> Pool for ArtifactsPool<T> {
    type TestCase = T;
    type Index = (usize, usize, usize);
    type Stats = Stats;

    #[no_coverage]
    fn stats(&self) -> Self::Stats {
        Stats {
            count: self.inputs.len(),
        }
    }

    #[no_coverage]
    fn len(&self) -> usize {
        self.inputs.len()
    }

    #[no_coverage]
    fn get_random_index(&mut self) -> Option<Self::Index> {
        if self.inputs.is_empty() {
            return None;
        }
        let error_choice = self.rng.usize(0..self.inputs.len());
        let list_for_error = &self.inputs[error_choice];
        let complexity_choice = list_for_error.inputs.len() - 1;
        let least_complexity = &list_for_error.inputs[complexity_choice];
        if least_complexity.inputs.is_empty() {
            return None;
        }
        let input_choice = self.rng.usize(0..least_complexity.inputs.len());
        Some((error_choice, complexity_choice, input_choice))
    }

    #[no_coverage]
    fn get(&self, idx: Self::Index) -> &Self::TestCase {
        &self.inputs[idx.0].inputs[idx.1].inputs[idx.2].data
    }

    #[no_coverage]
    fn get_mut(&mut self, idx: Self::Index) -> &mut Self::TestCase {
        &mut self.inputs[idx.0].inputs[idx.1].inputs[idx.2].data
    }

    #[no_coverage]
    fn retrieve_after_processing(&mut self, idx: Self::Index, generation: usize) -> Option<&mut Self::TestCase> {
        if let Some(input) = self.inputs[idx.0]
            .inputs
            .get_mut(idx.1)
            .map(|inputs| inputs.inputs.get_mut(idx.2))
            .flatten()
        {
            if input.data.generation() == generation {
                Some(&mut input.data)
            } else {
                None
            }
        } else {
            None
        }
    }

    #[no_coverage]
    fn mark_test_case_as_dead_end(&mut self, idx: Self::Index) {
        self.inputs[idx.0].inputs[idx.1].inputs.remove(idx.2);
    }

    #[no_coverage]
    fn minify(
        &mut self,
        _target_len: usize,
        _event_handler: impl FnMut(CorpusDelta<&Self::TestCase, Self::Index>, Self::Stats) -> Result<(), std::io::Error>,
    ) -> Result<(), std::io::Error> {
        // TODO
        Ok(())
    }
}
impl<T> CompatibleWithSensor<TestFailureSensor> for ArtifactsPool<T>
where
    T: TestCase,
{
    #[no_coverage]
    fn process(
        &mut self,
        sensor: &mut TestFailureSensor,
        get_input_ref: crate::mutators::either::Either<Self::Index, &Self::TestCase>,
        clone_input: &impl Fn(&Self::TestCase) -> Self::TestCase,
        complexity: f64,
        mut event_handler: impl FnMut(CorpusDelta<&Self::TestCase, Self::Index>, Self::Stats) -> Result<(), std::io::Error>,
    ) -> Result<(), std::io::Error> {
        let mut error = None;
        sensor.iterate_over_observations(&mut error);

        enum PositionOfNewInput {
            NewError,
            ExistingErrorNewCplx(usize),
            ExistingErrorAndCplx(usize),
        }

        let mut is_interesting = None;
        if let Some(error) = error {
            if let Some(list_index) = self.inputs.iter().position(|xs| xs.error.id == error.id) {
                let list = &self.inputs[list_index];
                if let Some(least_complex) = list.inputs.last() {
                    if least_complex.cplx > complexity {
                        is_interesting = Some(PositionOfNewInput::ExistingErrorNewCplx(list_index));
                    } else if least_complex.cplx == complexity {
                        if least_complex.inputs.len() < NBR_ARTIFACTS_PER_ERROR_AND_CPLX
                            && self
                                .inputs
                                .iter()
                                .position(|xs| xs.error.display == error.display)
                                .is_none()
                        {
                            is_interesting = Some(PositionOfNewInput::ExistingErrorAndCplx(list_index));
                        }
                    }
                } else {
                    is_interesting = Some(PositionOfNewInput::ExistingErrorNewCplx(list_index));
                }
            } else {
                // a new error we haven't seen before
                is_interesting = Some(PositionOfNewInput::NewError);
            }
            if let Some(position) = is_interesting {
                let data = match get_input_ref {
                    Either::Left(x) => {
                        let input = &self.inputs[x.0].inputs[x.1].inputs[x.2].data;
                        clone_input(input)
                    }
                    Either::Right(x) => clone_input(x),
                };
                let input = Input {
                    generation: 0,
                    data: data,
                };
                let mut path = PathBuf::new();
                path.push(&self.name);
                path.push(format!("{}", error.id));
                path.push(format!("{:.4}", complexity));

                let new_index = match position {
                    PositionOfNewInput::NewError => {
                        self.inputs.push(ArftifactList {
                            error,
                            inputs: vec![ArtifactListForError {
                                cplx: complexity,
                                inputs: vec![input],
                            }],
                        });

                        (self.inputs.len() - 1, 0, 0)
                    }
                    PositionOfNewInput::ExistingErrorNewCplx(error_idx) => {
                        // TODO: handle event
                        self.inputs[error_idx].inputs.push(ArtifactListForError {
                            cplx: complexity,
                            inputs: vec![input],
                        });
                        (error_idx, self.inputs[error_idx].inputs.len() - 1, 0)
                    }
                    PositionOfNewInput::ExistingErrorAndCplx(error_idx) => {
                        // NOTE: the complexity must be the last one
                        // TODO: handle event
                        self.inputs[error_idx].inputs.last_mut().unwrap().inputs.push(input);
                        (
                            error_idx,
                            self.inputs[error_idx].inputs.len() - 1,
                            self.inputs[error_idx].inputs.last().unwrap().inputs.len() - 1,
                        )
                    }
                };
                let data = self.get(new_index);
                let delta = CorpusDelta {
                    path,
                    add: Some((data, new_index)),
                    remove: vec![],
                };
                event_handler(delta, self.stats())?;
            }
        }
        Ok(())
    }
}
