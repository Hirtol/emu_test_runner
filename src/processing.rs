use std::path::{Path, PathBuf};

use anyhow::Context;

use crate::outputs::{
    TestChanged, TestError, TestFailed, TestOutput, TestOutputContext, TestOutputType, TestPassed, TestUnchanged,
};

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

pub struct PathDefinitions<'a> {
    output_path: &'a Path,
    snapshot_path: &'a Path,
    path_suffix: PathBuf,
    create_subfolder: bool,
}

impl<'a> PathDefinitions<'a> {
    pub fn new(output_path: &'a Path, snapshot_path: &'a Path, path_suffix: PathBuf, create_subfolder: bool) -> Self {
        PathDefinitions {
            output_path,
            snapshot_path,
            path_suffix,
            create_subfolder,
        }
    }

    pub fn new_path(&self) -> anyhow::Result<PathBuf> {
        self.check_and_create(crate::new_path(self.output_path).join(&self.path_suffix))
    }

    pub fn old_path(&self) -> anyhow::Result<PathBuf> {
        self.check_and_create(crate::old_path(self.output_path).join(&self.path_suffix))
    }

    pub fn changed_path(&self) -> anyhow::Result<PathBuf> {
        self.check_and_create(crate::changed_path(self.output_path).join(&self.path_suffix))
    }

    pub fn failed_path(&self) -> anyhow::Result<PathBuf> {
        self.check_and_create(crate::failures_path(self.output_path).join(&self.path_suffix))
    }

    pub fn snapshot_path(&self) -> anyhow::Result<PathBuf> {
        self.check_and_create(self.snapshot_path.join(&self.path_suffix))
    }

    fn check_and_create(&self, path: PathBuf) -> anyhow::Result<PathBuf> {
        if self.create_subfolder {
            std::fs::create_dir_all(path.parent().context("Couldn't retrieve parent")?)?;
        }

        Ok(path)
    }
}
