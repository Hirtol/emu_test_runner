use std::borrow::Cow;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct TestCandidate {
    pub rom_id: String,
    pub rom_path: PathBuf,
    pub is_sequence_test: bool,
}

impl TestCandidate {
    /// Create a new test candidate.
    ///
    /// # Arguments
    /// * `id` should be unique, and the path should point to a ROM that can be loaded by the emulator under test.
    /// * `is_sequence_test` should be `true` if this test case will produce multiple snapshots. These snapshots will be
    /// compared individually (e.g, can have a mix of expected/non-expected snapshots).
    pub fn new(id: impl Into<String>, path: impl Into<PathBuf>, is_sequence_test: bool) -> TestCandidate {
        Self {
            rom_id: id.into(),
            rom_path: path.into(),
            is_sequence_test,
        }
    }

    /// Find all possible test candidates in a directory and all its sub-directories based on a given file extension.
    ///
    /// Note that this will assume that there are *no* sequence tests.
    pub fn find_all_in_directory(
        path: impl AsRef<Path>,
        extension: impl AsRef<str>,
    ) -> anyhow::Result<Vec<TestCandidate>> {
        let files = list_files_with_extensions(path.as_ref(), extension.as_ref())?;

        Ok(files
            .into_iter()
            .map(|path| TestCandidate::new(get_rom_fs_id(&path).into_owned(), path, false))
            .collect())
    }
}

/// Lists all files in the provided `path` (if the former is a directory) with the provided
/// `extension`. Will traverse all sub-directories in search of this extension
pub fn list_files_with_extensions(path: impl AsRef<Path>, extension: impl AsRef<str>) -> anyhow::Result<Vec<PathBuf>> {
    let mut result = Vec::with_capacity(40);

    if path.as_ref().is_dir() {
        for entry in std::fs::read_dir(path)? {
            let path = entry?.path();
            if path.is_dir() {
                result.extend(list_files_with_extensions(&path, extension.as_ref())?);
            } else if path.to_str().filter(|t| t.ends_with(extension.as_ref())).is_some() {
                result.push(path);
            }
        }
    }

    Ok(result)
}

pub fn get_rom_fs_id(path: &Path) -> Cow<'_, str> {
    path.file_stem().expect("Failed to get rom stem").to_string_lossy()
}
