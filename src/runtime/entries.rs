use std::{
    fs,
    path::{Path, PathBuf},
};

use color_eyre::eyre::Result;

pub(crate) fn get_entries(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut entries = fs::read_dir(dir)?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .collect::<Vec<_>>();

    entries.sort_by_key(|path| {
        let is_file = !path.is_dir();

        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("")
            .to_lowercase();

        (is_file, name)
    });

    Ok(entries)
}

pub(crate) fn filter_entries(entries: &[PathBuf], filter: &str) -> Vec<PathBuf> {
    if filter.is_empty() {
        return entries.to_vec();
    }

    let needle = filter.to_lowercase();

    entries
        .iter()
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.to_lowercase().contains(&needle))
                .unwrap_or(false)
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::test_support::TempDir;

    fn file_names(entries: &[PathBuf]) -> Vec<String> {
        entries
            .iter()
            .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn get_entries_lists_directories_first_then_files_sorted_case_insensitively() {
        let dir = TempDir::new();

        dir.create_dir("b_dir");
        dir.create_dir("a_dir");
        dir.write_file("Zeta.txt", "");
        dir.write_file("alpha.txt", "");
        dir.write_file("Beta.txt", "");

        let entries = get_entries(dir.path()).unwrap();

        assert_eq!(
            file_names(&entries),
            vec!["a_dir", "b_dir", "alpha.txt", "Beta.txt", "Zeta.txt"]
        );
    }

    #[test]
    fn get_entries_returns_empty_for_empty_directory() {
        let dir = TempDir::new();

        let entries = get_entries(dir.path()).unwrap();

        assert!(entries.is_empty());
    }

    #[test]
    fn get_entries_returns_full_paths() {
        let dir = TempDir::new();
        dir.write_file("a.txt", "");

        let entries = get_entries(dir.path()).unwrap();

        assert_eq!(entries, vec![dir.child("a.txt")]);
    }

    #[test]
    fn get_entries_errors_for_missing_directory() {
        let dir = TempDir::new();

        let result = get_entries(&dir.child("does-not-exist"));

        assert!(result.is_err());
    }

    #[test]
    fn filter_entries_returns_all_for_empty_filter() {
        let entries = vec![PathBuf::from("/a.txt"), PathBuf::from("/b.txt")];

        let filtered = filter_entries(&entries, "");

        assert_eq!(filtered, entries);
    }

    #[test]
    fn filter_entries_matches_names_case_insensitively() {
        let entries = vec![
            PathBuf::from("/Alpha.txt"),
            PathBuf::from("/beta.txt"),
            PathBuf::from("/alphabet.md"),
        ];

        let filtered = filter_entries(&entries, "ALPHA");

        assert_eq!(
            filtered,
            vec![PathBuf::from("/Alpha.txt"), PathBuf::from("/alphabet.md")]
        );
    }

    #[test]
    fn filter_entries_returns_empty_when_nothing_matches() {
        let entries = vec![PathBuf::from("/a.txt")];

        let filtered = filter_entries(&entries, "zzz");

        assert!(filtered.is_empty());
    }

    #[test]
    fn filter_entries_matches_directory_names() {
        let entries = vec![PathBuf::from("/documents"), PathBuf::from("/a.txt")];

        let filtered = filter_entries(&entries, "doc");

        assert_eq!(filtered, vec![PathBuf::from("/documents")]);
    }
}
