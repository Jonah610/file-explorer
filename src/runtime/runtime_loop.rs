use std::{collections::HashSet, path::PathBuf};

use color_eyre::eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::DefaultTerminal;

use crate::{
    rendering::render::render,
    runtime::{
        clipboard::{ClipboardMode, FileClipboard, paste_entries},
        file_manager::FileManager,
        file_ops::{create_dir, create_file, delete_entries, rename_entry},
        prompt::{Prompt, PromptAction, PromptKind, apply_key},
        selection::{move_selection, selected_path},
    },
};

pub fn runtime_loop(mut terminal: DefaultTerminal) -> Result<()> {
    let mut manager = FileManager::new()?;

    let mut marked: HashSet<PathBuf> = HashSet::new();
    let mut clipboard = FileClipboard::new();
    let mut prompt: Option<Prompt> = None;
    let mut pending_delete: Option<Vec<PathBuf>> = None;
    let mut pending_rename: Option<PathBuf> = None;

    loop {
        terminal.draw(|frame| {
            let FileManager {
                current_dir,
                entries,
                table_state,
                status,
                ..
            } = &mut manager;

            render(frame, table_state, current_dir, entries, &marked, status);
        })?;

        let event = event::read()?;

        let Event::Key(key) = event else {
            continue;
        };

        if !matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
            continue;
        }

        if let Some(paths) = pending_delete.take() {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => match delete_entries(&paths) {
                    Ok(count) => {
                        for path in &paths {
                            marked.remove(path);
                        }

                        match manager.refresh() {
                            Ok(()) => manager.set_status(format!("Deleted {count} item(s)")),
                            Err(error) => manager.set_status(format!(
                                "Deleted {count} item(s), but refresh failed: {error}"
                            )),
                        }
                    }

                    Err(error) => manager.set_status(format!("Delete failed: {error}")),
                },

                _ => manager.set_status(String::from("Delete cancelled")),
            }

            continue;
        }

        if prompt.is_some() {
            let action = {
                let active = prompt.as_mut().unwrap();
                let action = apply_key(active, key.code);

                if action == PromptAction::Continue {
                    if active.kind() == PromptKind::Filter {
                        manager.set_filter(active.input().to_string());
                    }

                    manager.set_status(active.status());
                }

                action
            };

            match action {
                PromptAction::Cancel => {
                    if let Some(active) = prompt.as_ref()
                        && active.kind() == PromptKind::Filter
                    {
                        manager.set_filter(String::new());
                    }

                    pending_rename = None;
                    manager.set_status(String::from("Cancelled"));
                    prompt = None;
                }

                PromptAction::Submit => {
                    let (kind, input) = {
                        let active = prompt.as_ref().unwrap();

                        (active.kind(), active.input().to_string())
                    };

                    prompt = None;
                    submit_prompt(kind, input, &mut manager, &mut marked, &mut pending_rename);
                }

                PromptAction::Continue => {}
            }

            continue;
        }

        match key.code {
            // TODO: when exiting program have the terminal in the last visited directory (mid prio)
            KeyCode::Char('q') | KeyCode::Esc => break,

            // Move down
            KeyCode::Down | KeyCode::Char('j') => {
                let len = manager.entries().len();

                move_selection(manager.table_state_mut(), len, 1);
            }

            // Move up
            KeyCode::Up | KeyCode::Char('k') => {
                let len = manager.entries().len();

                move_selection(manager.table_state_mut(), len, -1);
            }

            // Open directory
            KeyCode::Enter => {
                if let Err(error) = manager.open_folder() {
                    manager.set_status(format!("Error opening directory: {error}"));
                }
            }

            // Go to parent directory
            KeyCode::Backspace => {
                if let Err(error) = manager.parent_dir() {
                    manager.set_status(format!("Error opening parent: {error}"));
                }
            }

            // Mark selected file
            KeyCode::Char(' ') => {
                if let Some(path) = selected_path(manager.table_state(), manager.entries()).cloned()
                {
                    if !marked.remove(&path) {
                        marked.insert(path);
                    }

                    manager.set_status(format!("{} item(s) marked", marked.len()));
                }
            }

            // Mark all / clear marks
            KeyCode::Char('a') => {
                let all_marked = !manager.entries().is_empty()
                    && manager.entries().iter().all(|path| marked.contains(path));

                if all_marked {
                    for path in manager.entries() {
                        marked.remove(path);
                    }

                    manager.set_status(String::from("Cleared marks in this directory"));
                } else {
                    marked.extend(manager.entries().iter().cloned());

                    manager.set_status(format!("{} item(s) marked", marked.len()));
                }
            }

            // Copy
            KeyCode::Char('c') => {
                clipboard.paths = manager.paths_for_action(&marked);

                if clipboard.paths.is_empty() {
                    manager.set_status(String::from("Nothing selected to copy"));
                } else {
                    clipboard.mode = Some(ClipboardMode::Copy);

                    manager.set_status(format!(
                        "Copied {} item(s) to clipboard",
                        clipboard.paths.len()
                    ));
                }
            }

            // Cut
            KeyCode::Char('x') => {
                clipboard.paths = manager.paths_for_action(&marked);

                if clipboard.paths.is_empty() {
                    manager.set_status(String::from("Nothing selected to cut"));
                } else {
                    clipboard.mode = Some(ClipboardMode::Cut);

                    manager.set_status(format!(
                        "Cut {} item(s); navigate and press v to move",
                        clipboard.paths.len()
                    ));
                }
            }

            // Paste
            KeyCode::Char('v') => match paste_entries(&clipboard, manager.current_dir()) {
                Ok(count) => {
                    if clipboard.mode == Some(ClipboardMode::Cut) {
                        clipboard.paths.clear();
                        clipboard.mode = None;
                    }

                    match manager.refresh() {
                        Ok(()) => {
                            manager.set_status(format!("Pasted {} item(s)", count));
                        }
                        Err(error) => {
                            manager.set_status(format!(
                                "Paste succeeded, but refresh failed: {error}"
                            ));
                        }
                    }
                }

                Err(error) => {
                    manager.set_status(format!("Paste failed: {error}"));
                }
            },

            // Delete (with confirmation)
            KeyCode::Char('d') | KeyCode::Delete => {
                let paths = manager.paths_for_action(&marked);

                if paths.is_empty() {
                    manager.set_status(String::from("Nothing selected to delete"));
                } else {
                    manager.set_status(format!(
                        "Delete {} item(s)? Press y to confirm, any other key to cancel",
                        paths.len()
                    ));

                    pending_delete = Some(paths);
                }
            }

            // Rename
            KeyCode::Char('r') => {
                if let Some(path) = selected_path(manager.table_state(), manager.entries()).cloned()
                {
                    let name = path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("")
                        .to_string();

                    let active = Prompt::prefilled(PromptKind::Rename, name);

                    manager.set_status(active.status());
                    pending_rename = Some(path);
                    prompt = Some(active);
                } else {
                    manager.set_status(String::from("Nothing selected to rename"));
                }
            }

            // Create file
            KeyCode::Char('n') => {
                let active = Prompt::new(PromptKind::CreateFile);

                manager.set_status(active.status());
                prompt = Some(active);
            }

            // Create directory
            KeyCode::Char('N') => {
                let active = Prompt::new(PromptKind::CreateDir);

                manager.set_status(active.status());
                prompt = Some(active);
            }

            // Search / filter
            KeyCode::Char('/') => {
                let active = Prompt::prefilled(PromptKind::Filter, manager.filter().to_string());

                manager.set_status(active.status());
                prompt = Some(active);
            }

            // TODO: implement undo for the last file operation
            // TODO: make keybindings configurable via the config file
            _ => {}
        }
    }

    Ok(())
}

