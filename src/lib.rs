use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use anyhow::Context;
use image::{EncodableLayout, ImageBuffer};
use rayon::prelude::*;

use processing::TestReport;
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
        let test_len = tests.len();
        self.formatter.handle_start(test_len)?;

        let start = Instant::now();

        let frame_results = panics::run_in_custom_handler(|| {
            self.thread_pool
                .install(|| self.run_tests_in_panic_handler(tests, emu_run))
        });
        let test_results = self.thread_pool.install(|| self.process_results(frame_results));

        let report = TestReport::new(test_len, test_results);

        self.formatter.handle_complete(&report, start.elapsed())
    }

    fn run_tests_in_panic_handler<F, I>(&self, tests: I, emu_run: F) -> Vec<Result<RunnerOutput, RunnerError>>
    where
        F: Fn(&TestCandidate, Vec<u8>) -> Vec<FrameOutput> + Send + Sync + std::panic::RefUnwindSafe,
        I: ExactSizeIterator<Item = TestCandidate> + Send,
    {
        tests
            .par_bridge()
            .map(|candidate| {
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
            })
            .collect::<Vec<_>>()
    }

    pub fn process_results(&self, results: Vec<Result<RunnerOutput, RunnerError>>) -> Vec<TestOutput> {
        let output = &self.options.output_path;
        results
            .into_par_iter()
            .flat_map(|runner_output| {
                let runner_output = match runner_output {
                    Ok(output) => output,
                    Err(e) => return vec![e.into()],
                };
                let lambda = |frame: FrameOutput| {
                    let image_frame: ImageBuffer<image::Rgba<u8>, &[u8]> = if let Some(img) = ImageBuffer::from_raw(
                        self.options.expected_frame_width as u32,
                        self.options.expected_frame_height as u32,
                        frame.frame.0.as_slice(),
                    ) {
                        img
                    } else {
                        anyhow::bail!("Failed to turn framebuffer to dynamic image")
                    };

                    let result_name = setup::rom_id_to_png(&runner_output.candidate.rom_id, frame.tag.as_deref());
                    let path_suffix =
                        if runner_output.candidate.is_sequence_test && self.options.put_sequence_tests_in_subfolder {
                            Path::new(&runner_output.candidate.rom_id).join(result_name)
                        } else {
                            Path::new(&result_name).to_path_buf()
                        };

                    let new_path = setup::new_path(output).join(&path_suffix);
                    let old_path = setup::old_path(output).join(&path_suffix);

                    std::fs::create_dir_all(new_path.parent().unwrap());
                    std::fs::create_dir_all(old_path.parent().unwrap());

                    image_frame.save(&new_path)?;
                    let old_equals_data = |new_data: &[u8]| {
                        if old_path.exists() {
                            image::open(&old_path)
                                .map(|data| data.as_bytes() == new_data)
                                .unwrap_or(false)
                        } else {
                            false
                        }
                    };

                    let snapshot = self.options.snapshot_path.join(&path_suffix);
                    let output = if snapshot.exists() {
                        // Time to see if our snapshot is still correct
                        let snapshot_data = image::open(&snapshot)?;
                        if snapshot_data.as_bytes() != image_frame.as_bytes() {
                            let failure_path = setup::failures_path(output).join(&path_suffix);
                            std::fs::create_dir_all(failure_path.parent().unwrap());
                            std::fs::copy(&new_path, &failure_path)?;

                            TestOutputType::Failure(TestOutputFailure {
                                failure_path,
                                snapshot_path: snapshot,
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
                            let changed_path = setup::changed_path(output).join(&path_suffix);
                            std::fs::create_dir_all(changed_path.parent().unwrap());
                            std::fs::copy(&new_path, &changed_path)?;

                            TestOutputType::Changed(TestOutputChanged { changed_path, old_path })
                        } else {
                            TestOutputType::Unchanged(TestOutputUnchanged {
                                newly_added: !old_path.exists(),
                            })
                        }
                    };

                    Ok(output)
                };

                runner_output
                    .context
                    .frame_output
                    .into_iter()
                    .map(|frame| match lambda(frame) {
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
            })
            .collect()
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
