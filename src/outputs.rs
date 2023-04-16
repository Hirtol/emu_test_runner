use std::fmt::{Debug, Formatter};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use crate::inputs::TestCandidate;

pub type TestOutput = EmuContext<TestOutputContext<TestOutputType>>;

pub type TestPassed = EmuContext<TestOutputContext<TestOutputPassed>>;
pub type TestUnchanged = EmuContext<TestOutputContext<TestOutputUnchanged>>;
pub type TestFailed = EmuContext<TestOutputContext<TestOutputFailure>>;
pub type TestError = EmuContext<TestOutputError>;
pub type TestChanged = EmuContext<TestOutputContext<TestOutputChanged>>;

pub type RunnerError = EmuContext<anyhow::Error>;
/// One [RunnerOutput] is a single test, with potentially multiple sub-tests due to being a sequence-test.
pub type RunnerOutput = EmuContext<RunnerOutputContext>;

#[derive(Debug, Clone)]
pub struct EmuContext<T> {
    pub candidate: TestCandidate,
    pub context: T,
}

impl<T> EmuContext<T> {
    pub fn map<E, F: FnOnce(&T) -> E>(&self, op: F) -> EmuContext<E> {
        EmuContext {
            candidate: self.candidate.clone(),
            context: op(&self.context),
        }
    }

    pub fn owned_map<E, F: FnOnce(T) -> E>(self, op: F) -> EmuContext<E> {
        EmuContext {
            candidate: self.candidate,
            context: op(self.context),
        }
    }
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

#[derive(Debug)]
pub struct RunnerOutputContext {
    pub time_taken: Duration,
    pub frame_output: Vec<FrameOutput>,
}

/// The output produced by a test.
///
/// A test can produce multiple instances of `FrameOutput`. This marks the test as a `sequence` test.
/// This can be useful if you need to perform some inputs on your test rom, and want to periodically make `FrameOutputs` to
/// ensure the intermediate results look correct as well.
#[derive(Debug)]
pub struct FrameOutput {
    pub tag: Option<String>,
    pub frame: RgbaFrame,
}

/// A single frame from the emulator, with the implicit assumption that:
///
/// `frame.len() == emu.FRAME_WIDTH * emu.FRAME_HEIGHT`
///
/// Bytes are expected in RGBA format, so one pixel is 32 bits.
pub struct RgbaFrame(pub Vec<u8>);

impl Debug for RgbaFrame {
    fn fmt(&self, f: &mut Formatter) -> ::core::fmt::Result {
        Formatter::debug_tuple(f, "RgbaFrame").field(&self.0.len()).finish()
    }
}
