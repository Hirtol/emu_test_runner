use std::path::Path;
use std::sync::Arc;

use image::{EncodableLayout, ImageBuffer};
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::outputs::{TestChanged, TestError, TestFailed, TestOutput, TestOutputChanged, TestOutputContext, TestOutputError, TestOutputFailure, TestOutputPassed, TestOutputType, TestOutputUnchanged, TestPassed, TestUnchanged};
use crate::{RunnerError, RunnerOutput, setup};

pub fn process_results(
    results: Vec<Result<RunnerOutput, RunnerError>>,
    output: &Path,
    snapshot_dir: &Path,
    frame_width: usize,
    frame_height: usize,
) -> Vec<TestOutput> {
    results
        .into_par_iter()
        .map(|runner_output| {
            let runner_output = match runner_output {
                Ok(output) => output,
                Err(e) => return e.into(),
            };
            let lambda = || {
                let image_frame: ImageBuffer<image::Rgba<u8>, &[u8]> = if let Some(img) = image::ImageBuffer::from_raw(
                    frame_width as u32,
                    frame_height as u32,
                    runner_output.context.frame_output.0.as_slice(),
                ) {
                    img
                } else {
                    anyhow::bail!("Failed to turn framebuffer to dynamic image")
                };

                let result_name = format!("{}.png", &runner_output.rom_id);
                let new_path = setup::new_path(output).join(&result_name);
                let old_path = setup::old_path(output).join(&result_name);

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

                let output = if let Some(snapshot) = setup::has_snapshot(&runner_output.rom_id, snapshot_dir) {
                    // Time to see if our snapshot is still correct
                    let snapshot_data = image::open(&snapshot)?;
                    if snapshot_data.as_bytes() != image_frame.as_bytes() {
                        let failure_path = setup::failures_path(output).join(&result_name);
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
                        let changed_path = setup::changed_path(output).join(&result_name);
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

            match lambda() {
                Ok(output) => runner_output.map(|context| TestOutputContext {
                    time_taken: Some(context.time_taken),
                    output,
                }),
                Err(e) => runner_output.map(|context| TestOutputContext {
                    time_taken: Some(context.time_taken),
                    output: TestOutputType::Error(TestOutputError { reason: Arc::new(e) }),
                }),
            }
        })
        .collect()
}

impl From<RunnerError> for TestOutput {
    fn from(value: RunnerError) -> Self {
        value.map(|error| TestOutputContext {
            time_taken: None,
            output: TestOutputType::Error(TestOutputError {
                reason: Arc::new(error),
            }),
        })
    }
}

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
                TestOutputType::Unchanged(same) => unchanged.push(TestUnchanged {
                    rom_path,
                    rom_id,
                    context: TestOutputContext {
                        time_taken: ctx.time_taken,
                        output: same,
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
                TestOutputType::Passed(pass) => passed.push(TestPassed {
                    rom_path,
                    rom_id,
                    context: TestOutputContext {
                        time_taken: ctx.time_taken,
                        output: pass,
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
