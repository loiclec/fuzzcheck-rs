#![cfg_attr(fuzzing, feature(coverage_attribute))]
use serde::{Deserialize, Serialize};

#[cfg_attr(fuzzing, derive(fuzzcheck::DefaultMutator))]
#[derive(Clone, Serialize, Deserialize)]
struct SampleStruct<T, U> {
    x: T,
    y: U,
}

#[cfg_attr(fuzzing, derive(fuzzcheck::DefaultMutator))]
#[derive(Clone, Serialize, Deserialize)]
enum SampleEnum {
    A(u16),
    B,
    C { x: bool, y: bool },
}

fn should_not_crash(xs: &[SampleStruct<u8, SampleEnum>]) {
    if xs.len() == 3
        && xs[0].x == 100
        && matches!(xs[0].y, SampleEnum::C { x: false, y: true })
        && xs[1].x == 55
        && matches!(xs[1].y, SampleEnum::C { x: true, y: false })
        && xs[2].x == 87
        && matches!(xs[2].y, SampleEnum::C { x: false, y: false })
        && xs[3].x == 24
        && matches!(xs[3].y, SampleEnum::C { x: true, y: true })
    {
        panic!()
    }
}

// fuzz tests reside along your other tests and have the #[test] attribute
#[cfg(all(fuzzing, test))]
mod tests {
    #[test]
    fn fuzz() {
        let result = fuzzcheck::fuzz_test(super::should_not_crash) // the test function to fuzz
            .default_mutator() // the mutator to generate values of &[SampleStruct<u8, SampleEnum>]
            .serde_serializer() // save the test cases to the file system using serde
            .default_sensor_and_pool() // gather observations using the default sensor (i.e. recording code coverage)
            .arguments_from_cargo_fuzzcheck() // take arguments from the cargo-fuzzcheck command line tool
            .stop_after_first_test_failure(true) // stop the fuzzer as soon as a test failure is found
            .launch();
        assert!(result.found_test_failure);
    }
}
