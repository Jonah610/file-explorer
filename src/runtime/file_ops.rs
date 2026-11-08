use std::{
    fs::{self, OpenOptions},
    path::{Path, PathBuf},
};

use color_eyre::eyre::{Result, eyre};

pub(crate) fn validate_entry_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(eyre!("Name cannot be empty"));
    }

    if name == "." || name == ".." {
        return Err(eyre!("Name cannot be '.' or '..'"));
    }

    if name.contains('/') || name.contains('\\') {
        return Err(eyre!("Name cannot contain path separators"));
    }

    if name.contains('\0') {
        return Err(eyre!("Name cannot contain null bytes"));
    }

    Ok(())
}

pub(crate) fn delete_entries(paths: &[PathBuf]) -> Result<usize> {
    let mut deleted = 0;

    for path in paths {
        trash::delete(path)
            .map_err(|error| eyre!("Failed to move {} to trash: {error}", path.display()))?;

        deleted += 1;
    }

    Ok(deleted)
}

pub(crate) fn rename_entry(source: &Path, new_name: &str) -> Result<PathBuf> {
    validate_entry_name(new_name)?;

    let parent = source.parent().ok_or_else(|| {
        eyre!(
            "Cannot rename a path without a parent: {}",
            source.display()
        )
    })?;

    let destination = parent.join(new_name);

    if destination == source {
        return Ok(destination);
    }

    if destination.exists() {
        return Err(eyre!(
            "Destination already exists: {}",
            destination.display()
        ));
    }

    fs::rename(source, &destination)?;

    Ok(destination)
}

pub(crate) fn create_file(dir: &Path, name: &str) -> Result<PathBuf> {
    validate_entry_name(name)?;

    let path = dir.join(name);

    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?;

    Ok(path)
}

pub(crate) fn create_dir(dir: &Path, name: &str) -> Result<PathBuf> {
    validate_entry_name(name)?;

    let path = dir.join(name);

    fs::create_dir(&path)?;

    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::test_support::TempDir;

    #[test]
    fn validate_entry_name_accepts_regular_names() {
        assert!(validate_entry_name("notes.txt").is_ok());
        assert!(validate_entry_name("with space").is_ok());
        assert!(validate_entry_name(".hidden").is_ok());
    }

    #[test]
    fn validate_entry_name_rejects_empty_name() {
        let error = validate_entry_name("").unwrap_err();

        assert_eq!(error.to_string(), "Name cannot be empty");
    }

    #[test]
    fn validate_entry_name_rejects_dot_names() {
        assert!(validate_entry_name(".").is_err());
        assert!(validate_entry_name("..").is_err());
    }

    #[test]
    fn validate_entry_name_rejects_path_separators() {
        assert!(validate_entry_name("a/b").is_err());
        assert!(validate_entry_name("a\\b").is_err());
    }

    #[test]
    fn validate_entry_name_rejects_null_bytes() {
        assert!(validate_entry_name("a\0b").is_err());
    }

    #[test]
    fn rename_entry_renames_file() {
        let dir = TempDir::new();
        let source = dir.write_file("old.txt", "contents");

        let destination = rename_entry(&source, "new.txt").unwrap();

        assert_eq!(destination, dir.child("new.txt"));
        assert!(!source.exists());
        assert_eq!(fs::read_to_string(&destination).unwrap(), "contents");
    }

    #[test]
    fn rename_entry_renames_directory() {
        let dir = TempDir::new();
        let source = dir.create_dir("old");
        dir.write_file("old/inner.txt", "contents");

        let destination = rename_entry(&source, "new").unwrap();

        assert_eq!(destination, dir.child("new"));
        assert!(!source.exists());
        assert_eq!(
            fs::read_to_string(destination.join("inner.txt")).unwrap(),
            "contents"
        );
    }

    #[test]
    fn rename_entry_to_same_name_keeps_path() {
        let dir = TempDir::new();
        let source = dir.write_file("same.txt", "");

        let destination = rename_entry(&source, "same.txt").unwrap();

        assert_eq!(destination, source);
        assert!(source.exists());
    }

    #[test]
    fn rename_entry_rejects_existing_destination() {
        let dir = TempDir::new();
        let source = dir.write_file("a.txt", "");
        dir.write_file("b.txt", "");

        let error = rename_entry(&source, "b.txt").unwrap_err();

        assert!(
            error.to_string().contains("Destination already exists"),
            "unexpected error: {error}"
        );
        assert!(source.exists());
    }

    #[test]
    fn rename_entry_rejects_invalid_name() {
        let dir = TempDir::new();
        let source = dir.write_file("a.txt", "");

        let error = rename_entry(&source, "nested/b.txt").unwrap_err();

        assert_eq!(error.to_string(), "Name cannot contain path separators");
        assert!(source.exists());
    }

    #[test]
    fn create_file_creates_empty_file() {
        let dir = TempDir::new();

        let path = create_file(dir.path(), "new.txt").unwrap();

        assert_eq!(path, dir.child("new.txt"));
        assert!(path.is_file());
        assert_eq!(fs::read_to_string(&path).unwrap(), "");
    }

    #[test]
    fn create_file_rejects_existing_file() {
        let dir = TempDir::new();
        dir.write_file("existing.txt", "keep me");

        let result = create_file(dir.path(), "existing.txt");

        assert!(result.is_err());
        assert_eq!(
            fs::read_to_string(dir.child("existing.txt")).unwrap(),
            "keep me"
        );
    }

    #[test]
    fn create_file_rejects_invalid_name() {
        let dir = TempDir::new();

        let result = create_file(dir.path(), "nested/file.txt");

        assert!(result.is_err());
    }

    #[test]
    fn create_dir_creates_directory() {
        let dir = TempDir::new();

        let path = create_dir(dir.path(), "new-folder").unwrap();

        assert_eq!(path, dir.child("new-folder"));
        assert!(path.is_dir());
    }

    #[test]
    fn create_dir_rejects_existing_directory() {
        let dir = TempDir::new();
        dir.create_dir("existing");

        let result = create_dir(dir.path(), "existing");

        assert!(result.is_err());
    }

    #[test]
    fn create_dir_rejects_invalid_name() {
        let dir = TempDir::new();

        let result = create_dir(dir.path(), "..");

        assert!(result.is_err());
    }

    #[test]
    fn delete_entries_with_no_paths_deletes_nothing() {
        assert_eq!(delete_entries(&[]).unwrap(), 0);
    }

    #[test]
    fn delete_entries_errors_for_missing_path() {
        let dir = TempDir::new();

        let result = delete_entries(&[dir.child("missing.txt")]);

        assert!(result.is_err());
    }
}
