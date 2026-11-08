use std::{
    collections::HashSet,
    env,
    path::{Path, PathBuf},
};

use color_eyre::eyre::Result;
use ratatui::widgets::TableState;

use crate::runtime::{
    entries::{filter_entries, get_entries},
    selection::{keep_selection_valid, paths_for_action, selected_path},
};

pub(crate) struct FileManager {
    pub(crate) current_dir: PathBuf,
    all_entries: Vec<PathBuf>,
    pub(crate) entries: Vec<PathBuf>,
    pub(crate) table_state: TableState,
    pub(crate) status: String,
    filter: String,
}

impl FileManager {
    pub(crate) fn new() -> Result<Self> {
        let current_dir = env::current_dir()?;
        let all_entries = get_entries(&current_dir)?;
        let entries = all_entries.clone();

        let mut table_state = TableState::default();

        if !entries.is_empty() {
            table_state.select(Some(0));
        }

        Ok(Self {
            current_dir,
            all_entries,
            entries,
            table_state,
            status: String::from(
                "j/k: move | Enter: open | Bksp: parent | Space: mark | d: delete | r: rename | n/N: new | /: filter | q: quit",
            ),
            filter: String::new(),
        })
    }

    // Getters

    pub(crate) fn current_dir(&self) -> &Path {
        &self.current_dir
    }

    pub(crate) fn entries(&self) -> &[PathBuf] {
        &self.entries
    }

    pub(crate) fn table_state(&self) -> &TableState {
        &self.table_state
    }

    pub(crate) fn table_state_mut(&mut self) -> &mut TableState {
        &mut self.table_state
    }

    pub(crate) fn filter(&self) -> &str {
        &self.filter
    }

    #[allow(dead_code)]
    pub(crate) fn status(&self) -> &str {
        &self.status
    }

    // Setters

    fn set_current_dir(&mut self, path: PathBuf) -> Result<()> {
        let entries = get_entries(&path)?;

        self.current_dir = path;
        self.all_entries = entries;
        self.filter.clear();
        self.apply_filter();

        self.status = format!("Directory: {}", self.current_dir.display());

        Ok(())
    }

    fn set_entries(&mut self, entries: Vec<PathBuf>) {
        self.all_entries = entries;
        self.apply_filter();
    }

    fn apply_filter(&mut self) {
        self.entries = filter_entries(&self.all_entries, &self.filter);

        keep_selection_valid(&mut self.table_state, self.entries.len());
    }

    pub(crate) fn set_filter(&mut self, filter: String) {
        self.filter = filter;
        self.apply_filter();
    }

    pub(crate) fn set_status(&mut self, status: String) {
        self.status = status;
    }

    // Directory operations

    pub(crate) fn open_folder(&mut self) -> Result<()> {
        if let Some(path) = selected_path(&self.table_state, &self.entries).cloned() {
            if path.is_dir() {
                self.set_current_dir(path)?;
            } else {
                // TODO: open files with the OS handler (xdg-open / open / start)
                self.status = format!("Not a directory: {}", path.display());
            }
        }

        Ok(())
    }

    pub(crate) fn parent_dir(&mut self) -> Result<()> {
        if let Some(parent) = self.current_dir.parent() {
            self.set_current_dir(parent.to_path_buf())?;
        }

        Ok(())
    }

    pub(crate) fn refresh(&mut self) -> Result<()> {
        let entries = get_entries(&self.current_dir)?;
        self.set_entries(entries);

        Ok(())
    }

    pub(crate) fn paths_for_action(&self, marked: &HashSet<PathBuf>) -> Vec<PathBuf> {
        paths_for_action(&self.table_state, &self.entries, marked)
    }
}

#[cfg(test)]
mod tests {
    use std::env;

    use super::*;
    use crate::runtime::test_support::TempDir;

    fn manager_for(dir: &Path) -> FileManager {
        let all_entries = get_entries(dir).unwrap();
        let entries = all_entries.clone();

        let mut table_state = TableState::default();

        if !entries.is_empty() {
            table_state.select(Some(0));
        }

        FileManager {
            current_dir: dir.to_path_buf(),
            all_entries,
            entries,
            table_state,
            status: String::new(),
            filter: String::new(),
        }
    }

    #[test]
    fn new_uses_current_directory() {
        let manager = FileManager::new().unwrap();

        assert_eq!(manager.current_dir(), env::current_dir().unwrap());
    }

    #[test]
    fn new_selects_first_entry_when_present() {
        // The repository root always has entries, so a fresh manager should
        // start with a valid selection.
        let manager = FileManager::new().unwrap();

        if !manager.entries().is_empty() {
            assert_eq!(manager.table_state().selected(), Some(0));
        }
    }

    #[test]
    fn open_folder_enters_selected_directory() {
        let dir = TempDir::new();
        let sub = dir.create_dir("sub");
        dir.write_file("sub/inner.txt", "");

        let mut manager = manager_for(dir.path());
        assert_eq!(manager.entries().to_vec(), vec![sub.clone()]);

        manager.open_folder().unwrap();

        assert_eq!(manager.current_dir(), sub.as_path());
        assert_eq!(manager.entries().len(), 1);
        assert_eq!(manager.status(), format!("Directory: {}", sub.display()));
    }

