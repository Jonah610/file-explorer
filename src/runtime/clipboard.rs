use std::{
    fs,
    path::{Path, PathBuf},
};

use color_eyre::eyre::{Result, eyre};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClipboardMode {
    Copy,
    Cut,
}

pub(crate) struct FileClipboard {
    pub(crate) paths: Vec<PathBuf>,
    pub(crate) mode: Option<ClipboardMode>,
}

impl FileClipboard {
    pub(crate) fn new() -> Self {
        Self {
            paths: Vec::new(),
            mode: None,
        }
    }
}

pub(crate) fn paste_entries(clipboard: &FileClipboard, destination_dir: &Path) -> Result<usize> {
    let Some(mode) = clipboard.mode else {
        return Err(eyre!("Clipboard is empty"));
    };

    if clipboard.paths.is_empty() {
        return Err(eyre!("Clipboard is empty"));
    }

    // Validate destinations first
    for source in &clipboard.paths {
        if !source.exists() {
            return Err(eyre!("Source no longer exists: {}", source.display()));
        }

        let destination = destination_dir.join(
            source
                .file_name()
                .ok_or_else(|| eyre!("Invalid source path: {}", source.display()))?,
        );

        if source == &destination {
            return Err(eyre!("Source and destination are the same"));
        }

        if source.is_dir() && destination.starts_with(source) {
            return Err(eyre!(
                "Cannot paste a directory into itself: {}",
                source.display()
            ));
        }

        if destination.exists() {
            return Err(eyre!(
                "Destination already exists: {}",
                destination.display()
            ));
        }
    }

    // Perform operations
    for source in &clipboard.paths {
        let destination = destination_dir.join(
            source
                .file_name()
                .ok_or_else(|| eyre!("Invalid source path: {}", source.display()))?,
        );

        match mode {
            ClipboardMode::Copy => {
                copy_entry(source, &destination)?;
            }

            ClipboardMode::Cut => {
                fs::rename(source, &destination)?;
            }
        }
    }

    Ok(clipboard.paths.len())
}

