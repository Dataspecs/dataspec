use std::path::{Path, PathBuf};

pub fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/data-specs")
}

pub fn fixture(name: impl AsRef<Path>) -> PathBuf {
    fixture_dir().join(name)
}
