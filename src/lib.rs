use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use anyhow::Context;
use image::{EncodableLayout, ImageBuffer, Rgba};
use rayon::prelude::*;

use processing::{PathDefinitions, TestReport};
pub use setup::{changed_path, failures_path, new_path, old_path};

use crate::formatters::EmuTestResultFormatter;
use crate::inputs::TestCandidate;
use crate::options::EmuRunnerOptions;
use crate::outputs::{
    EmuContext, FrameOutput, RunnerError, RunnerOutput, RunnerOutputContext, TestOutput, TestOutputChanged,
    TestOutputContext, TestOutputError, TestOutputFailure, TestOutputPassed, TestOutputType, TestOutputUnchanged,
};

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
        F: Fn(&TestCandidate, Vec<u8>) -> Vec<FrameOutput> + Send + Sync + std::panic::RefUnwindSafe,
        I: ExactSizeIterator<Item = TestCandidate> + Send,
    {
        let start = Instant::now();
        let test_len = tests.len();
        self.formatter.handle_start(test_len)?;

        let frame_results = panics::run_in_custom_handler(|| {
            self.thread_pool.install(|| {
                tests
                    .par_bridge()
                    .map(|candidate| self.run_test_in_panic_handler(candidate, &emu_run))
                    .collect::<Vec<_>>()
            })
        });
        let test_results = self.thread_pool.install(|| {
            frame_results
                .into_par_iter()
                .flat_map(|runner_output| self.process_result(runner_output))
                .collect()
        });

        let report = TestReport::new(test_len, test_results);

        self.formatter.handle_complete(&report, start.elapsed())
    }

    fn run_test_in_panic_handler<F>(&self, candidate: TestCandidate, emu_run: &F) -> Result<RunnerOutput, RunnerError>
    where
        F: Fn(&TestCandidate, Vec<u8>) -> Vec<FrameOutput> + Send + Sync + std::panic::RefUnwindSafe,
    {
        let runner_output = std::fs::read(&candidate.rom_path)
            .context("Couldn't read ROM")
            .and_then(|rom_data| {
                let now = Instant::now();

                let frame = std::panic::catch_unwind(|| emu_run(&candidate, rom_data));

                let frame = match frame {
                    Ok(frame) => Ok(frame),
                    Err(_) => Err(anyhow::anyhow!(
                        "Caught an emulator panic: `{}`",
                        panics::latest_panic().unwrap()
                    )),
                }?;

                Ok(RunnerOutput {
                    candidate: candidate.clone(),
                    context: RunnerOutputContext {
                        time_taken: now.elapsed(),
                        frame_output: frame,
                    },
                })
            });

        let result = runner_output.map_err(|e| RunnerError { candidate, context: e });

        let _ = self.formatter.handle_test_progress(result.as_ref());

        result
    }

    fn process_result(&self, runner_output: Result<RunnerOutput, RunnerError>) -> Vec<TestOutput> {
        let runner_output = match runner_output {
            Ok(output) => output,
            Err(e) => {
                return vec![e.owned_map(|error| TestOutputContext {
                    time_taken: None,
                    output: TestOutputType::Error(TestOutputError {
                        reason: Arc::new(error),
                    }),
                })]
            }
        };

        // Generate the path definitions for *all* the test's context frames.
        self.frame_and_path_definitions(&runner_output)
            .map(|(frame, path_def)| match self.process_frame(frame, path_def) {
                Ok(output) => EmuContext {
                    candidate: runner_output.candidate.clone(),
                    context: TestOutputContext {
                        time_taken: Some(runner_output.context.time_taken),
                        output,
                    },
                },
                Err(e) => EmuContext {
                    candidate: runner_output.candidate.clone(),
                    context: TestOutputContext {
                        time_taken: Some(runner_output.context.time_taken),
                        output: TestOutputType::Error(TestOutputError { reason: Arc::new(e) }),
                    },
                },
            })
            .collect()
    }

    fn process_frame(&self, frame: &FrameOutput, path_def: PathDefinitions) -> anyhow::Result<TestOutputType> {
        let new_path = path_def.new_path()?;
        let old_path = path_def.old_path()?;
        let snapshot_path = path_def.snapshot_path()?;
        let image_frame = self.save_image(frame, &new_path)?;

        let old_equals_data = |new_data: &[u8]| {
            if old_path.exists() {
                image::open(&old_path)
                    .map(|data| data.as_bytes() == new_data)
                    .unwrap_or(false)
            } else {
                false
            }
        };

        let output = if snapshot_path.exists() {
            // Time to see if our snapshot is still correct
            let snapshot_data = image::open(&snapshot_path)?;
            if snapshot_data.as_bytes() != image_frame.as_bytes() {
                let new_failure_path = path_def.failed_path_with_suffix("fail")?;
                std::fs::copy(&new_path, &new_failure_path)?;

                if self.options.copy_comparison_image {
                    let expected_file_in_failure_path = path_def.failed_path_with_suffix("pass")?;
                    std::fs::copy(&snapshot_path, expected_file_in_failure_path)?;
                }

                TestOutputType::Failure(TestOutputFailure {
                    failure_path: new_failure_path,
                    snapshot_path,
                    is_new: old_equals_data(snapshot_data.as_bytes()),
                })
            } else {
                TestOutputType::Passed(TestOutputPassed {
                    is_new: !old_equals_data(snapshot_data.as_bytes()),
                })
            }
        } else {
            // Just check if there has been *any* change at all
            if !old_equals_data(image_frame.as_bytes()) {
                let changed_path = path_def.changed_path_with_suffix("new")?;
                std::fs::copy(&new_path, &changed_path)?;

                if self.options.copy_comparison_image {
                    let old_file_in_changed_path = path_def.changed_path_with_suffix("old")?;
                    std::fs::copy(&old_path, old_file_in_changed_path)?;
                }

                TestOutputType::Changed(TestOutputChanged { changed_path, old_path })
            } else {
                TestOutputType::Unchanged(TestOutputUnchanged {
                    newly_added: !old_path.exists(),
                })
            }
        };

        Ok(output)
    }

    fn save_image<'a>(
        &'a self,
        frame: &'a FrameOutput,
        path_to_save: &Path,
    ) -> anyhow::Result<ImageBuffer<Rgba<u8>, &'a [u8]>> {
        let image_frame = ImageBuffer::from_raw(
            self.options.expected_frame_width as u32,
            self.options.expected_frame_height as u32,
            frame.frame.0.as_slice(),
        )
        .context("Failed to turn framebuffer into a dynamic image")?;

        image_frame.save(path_to_save)?;

        Ok(image_frame)
    }

    /// Return an iterator which contains a tuple of a [FrameOutput] and the [PathDefinitions] where this frame may be saved
    /// after further classification.
    fn frame_and_path_definitions<'a>(
        &'a self,
        runner_output: &'a RunnerOutput,
    ) -> impl Iterator<Item = (&'a FrameOutput, PathDefinitions<'a>)> {
        let is_sequence_test = runner_output.context.frame_output.len() > 1;
        let create_subfolder = is_sequence_test && self.options.put_sequence_tests_in_subfolder;
        let rom_id = &runner_output.candidate.rom_id;

        runner_output.context.frame_output.iter().map(move |frame| {
            let frame_file_png = setup::rom_id_to_png(rom_id, frame.tag.as_deref());

            (
                frame,
                PathDefinitions::new(
                    &self.options.output_path,
                    &self.options.snapshot_path,
                    create_subfolder.then(|| Path::new(rom_id)),
                    frame_file_png,
                ),
            )
        })
    }
}
