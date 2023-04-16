use std::num::NonZeroUsize;
use std::path::PathBuf;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct EmuRunnerOptions {
    pub output_path: PathBuf,
    pub snapshot_path: PathBuf,
    pub num_threads: NonZeroUsize,
    pub expected_frame_width: usize,
    pub expected_frame_height: usize,

    pub put_sequence_tests_in_subfolder: bool,
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
        }
    }
}
