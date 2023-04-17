use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::time::Duration;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct EmuRunnerOptions {
    pub output_path: PathBuf,
    pub snapshot_path: PathBuf,
    pub num_threads: NonZeroUsize,
    pub expected_frame_width: usize,
    pub expected_frame_height: usize,
    /// Whenever a test has more than 1 produced image this will put those together in a sub-folder.
    pub put_sequence_tests_in_subfolder: bool,
    /// Put a copy of a comparison image in the failed/changed directory for easy comparison.
    pub copy_comparison_image: bool,
    /// How long the entire test suite is allowed to take before the process is forcefully killed.
    pub timeout: Option<Duration>,
}

impl Default for EmuRunnerOptions {
    fn default() -> Self {
        Self {
            output_path: PathBuf::from("./test_output"),
            snapshot_path: PathBuf::from("./test_roms/expected"),
            num_threads: std::thread::available_parallelism().expect("Couldn't get available threads"),
            expected_frame_width: 240,
            expected_frame_height: 160,
            put_sequence_tests_in_subfolder: true,
            copy_comparison_image: true,
            timeout: Some(Duration::from_secs(15)),
        }
    }
}
