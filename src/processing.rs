use std::path::Path;
use std::sync::Arc;

use image::{EncodableLayout, ImageBuffer};
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::outputs::{
    TestOutput, TestOutputChanged, TestOutputContext, TestOutputError, TestOutputFailure, TestOutputType,
};
use crate::{setup, RunnerError, RunnerOutput};

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

                image_frame.save(&new_path)?;

                let output = if let Some(snapshot) = setup::has_snapshot(&runner_output.rom_id, snapshot_dir) {
                    // Time to see if our snapshot is still correct
                    let snapshot_data = image::open(&snapshot)?;

                    if snapshot_data.as_bytes() != image_frame.as_bytes() {
                        let failure_path = setup::failures_path(output).join(&result_name);
                        std::fs::copy(&new_path, &failure_path)?;

                        TestOutputType::Failure(TestOutputFailure {
                            failure_path,
                            snapshot_path: snapshot,
                        })
                    } else {
                        TestOutputType::Passed
                    }
                } else {
                    // Just check if there has been *any* change at all
                    let old_path = setup::old_path(output).join(&result_name);

                    if old_path.exists() {
                        let old_data = image::open(&old_path)?;

                        if old_data.as_bytes() != image_frame.as_bytes() {
                            let changed_path = setup::changed_path(output).join(&result_name);
                            std::fs::copy(&new_path, &changed_path)?;

                            TestOutputType::Changed(TestOutputChanged { changed_path, old_path })
                        } else {
                            TestOutputType::Unchanged
                        }
                    } else {
                        TestOutputType::Unchanged
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
