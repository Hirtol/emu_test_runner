use std::time::Duration;

use crate::inputs::TestCandidate;
pub use indicatif;

use crate::outputs::{RunnerError, RunnerOutput};
use crate::processing::TestReport;

pub mod simple;

pub trait EmuTestResultFormatter {
    /// Create the start of a report, usually indicating how many tests are about to be ran.
    fn handle_start(&self, test_count: usize) -> anyhow::Result<()>;

    /// Called whenever a test is about to start executing
    ///
    /// Note that this can be called from several threads at the same time.
    fn handle_test_start(&self, test: &TestCandidate) -> anyhow::Result<()>;

    /// Called whenever a test has been completed.
    ///
    /// Note that this can be called from several threads at the same time.
    ///
    /// Can be used to show a progress bar if desired.
    fn handle_test_finish(&self, test_complete: Result<&RunnerOutput, &RunnerError>) -> anyhow::Result<()>;

    /// Handle the final report, containing all tests and the results thereof.
    ///
    /// # Arguments
    /// * `report` - The report containing all the categories of test results
    /// * `time_taken` - The time it took for the *entire* test suite to run from a user perspective.
    /// Individual tests also have a `time_taken` variable for that particular test.
    fn handle_complete(&self, report: &TestReport, time_taken: Duration) -> anyhow::Result<()>;
}