fn copy_entry(source: &Path, destination: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(source)?;

    if metadata.file_type().is_symlink() {
        return Err(eyre!(
            "Copying symbolic links is not supported: {}",
            source.display()
        ));
    }

    if metadata.is_dir() {
        fs::create_dir(destination)?;

        for entry in fs::read_dir(source)? {
            let entry = entry?;

            let child_source = entry.path();
            let child_destination = destination.join(entry.file_name());

            copy_entry(&child_source, &child_destination)?;
        }
    } else if metadata.is_file() {
        fs::copy(source, destination)?;
    } else {
        return Err(eyre!("Unsupported file type: {}", source.display()));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::test_support::TempDir;

    fn clipboard(paths: Vec<PathBuf>, mode: ClipboardMode) -> FileClipboard {
        FileClipboard {
            paths,
            mode: Some(mode),
        }
    }

    #[test]
    fn new_clipboard_is_empty() {
        let clipboard = FileClipboard::new();

        assert!(clipboard.paths.is_empty());
        assert!(clipboard.mode.is_none());
    }

    #[test]
    fn paste_entries_errors_when_clipboard_has_no_mode() {
        let dir = TempDir::new();

        let error = paste_entries(&FileClipboard::new(), dir.path()).unwrap_err();

        assert_eq!(error.to_string(), "Clipboard is empty");
    }

    #[test]
    fn paste_entries_errors_when_clipboard_has_no_paths() {
        let dir = TempDir::new();
        let clipboard = clipboard(Vec::new(), ClipboardMode::Copy);

        let error = paste_entries(&clipboard, dir.path()).unwrap_err();

        assert_eq!(error.to_string(), "Clipboard is empty");
    }

    #[test]
    fn paste_entries_errors_when_source_is_missing() {
        let dir = TempDir::new();
        let clipboard = clipboard(vec![dir.child("missing.txt")], ClipboardMode::Copy);

        let error = paste_entries(&clipboard, dir.path()).unwrap_err();

        assert!(
            error.to_string().contains("Source no longer exists"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn paste_entries_errors_when_destination_already_exists() {
        let dir = TempDir::new();
        let source = dir.write_file("src/file.txt", "new");
        let destination_dir = dir.create_dir("dst");
        dir.write_file("dst/file.txt", "old");

        let clipboard = clipboard(vec![source], ClipboardMode::Copy);

        let error = paste_entries(&clipboard, &destination_dir).unwrap_err();

        assert!(
            error.to_string().contains("Destination already exists"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn paste_entries_errors_when_pasting_into_same_directory() {
        let dir = TempDir::new();
        let source = dir.write_file("file.txt", "contents");

        let clipboard = clipboard(vec![source], ClipboardMode::Copy);

        let error = paste_entries(&clipboard, dir.path()).unwrap_err();

        assert_eq!(error.to_string(), "Source and destination are the same");
    }

    #[test]
    fn paste_entries_errors_when_pasting_directory_into_itself() {
        let dir = TempDir::new();
        let source = dir.create_dir("folder");

        let clipboard = clipboard(vec![source.clone()], ClipboardMode::Copy);

        let error = paste_entries(&clipboard, &source).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("Cannot paste a directory into itself"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn paste_entries_copies_file() {
        let dir = TempDir::new();
        let source = dir.write_file("src/file.txt", "hello");
        let destination_dir = dir.create_dir("dst");

        let clipboard = clipboard(vec![source.clone()], ClipboardMode::Copy);

        let count = paste_entries(&clipboard, &destination_dir).unwrap();

        assert_eq!(count, 1);
        assert_eq!(
            fs::read_to_string(destination_dir.join("file.txt")).unwrap(),
            "hello"
        );
        assert!(source.exists(), "copy should leave the source in place");
    }

    #[test]
    fn paste_entries_copies_directories_recursively() {
        let dir = TempDir::new();
        let source = dir.create_dir("src");
        dir.write_file("src/a.txt", "a");
        dir.write_file("src/nested/b.txt", "b");
        let destination_dir = dir.create_dir("dst");

        let clipboard = clipboard(vec![source], ClipboardMode::Copy);

        let count = paste_entries(&clipboard, &destination_dir).unwrap();

        assert_eq!(count, 1);
        assert_eq!(
            fs::read_to_string(destination_dir.join("src/a.txt")).unwrap(),
            "a"
        );
        assert_eq!(
            fs::read_to_string(destination_dir.join("src/nested/b.txt")).unwrap(),
            "b"
        );
    }

    #[test]
    fn paste_entries_copies_multiple_sources() {
        let dir = TempDir::new();
        let first = dir.write_file("first.txt", "1");
        let second = dir.write_file("second.txt", "2");
        let destination_dir = dir.create_dir("dst");

        let clipboard = clipboard(vec![first, second], ClipboardMode::Copy);

        let count = paste_entries(&clipboard, &destination_dir).unwrap();

        assert_eq!(count, 2);
        assert_eq!(
            fs::read_to_string(destination_dir.join("first.txt")).unwrap(),
            "1"
        );
        assert_eq!(
            fs::read_to_string(destination_dir.join("second.txt")).unwrap(),
            "2"
        );
    }

    #[test]
    fn paste_entries_cut_moves_file() {
        let dir = TempDir::new();
        let source = dir.write_file("src/file.txt", "move me");
        let destination_dir = dir.create_dir("dst");

        let clipboard = clipboard(vec![source.clone()], ClipboardMode::Cut);

        let count = paste_entries(&clipboard, &destination_dir).unwrap();

        assert_eq!(count, 1);
        assert!(!source.exists(), "cut should remove the source");
        assert_eq!(
            fs::read_to_string(destination_dir.join("file.txt")).unwrap(),
            "move me"
        );
    }

    #[test]
    fn paste_entries_cut_moves_directory() {
        let dir = TempDir::new();
        let source = dir.create_dir("src");
        dir.write_file("src/a.txt", "a");
        let destination_dir = dir.create_dir("dst");

        let clipboard = clipboard(vec![source.clone()], ClipboardMode::Cut);

        paste_entries(&clipboard, &destination_dir).unwrap();

        assert!(!source.exists());
        assert_eq!(
            fs::read_to_string(destination_dir.join("src/a.txt")).unwrap(),
            "a"
        );
    }

    #[test]
    fn copy_entry_rejects_unsupported_file_types() {
        // /dev/null is a character device, so it is neither a regular file
        // nor a directory and copy_entry should refuse it.
        #[cfg(unix)]
        {
            let dir = TempDir::new();
            let source = PathBuf::from("/dev/null");

            let error = copy_entry(&source, &dir.child("null-copy")).unwrap_err();

            assert!(
                error.to_string().contains("Unsupported file type"),
                "unexpected error: {error}"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn paste_entries_rejects_symbolic_links() {
        use std::os::unix::fs::symlink;

        let dir = TempDir::new();
        let target = dir.write_file("target.txt", "target");
        let link = dir.child("link.txt");
        symlink(&target, &link).unwrap();
        let destination_dir = dir.create_dir("dst");

        let clipboard = clipboard(vec![link], ClipboardMode::Copy);

        let error = paste_entries(&clipboard, &destination_dir).unwrap_err();

        assert!(
            error.to_string().contains("symbolic links"),
            "unexpected error: {error}"
        );
    }
}
