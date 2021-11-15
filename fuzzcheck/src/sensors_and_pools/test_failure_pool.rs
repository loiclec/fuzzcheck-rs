use crate::fuzzer::PoolStorageIndex;
use crate::traits::{CompatibleWithSensor, CorpusDelta, Pool, SaveToStatsFolder, Sensor};
use crate::{CSVField, ToCSV};
use nu_ansi_term::Color;
use std::fmt::Display;
use std::path::PathBuf;

const NBR_ARTIFACTS_PER_ERROR_AND_CPLX: usize = 8;

pub(crate) static mut TEST_FAILURE: Option<TestFailure> = None;

/// A type describing a test failure.
///
/// It is uniquely identifiable through `self.id` and displayable through `self.display`.
#[derive(Debug, Clone)]
pub struct TestFailure {
    pub display: String,
    pub id: u64,
}

/// A sensor that records test failures.
///
/// It is [compatible with](CompatibleWithSensor) [`TestFailurePool`].
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
impl SaveToStatsFolder for TestFailureSensor {
    #[no_coverage]
    fn save_to_stats_folder(&self) -> Vec<(PathBuf, Vec<u8>)> {
        vec![]
    }
}

#[derive(Clone, Copy)]
pub struct TestFailurePoolStats {
    count: usize,
}
impl Display for TestFailurePoolStats {
    #[no_coverage]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.count == 0 {
            write!(f, "{}", format!("failures({})", self.count))
        } else {
            write!(f, "{}", Color::Red.paint(format!("failures({})", self.count)))
        }
    }
}
impl ToCSV for TestFailurePoolStats {
    #[no_coverage]
    fn csv_headers(&self) -> Vec<CSVField> {
        vec![CSVField::String("test_failures_count".to_string())]
    }
    #[no_coverage]
    fn to_csv_record(&self) -> Vec<CSVField> {
        vec![CSVField::Integer(self.count as isize)]
    }
}

struct TestFailureList {
    error: TestFailure,
    inputs: Vec<TestFailureListForError>,
}

struct TestFailureListForError {
    cplx: f64,
    inputs: Vec<PoolStorageIndex>,
}

/// A pool that saves failing test cases.
///
/// It categorizes the test cases by their failure information and sort them by complexity.
///
/// It is [compatible with](crate::CompatibleWithSensor) [`TestFailureSensor`]
pub struct TestFailurePool {
    name: String,
    inputs: Vec<TestFailureList>,
    rng: fastrand::Rng,
}

impl TestFailurePool {
    #[no_coverage]
    pub(crate) fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            inputs: vec![],
            rng: fastrand::Rng::new(),
        }
    }
}

impl Pool for TestFailurePool {
    type Stats = TestFailurePoolStats;

    #[no_coverage]
    fn stats(&self) -> Self::Stats {
        TestFailurePoolStats {
            count: self.inputs.len(),
        }
    }

    #[no_coverage]
    fn len(&self) -> usize {
        self.inputs.len()
    }

    #[no_coverage]
    fn get_random_index(&mut self) -> Option<PoolStorageIndex> {
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
        Some(least_complexity.inputs[input_choice])
    }

    #[no_coverage]
    fn mark_test_case_as_dead_end(&mut self, idx: PoolStorageIndex) {
        for x in self.inputs.iter_mut() {
            for x in x.inputs.iter_mut() {
                if let Some(i) = x.inputs.iter().position(
                    #[no_coverage]
                    |&x| x == idx,
                ) {
                    x.inputs.remove(i);
                }
            }
        }
    }
}
impl SaveToStatsFolder for TestFailurePool {
    #[no_coverage]
    fn save_to_stats_folder(&self) -> Vec<(PathBuf, Vec<u8>)> {
        vec![]
    }
}

impl CompatibleWithSensor<TestFailureSensor> for TestFailurePool {
    #[no_coverage]
    fn process(
        &mut self,
        input_idx: PoolStorageIndex,
        sensor: &mut TestFailureSensor,
        complexity: f64,
    ) -> Vec<CorpusDelta> {
        let mut error = None;
        sensor.iterate_over_observations(&mut error);

        enum PositionOfNewInput {
            NewError,
            ExistingErrorNewCplx(usize),
            ExistingErrorAndCplx(usize),
        }

        let mut is_interesting = None;
        if let Some(error) = error {
            if let Some(list_index) = self.inputs.iter().position(
                #[no_coverage]
                |xs| xs.error.id == error.id,
            ) {
                let list = &self.inputs[list_index];
                if let Some(least_complex) = list.inputs.last() {
                    if least_complex.cplx > complexity {
                        is_interesting = Some(PositionOfNewInput::ExistingErrorNewCplx(list_index));
                    } else if least_complex.cplx == complexity {
                        if least_complex.inputs.len() < NBR_ARTIFACTS_PER_ERROR_AND_CPLX
                            && self
                                .inputs
                                .iter()
                                .position(
                                    #[no_coverage]
                                    |xs| xs.error.display == error.display,
                                )
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
                let mut path = PathBuf::new();
                path.push(&self.name);
                path.push(format!("{}", error.id));
                path.push(format!("{:.4}", complexity));

                match position {
                    PositionOfNewInput::NewError => {
                        self.inputs.push(TestFailureList {
                            error,
                            inputs: vec![TestFailureListForError {
                                cplx: complexity,
                                inputs: vec![input_idx],
                            }],
                        });
                    }
                    PositionOfNewInput::ExistingErrorNewCplx(error_idx) => {
                        // TODO: handle event
                        self.inputs[error_idx].inputs.push(TestFailureListForError {
                            cplx: complexity,
                            inputs: vec![input_idx],
                        });
                    }
                    PositionOfNewInput::ExistingErrorAndCplx(error_idx) => {
                        // NOTE: the complexity must be the last one
                        // TODO: handle event
                        self.inputs[error_idx].inputs.last_mut().unwrap().inputs.push(input_idx);
                    }
                };

                let delta = CorpusDelta {
                    path,
                    add: true,
                    remove: vec![],
                };
                return vec![delta];
            }
        }
        vec![]
    }
}
