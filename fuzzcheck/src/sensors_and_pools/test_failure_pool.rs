use std::fmt::Display;
use std::path::PathBuf;

use nu_ansi_term::Color;

use crate::traits::{CompatibleWithObservations, CorpusDelta, Pool, SaveToStatsFolder, Sensor, Stats};
use crate::{CSVField, PoolStorageIndex, ToCSV};

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
#[derive(Default)]
pub struct TestFailureSensor {
    error: Option<TestFailure>,
}

impl Sensor for TestFailureSensor {
    type Observations = Option<TestFailure>;

    #[coverage(off)]
    fn start_recording(&mut self) {
        self.error = None;
        unsafe {
            TEST_FAILURE = None;
        }
    }

    #[coverage(off)]
    fn stop_recording(&mut self) {
        unsafe {
            self.error = TEST_FAILURE.clone();
        }
    }

    #[coverage(off)]
    fn get_observations(&mut self) -> Option<TestFailure> {
        std::mem::take(&mut self.error)
    }
}
impl SaveToStatsFolder for TestFailureSensor {
    #[coverage(off)]
    fn save_to_stats_folder(&self) -> Vec<(PathBuf, Vec<u8>)> {
        vec![]
    }
}

#[derive(Clone, Copy)]
pub struct TestFailurePoolStats {
    pub count: usize,
}
impl Display for TestFailurePoolStats {
    #[coverage(off)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.count == 0 {
            write!(f, "failures({})", self.count)
        } else {
            write!(f, "{}", Color::Red.paint(format!("failures({})", self.count)))
        }
    }
}
impl ToCSV for TestFailurePoolStats {
    #[coverage(off)]
    fn csv_headers(&self) -> Vec<CSVField> {
        vec![CSVField::String("test_failures_count".to_string())]
    }
    #[coverage(off)]
    fn to_csv_record(&self) -> Vec<CSVField> {
        vec![CSVField::Integer(self.count as isize)]
    }
}
impl Stats for TestFailurePoolStats {}

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
pub struct TestFailurePool {
    name: String,
    inputs: Vec<TestFailureList>,
    rng: fastrand::Rng,
}

impl TestFailurePool {
    #[coverage(off)]
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

    #[coverage(off)]
    fn stats(&self) -> Self::Stats {
        TestFailurePoolStats {
            count: self.inputs.len(),
        }
    }

    #[coverage(off)]
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
}
impl SaveToStatsFolder for TestFailurePool {
    #[coverage(off)]
    fn save_to_stats_folder(&self) -> Vec<(PathBuf, Vec<u8>)> {
        cfg_if::cfg_if! {
            if #[cfg(feature = "serde_json_serializer")]
            {
                let path = PathBuf::new().join("test_failures.json");
                let content = serde_json::to_string(&self.inputs.iter().map(
                    #[coverage(off)]
                    |tf| (tf.error.id, tf.error.display.clone()) ).collect::<Vec<_>>()).unwrap();
                vec![(path, content.into_bytes())]
            } else {
                vec![]
            }
        }
    }
}

impl CompatibleWithObservations<Option<TestFailure>> for TestFailurePool {
    #[coverage(off)]
    fn process(
        &mut self,
        input_idx: PoolStorageIndex,
        observations: &Option<TestFailure>,
        complexity: f64,
    ) -> Vec<CorpusDelta> {
        let error = observations;

        enum PositionOfNewInput {
            NewError,
            ExistingErrorNewCplx(usize),
            ExistingErrorAndCplx(usize),
        }

        let mut is_interesting = None;
        if let Some(error) = error {
            if let Some(list_index) = self.inputs.iter().position(
                #[coverage(off)]
                |xs| xs.error.id == error.id,
            ) {
                let list = &self.inputs[list_index];
                if let Some(least_complex) = list.inputs.last() {
                    if least_complex.cplx > complexity {
                        is_interesting = Some(PositionOfNewInput::ExistingErrorNewCplx(list_index));
                    } else if least_complex.cplx == complexity {
                        if least_complex.inputs.len() < NBR_ARTIFACTS_PER_ERROR_AND_CPLX
                            && !self.inputs.iter().any(
                                #[coverage(off)]
                                |xs| xs.error.display == error.display,
                            )
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
                            error: error.clone(),
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
