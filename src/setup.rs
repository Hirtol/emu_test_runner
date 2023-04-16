use std::path::{Path, PathBuf};

pub const NEW_DIR_NAME: &str = "new";
pub const OLD_DIR_NAME: &str = "old";
pub const CHANGED_DIR_NAME: &str = "changed";
pub const FAILED_DIR_NAME: &str = "failures";

/// Will clean and setup the directory structure in the output directory as follows:
///
///
/// * OUTPUT_DIR
///     * /new
///     * /old
///     * /changed
///     * /failures
pub fn setup_output_directory(output: &Path) -> anyhow::Result<()> {
    let new_dir = new_path(output);
    let old_dir = old_path(output);
    let changed_dir = changed_path(output);
    let failures = failures_path(output);

    let _ = std::fs::remove_dir_all(&old_dir);
    // Move the `new` dir to the `old`
    if new_dir.exists() {
        std::fs::rename(&new_dir, &old_dir)?;
    }

    let _ = std::fs::remove_dir_all(&changed_dir);
    let _ = std::fs::remove_dir_all(&failures);

    std::fs::create_dir_all(new_dir)?;
    std::fs::create_dir_all(changed_dir)?;
    std::fs::create_dir_all(failures)?;

    Ok(())
}

/// Setup the directory where one can save the Snapshots for tests.
///
/// A test with an associated snapshot will fail if it starts to differ from the established baseline.
pub fn setup_snapshot_directory(snapshot: &Path) -> anyhow::Result<()> {
    Ok(std::fs::create_dir_all(snapshot)?)
}

pub fn old_path(output: &Path) -> PathBuf {
    output.join(OLD_DIR_NAME)
}

pub fn new_path(output: &Path) -> PathBuf {
    output.join(NEW_DIR_NAME)
}

pub fn changed_path(output: &Path) -> PathBuf {
    output.join(CHANGED_DIR_NAME)
}

pub fn failures_path(output: &Path) -> PathBuf {
    output.join(FAILED_DIR_NAME)
}

pub fn rom_id_to_png(rom_id: &str, suffix: Option<&str>) -> String {
    if let Some(suffix) = suffix {
        format!("{rom_id}_{suffix}.png")
    } else {
        format!("{rom_id}.png")
    }
}