fn submit_prompt(
    kind: PromptKind,
    input: String,
    manager: &mut FileManager,
    marked: &mut HashSet<PathBuf>,
    pending_rename: &mut Option<PathBuf>,
) {
    match kind {
        PromptKind::Filter => {
            manager.set_filter(input.clone());

            if input.is_empty() {
                manager.set_status(String::from("Filter cleared"));
            } else {
                manager.set_status(format!("Filter: {input}"));
            }
        }

        PromptKind::Rename => {
            let Some(source) = pending_rename.take() else {
                manager.set_status(String::from("Nothing selected to rename"));
                return;
            };

            match rename_entry(&source, &input) {
                Ok(destination) => {
                    if marked.remove(&source) {
                        marked.insert(destination.clone());
                    }

                    match manager.refresh() {
                        Ok(()) => {
                            manager.set_status(format!("Renamed to {}", destination.display()));
                        }
                        Err(error) => {
                            manager.set_status(format!(
                                "Rename succeeded, but refresh failed: {error}"
                            ));
                        }
                    }
                }

                Err(error) => manager.set_status(format!("Rename failed: {error}")),
            }
        }

        PromptKind::CreateFile => match create_file(manager.current_dir(), &input) {
            Ok(path) => finish_create(manager, path),
            Err(error) => manager.set_status(format!("Create failed: {error}")),
        },

        PromptKind::CreateDir => match create_dir(manager.current_dir(), &input) {
            Ok(path) => finish_create(manager, path),
            Err(error) => manager.set_status(format!("Create failed: {error}")),
        },
    }
}

fn finish_create(manager: &mut FileManager, path: PathBuf) {
    match manager.refresh() {
        Ok(()) => manager.set_status(format!("Created {}", path.display())),
        Err(error) => manager.set_status(format!(
            "Created {} but refresh failed: {error}",
            path.display()
        )),
    }
}
