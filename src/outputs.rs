use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

pub type TestOutput = EmuContext<TestOutputContext<TestOutputType>>;

pub type TestPassed = EmuContext<TestOutputContext<TestOutputPassed>>;
pub type TestUnchanged = EmuContext<TestOutputContext<TestOutputUnchanged>>;
pub type TestFailed = EmuContext<TestOutputContext<TestOutputFailure>>;
pub type TestError = EmuContext<TestOutputError>;
pub type TestChanged = EmuContext<TestOutputContext<TestOutputChanged>>;

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
    Unchanged(TestOutputUnchanged),
    Changed(TestOutputChanged),
    Failure(TestOutputFailure),
    Passed(TestOutputPassed),
    Error(TestOutputError),
}

#[derive(Debug, Clone)]
pub struct TestOutputUnchanged {
    pub newly_added: bool,
}

#[derive(Debug, Clone)]
pub struct TestOutputPassed {
    pub is_new: bool,
}

#[derive(Debug, Clone)]
pub struct TestOutputFailure {
    pub failure_path: PathBuf,
    pub snapshot_path: PathBuf,
    pub is_new: bool,
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

/// A single frame from the emulator, with the implicit assumption that:
/// 
/// `frame.len() == emu.FRAME_WIDTH * emu.FRAME_HEIGHT`
/// 
/// Bytes are expected in RGBA format, so one pixel is 32 bits.
#[derive(Debug)]
pub struct RgbaFrame(pub Vec<u8>);
