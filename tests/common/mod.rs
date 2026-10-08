use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn template_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/templates")
        .join(relative)
}

pub fn cleanup(directory: tempfile::TempDir) {
    let path = directory.path().to_owned();
    directory
        .close()
        .unwrap_or_else(|error| panic!("Could not clean up {}: {error}", path.display()));
    assert!(!path.exists(), "Test directory remains: {}", path.display());
}

pub fn copy_directory(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap();
    if !source.exists() {
        return;
    }
    for entry in walkdir::WalkDir::new(source).min_depth(1) {
        let entry = entry.unwrap();
        let output = destination.join(entry.path().strip_prefix(source).unwrap());
        if entry.file_type().is_dir() {
            fs::create_dir_all(output).unwrap();
        } else {
            fs::copy(entry.path(), output).unwrap();
        }
    }
}

fn files(directory: &Path) -> Vec<PathBuf> {
    let mut files: Vec<_> = walkdir::WalkDir::new(directory)
        .into_iter()
        .map(|entry| entry.unwrap())
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.path().strip_prefix(directory).unwrap().to_owned())
        .collect();
    files.sort();
    files
}

pub fn assert_directory_eq(actual: &Path, expected: &Path) {
    assert_eq!(
        files(actual),
        files(expected),
        "File list differs from {}",
        expected.display()
    );
    for path in files(expected) {
        assert_eq!(
            fs::read(actual.join(&path)).unwrap(),
            fs::read(expected.join(&path)).unwrap(),
            "Contents differ: {} (expected {})",
            actual.join(&path).display(),
            expected.join(&path).display()
        );
    }
}
