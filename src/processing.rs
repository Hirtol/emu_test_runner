use std::sync::Arc;

use crate::outputs::{
    TestChanged, TestError, TestFailed, TestOutput, TestOutputContext, TestOutputError, TestOutputType, TestPassed,
    TestUnchanged,
};
use crate::RunnerError;

impl From<RunnerError> for TestOutput {
    fn from(value: RunnerError) -> Self {
        value.owned_map(|error| TestOutputContext {
            time_taken: None,
            output: TestOutputType::Error(TestOutputError {
                reason: Arc::new(error),
            }),
        })
    }
}

pub struct TestReport {
    pub original_tests_count: usize,
    pub test_outputs: Vec<TestOutput>,
    pub passed: Vec<TestPassed>,
    pub unchanged: Vec<TestUnchanged>,
    pub fails: Vec<TestFailed>,
    pub changed: Vec<TestChanged>,
    pub errors: Vec<TestError>,
}

impl TestReport {
    pub(crate) fn new(original_tests_count: usize, test_outputs: Vec<TestOutput>) -> Self {
        let (mut passed, mut fails, mut unchanged, mut changed, mut errors) = (vec![], vec![], vec![], vec![], vec![]);

        for report in test_outputs.clone() {
            let candidate = report.candidate;
            let ctx = report.context;

            match ctx.output {
                TestOutputType::Unchanged(same) => unchanged.push(TestUnchanged {
                    candidate,
                    context: TestOutputContext {
                        time_taken: ctx.time_taken,
                        output: same,
                    },
                }),
                TestOutputType::Changed(changes) => changed.push(TestChanged {
                    candidate,
                    context: TestOutputContext {
                        time_taken: ctx.time_taken,
                        output: changes,
                    },
                }),
                TestOutputType::Failure(fail) => fails.push(TestFailed {
                    candidate,
                    context: TestOutputContext {
                        time_taken: ctx.time_taken,
                        output: fail,
                    },
                }),
                TestOutputType::Passed(pass) => passed.push(TestPassed {
                    candidate,
                    context: TestOutputContext {
                        time_taken: ctx.time_taken,
                        output: pass,
                    },
                }),
                TestOutputType::Error(error) => errors.push(TestError {
                    candidate,
                    context: error,
                }),
            }
        }

        Self {
            original_tests_count,
            test_outputs,
            passed,
            unchanged,
            fails,
            changed,
            errors,
        }
    }
}