    #[test]
    fn open_folder_reports_when_selected_entry_is_a_file() {
        let dir = TempDir::new();
        let file = dir.write_file("a.txt", "");

        let mut manager = manager_for(dir.path());

        manager.open_folder().unwrap();

        assert_eq!(manager.current_dir(), dir.path());
        assert_eq!(
            manager.status(),
            format!("Not a directory: {}", file.display())
        );
    }

    #[test]
    fn open_folder_is_noop_without_selection() {
        let dir = TempDir::new();
        let mut manager = manager_for(dir.path());

        manager.open_folder().unwrap();

        assert_eq!(manager.current_dir(), dir.path());
        assert!(manager.status().is_empty());
    }

    #[test]
    fn parent_dir_moves_up_one_level() {
        let dir = TempDir::new();
        let child = dir.create_dir("child");

        let mut manager = manager_for(&child);

        manager.parent_dir().unwrap();

        assert_eq!(manager.current_dir(), dir.path());
        assert_eq!(
            manager.status(),
            format!("Directory: {}", dir.path().display())
        );
    }

    #[test]
    fn refresh_picks_up_new_entries() {
        let dir = TempDir::new();
        let mut manager = manager_for(dir.path());
        assert!(manager.entries().is_empty());

        dir.write_file("new.txt", "");

        manager.refresh().unwrap();

        assert_eq!(manager.entries().to_vec(), vec![dir.child("new.txt")]);
        assert_eq!(manager.table_state().selected(), Some(0));
    }

    #[test]
    fn refresh_keeps_valid_selection() {
        let dir = TempDir::new();
        dir.write_file("a.txt", "");
        dir.write_file("b.txt", "");

        let mut manager = manager_for(dir.path());
        manager.table_state_mut().select(Some(1));

        manager.refresh().unwrap();

        assert_eq!(manager.table_state().selected(), Some(1));
    }

    #[test]
    fn set_current_dir_clamps_selection_for_empty_directory() {
        let dir = TempDir::new();
        dir.write_file("a.txt", "");
        dir.write_file("b.txt", "");

        let mut manager = manager_for(dir.path());
        manager.table_state_mut().select(Some(1));

        let empty = dir.create_dir("empty");
        manager.set_current_dir(empty).unwrap();

        assert_eq!(manager.table_state().selected(), None);
    }

    #[test]
    fn set_entries_clamps_selection_to_first_entry() {
        let dir = TempDir::new();

        let mut manager = manager_for(dir.path());
        manager.table_state_mut().select(Some(5));

        manager.set_entries(vec![dir.child("a.txt")]);

        assert_eq!(manager.table_state().selected(), Some(0));
    }

    #[test]
    fn set_status_replaces_status() {
        let dir = TempDir::new();
        let mut manager = manager_for(dir.path());

        manager.set_status(String::from("hello"));

        assert_eq!(manager.status(), "hello");
    }

    #[test]
    fn paths_for_action_uses_selection_when_nothing_marked() {
        let dir = TempDir::new();
        dir.write_file("a.txt", "");
        dir.write_file("b.txt", "");

        let mut manager = manager_for(dir.path());
        manager.table_state_mut().select(Some(1));

        let paths = manager.paths_for_action(&HashSet::new());

        assert_eq!(paths, vec![manager.entries()[1].clone()]);
    }

    #[test]
    fn paths_for_action_returns_all_marked_entries() {
        let dir = TempDir::new();
        dir.write_file("a.txt", "");
        dir.write_file("b.txt", "");

        let manager = manager_for(dir.path());
        let marked: HashSet<PathBuf> = manager.entries().iter().cloned().collect();

        let mut paths = manager.paths_for_action(&marked);
        paths.sort();

        let mut expected = manager.entries().to_vec();
        expected.sort();

        assert_eq!(paths, expected);
    }

    #[test]
    fn set_filter_shows_only_matching_entries() {
        let dir = TempDir::new();
        dir.write_file("alpha.txt", "");
        dir.write_file("beta.txt", "");

        let mut manager = manager_for(dir.path());

        manager.set_filter(String::from("ALPHA"));

        assert_eq!(manager.filter(), "ALPHA");
        assert_eq!(manager.entries().to_vec(), vec![dir.child("alpha.txt")]);
    }

    #[test]
    fn set_filter_clears_selection_when_nothing_matches() {
        let dir = TempDir::new();
        dir.write_file("a.txt", "");

        let mut manager = manager_for(dir.path());

        manager.set_filter(String::from("zzz"));

        assert!(manager.entries().is_empty());
        assert_eq!(manager.table_state().selected(), None);
    }

    #[test]
    fn clearing_filter_restores_all_entries() {
        let dir = TempDir::new();
        dir.write_file("a.txt", "");
        dir.write_file("b.txt", "");

        let mut manager = manager_for(dir.path());
        manager.set_filter(String::from("a"));

        manager.set_filter(String::new());

        assert_eq!(manager.entries().len(), 2);
    }

    #[test]
    fn changing_directory_clears_filter() {
        let dir = TempDir::new();
        dir.write_file("a.txt", "");
        let child = dir.create_dir("child");

        let mut manager = manager_for(dir.path());
        manager.set_filter(String::from("a"));

        manager.set_current_dir(child).unwrap();

        assert_eq!(manager.filter(), "");
    }
}
