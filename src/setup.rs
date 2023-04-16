use std::path::{Path, PathBuf};

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

pub fn old_path(output: &Path) -> PathBuf {
    output.join("old")
}

pub fn new_path(output: &Path) -> PathBuf {
    output.join("new")
}

pub fn changed_path(output: &Path) -> PathBuf {
    output.join("changed")
}

pub fn failures_path(output: &Path) -> PathBuf {
    output.join("failures")
}

pub fn has_snapshot(rom_name: &str, snapshot_dir: &Path) -> Option<PathBuf> {
    let snapshot = snapshot_dir.join(format!("{rom_name}.png"));

    if snapshot.exists() {
        Some(snapshot)
    } else {
        None
    }
}

/// Setup the directory where one can save the Snapshots for tests.
///
/// A test with an associated snapshot will fail if it starts to differ from the established baseline.
pub fn setup_snapshot_directory(snapshot: &Path) -> anyhow::Result<()> {
    Ok(std::fs::create_dir_all(snapshot)?)
}
