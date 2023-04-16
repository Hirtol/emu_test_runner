use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::Context;
use processing::TestReport;
use rayon::prelude::*;

use crate::formatters::EmuTestResultFormatter;
use crate::inputs::TestCandidate;
use crate::options::EmuRunnerOptions;
use crate::outputs::{RgbaFrame, RunnerError, RunnerOutput, RunnerOutputContext};

pub mod formatters;
pub mod inputs;
pub mod options;
pub mod outputs;
mod panics;
mod processing;
mod setup;

pub struct EmuTestRunner {
    formatter: Box<dyn EmuTestResultFormatter + Send + Sync>,
    options: EmuRunnerOptions,
    thread_pool: rayon::ThreadPool,
}

impl EmuTestRunner {
    /// Instantiate a new test runner with the given formatter and options.
    ///
    /// Will create a new [rayon::ThreadPool] for executing the tests on.
    pub fn new(
        formatter: Box<dyn EmuTestResultFormatter + Send + Sync>,
        options: EmuRunnerOptions,
    ) -> anyhow::Result<Self> {
        setup::setup_output_directory(&options.output_path)?;
        setup::setup_snapshot_directory(&options.snapshot_path)?;

        let thread_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(options.num_threads.get())
            .build()?;

        Ok(Self {
            formatter,
            options,
            thread_pool,
        })
    }

    /// Run the given tests and pass the results to the `formatter`.
    ///
    /// Any panic that occurs during the test execution is caught and can be reported on by the `formatter`.
    pub fn run_tests<F, I>(&self, tests: I, emu_run: F) -> anyhow::Result<()>
    where
        F: Fn(&TestCandidate, Vec<u8>) -> RgbaFrame + Send + Sync + std::panic::RefUnwindSafe,
        I: ExactSizeIterator<Item = TestCandidate> + Send,
    {
        self.formatter.handle_start(tests.len())?;

        let start = Instant::now();

        let frame_results = panics::run_in_custom_handler(|| {
            self.thread_pool
                .install(|| self.run_tests_in_panic_handler(tests, emu_run))
        });
        let test_results = processing::process_results(
            frame_results,
            &self.options.output_path,
            &self.options.snapshot_path,
            self.options.expected_frame_width,
            self.options.expected_frame_height,
        );

        let report = TestReport::new(test_results);

        self.formatter.handle_complete(&report, start.elapsed())
    }

    fn run_tests_in_panic_handler<F, I>(&self, tests: I, emu_run: F) -> Vec<Result<RunnerOutput, RunnerError>>
    where
        F: Fn(&TestCandidate, Vec<u8>) -> RgbaFrame + Send + Sync + std::panic::RefUnwindSafe,
        I: ExactSizeIterator<Item = TestCandidate> + Send,
    {
        tests
            .par_bridge()
            .map(|rom| {
                let runner_output = std::fs::read(&rom.rom_path)
                    .context("Couldn't read ROM")
                    .and_then(|rom_data| {
                        let now = Instant::now();

                        let frame = std::panic::catch_unwind(|| emu_run(&rom, rom_data));

                        let frame = match frame {
                            Ok(frame) => Ok(frame),
                            Err(_) => Err(anyhow::anyhow!(
                                "Caught an emulator panic: `{}`",
                                panics::latest_panic().unwrap()
                            )),
                        }?;

                        Ok(RunnerOutput {
                            rom_path: rom.rom_path.clone(),
                            rom_id: rom.rom_id.clone(),
                            context: RunnerOutputContext {
                                time_taken: now.elapsed(),
                                frame_output: frame,
                            },
                        })
                    });

                let result = runner_output.map_err(|e| RunnerError {
                    rom_path: rom.rom_path,
                    rom_id: rom.rom_id,
                    context: e,
                });

                let _ = self.formatter.handle_test_progress(result.as_ref());

                result
            })
            .collect::<Vec<_>>()
    }
}

#[derive(Debug, Clone)]
pub enum OutputDestinations<'a> {
    Old,
    New,
    Failures,
    Changed,
    InMemory(&'a [u8]),
}

impl<'a> OutputDestinations<'a> {
    pub fn compare(&self, rom_id: &str, output_path: &Path, compare_to: OutputDestinations) -> anyhow::Result<bool> {
        let data = self.to_data(rom_id, output_path)?;
        let other_data = compare_to.to_data(rom_id, output_path)?;

        Ok(data == other_data)
    }

    pub fn to_path(&self, output_path: &Path) -> Option<PathBuf> {
        match self {
            OutputDestinations::Old => crate::setup::old_path(output_path).into(),
            OutputDestinations::New => crate::setup::new_path(output_path).into(),
            OutputDestinations::Failures => crate::setup::failures_path(output_path).into(),
            OutputDestinations::Changed => crate::setup::changed_path(output_path).into(),
            OutputDestinations::InMemory(_) => None,
        }
    }

    pub fn to_data(&self, rom_name: &str, output_path: &Path) -> anyhow::Result<Cow<'_, [u8]>> {
        if let OutputDestinations::InMemory(data) = self {
            Ok((*data).into())
        } else {
            let picture_name = format!("{}.png", rom_name);
            let path = self
                .to_path(output_path)
                .context("Failed to get path")?
                .join(picture_name);

            Ok(std::fs::read(path)?.into())
        }
    }
}
