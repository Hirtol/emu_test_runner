use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

pub struct TestReport {
    pub test_outputs: Vec<TestOutput>,
    pub passed: Vec<TestPassed>,
    pub unchanged: Vec<TestUnchanged>,
    pub fails: Vec<TestFailed>,
    pub changed: Vec<TestChanged>,
    pub errors: Vec<TestError>,
}

impl TestReport {
    pub(crate) fn new(test_outputs: Vec<TestOutput>) -> Self {
        let (mut passed, mut fails, mut unchanged, mut changed, mut errors) = (vec![], vec![], vec![], vec![], vec![]);

        for report in test_outputs.clone() {
            let rom_path = report.rom_path;
            let rom_id = report.rom_id;
            let ctx = report.context;

            match ctx.output {
                TestOutputType::Unchanged => unchanged.push(TestUnchanged {
                    rom_path,
                    rom_id,
                    context: TestOutputContext {
                        time_taken: ctx.time_taken,
                        output: (),
                    },
                }),
                TestOutputType::Changed(changes) => changed.push(TestChanged {
                    rom_path,
                    rom_id,
                    context: TestOutputContext {
                        time_taken: ctx.time_taken,
                        output: changes,
                    },
                }),
                TestOutputType::Failure(fail) => fails.push(TestFailed {
                    rom_path,
                    rom_id,
                    context: TestOutputContext {
                        time_taken: ctx.time_taken,
                        output: fail,
                    },
                }),
                TestOutputType::Passed => passed.push(TestPassed {
                    rom_path,
                    rom_id,
                    context: TestOutputContext {
                        time_taken: ctx.time_taken,
                        output: (),
                    },
                }),
                TestOutputType::Error(error) => errors.push(TestError {
                    rom_path,
                    rom_id,
                    context: error,
                }),
            }
        }

        Self {
            test_outputs,
            passed,
            unchanged,
            fails,
            changed,
            errors,
        }
    }
}

pub type TestPassed = EmuContext<TestOutputContext<()>>;
pub type TestUnchanged = EmuContext<TestOutputContext<()>>;
pub type TestFailed = EmuContext<TestOutputContext<TestOutputFailure>>;
pub type TestError = EmuContext<TestOutputError>;
pub type TestChanged = EmuContext<TestOutputContext<TestOutputChanged>>;

pub type TestOutput = EmuContext<TestOutputContext<TestOutputType>>;
pub type RunnerError = EmuContext<anyhow::Error>;
pub type RunnerOutput = EmuContext<RunnerOutputContext>;

#[derive(Debug, Clone)]
pub struct EmuContext<T> {
    pub rom_path: PathBuf,
    pub rom_id: String,
    pub context: T,
}

impl<T> EmuContext<T> {
    pub fn map<E, F: FnOnce(T) -> E>(self, op: F) -> EmuContext<E> {
        EmuContext {
            rom_path: self.rom_path,
            rom_id: self.rom_id,
            context: op(self.context),
        }
    }
}

#[derive(Debug)]
pub struct RunnerOutputContext {
    pub time_taken: Duration,
    pub frame_output: RgbaFrame,
}

#[derive(Debug, Clone)]
pub struct TestOutputContext<T> {
    pub time_taken: Option<Duration>,
    pub output: T,
}

#[derive(Debug, Clone)]
pub enum TestOutputType {
    Unchanged,
    Changed(TestOutputChanged),
    Failure(TestOutputFailure),
    Passed,
    Error(TestOutputError),
}

#[derive(Debug, Clone)]
pub struct TestOutputFailure {
    pub failure_path: PathBuf,
    pub snapshot_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct TestOutputChanged {
    pub changed_path: PathBuf,
    pub old_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct TestOutputError {
    pub reason: Arc<anyhow::Error>,
}

#[derive(Debug)]
pub struct RgbaFrame(pub Vec<u8>);
