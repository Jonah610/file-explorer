use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
};

pub fn render(
    frame: &mut Frame,
    table_state: &mut TableState,
    current_dir: &Path,
    entries: &[PathBuf],
    marked: &HashSet<PathBuf>,
    status: &str,
) {
    // TODO: make colors/icons/theme configurable via the config file
    let folder = "\u{e5ff}";
    let file = "\u{f15b}";
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(2)])
        .split(frame.area());

    let rows = entries.iter().map(|path| {
        let is_dir = path.is_dir();
        let is_marked = marked.contains(path);

        let marker = if is_marked { "\u{f0132} " } else { "\u{e640} " };
        // TODO: make nerdfont optional (low prio)
        // TODO: ad file extension nerdfont instead of defaulting to file (low prio)
        // TODO: toggle file extention at the end of the file name (low prio)
        let kind = if is_dir { folder } else { file };
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("<unknown>");

        let style = if is_marked {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else if is_dir {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default()
        };

        Row::new(vec![
            Cell::from(marker.to_owned()),
            Cell::from(kind),
            Cell::from(name.to_owned()),
        ])
        .style(style)
    });

    let table = Table::new(
        rows,
        [
            Constraint::Length(5),
            Constraint::Length(5),
            Constraint::Min(1),
        ],
    )
    .header(
        Row::new(vec!["Mark", "Type", "Name"]).style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Green),
        ),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(Line::from(vec![
                Span::styled(
                    " File Explorer ",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(current_dir.display().to_string()),
            ])),
    )
    .row_highlight_style(
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol("> ");

    frame.render_stateful_widget(table, layout[0], table_state);

    let status_line = Line::from(Span::styled(status, Style::default().fg(Color::Gray)));

    frame.render_widget(
        ratatui::widgets::Paragraph::new(status_line).block(Block::default().borders(Borders::TOP)),
        layout[1],
    );
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::path::{Path, PathBuf};

    use ratatui::{Terminal, backend::TestBackend, widgets::TableState};

    use super::*;
    use crate::runtime::test_support::TempDir;

    fn draw(
        width: u16,
        height: u16,
        table_state: &mut TableState,
        current_dir: &Path,
        entries: &[PathBuf],
        marked: &HashSet<PathBuf>,
        status: &str,
    ) -> TestBackend {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| render(frame, table_state, current_dir, entries, marked, status))
            .unwrap();

        terminal.backend().clone()
    }

    fn screen(backend: &TestBackend) -> String {
        backend
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

    fn entries(names: &[&str]) -> Vec<PathBuf> {
        names.iter().map(PathBuf::from).collect()
    }

    #[test]
    fn renders_directory_title_and_headers() {
        let mut state = TableState::default();

        let backend = draw(
            60,
            10,
            &mut state,
            Path::new("/tmp/example"),
            &[],
            &HashSet::new(),
            "",
        );

        let screen = screen(&backend);

        assert!(screen.contains("File Explorer"), "screen was:\n{screen}");
        assert!(screen.contains("/tmp/example"), "screen was:\n{screen}");
        assert!(screen.contains("Mark"), "screen was:\n{screen}");
        assert!(screen.contains("Type"), "screen was:\n{screen}");
        assert!(screen.contains("Name"), "screen was:\n{screen}");
    }

    #[test]
    fn renders_entry_names() {
        let mut state = TableState::default();
        let entries = entries(&["/tmp/alpha.txt", "/tmp/beta.txt"]);

        let backend = draw(
            60,
            10,
            &mut state,
            Path::new("/tmp"),
            &entries,
            &HashSet::new(),
            "",
        );

        let screen = screen(&backend);

        assert!(screen.contains("alpha.txt"), "screen was:\n{screen}");
        assert!(screen.contains("beta.txt"), "screen was:\n{screen}");
    }

    #[test]
    fn renders_status_message() {
        let mut state = TableState::default();

        let backend = draw(
            60,
            10,
            &mut state,
            Path::new("/tmp"),
            &[],
            &HashSet::new(),
            "hello status",
        );

        assert!(screen(&backend).contains("hello status"));
    }

    #[test]
    fn renders_mark_glyphs_for_marked_and_unmarked_entries() {
        let mut state = TableState::default();
        let entries = entries(&["/tmp/a.txt", "/tmp/b.txt"]);
        let marked = HashSet::from([PathBuf::from("/tmp/a.txt")]);

        let backend = draw(60, 10, &mut state, Path::new("/tmp"), &entries, &marked, "");

        let screen = screen(&backend);

        assert!(screen.contains('\u{f0132}'), "screen was:\n{screen}");
        assert!(screen.contains('\u{e640}'), "screen was:\n{screen}");
    }

    #[test]
    fn renders_directory_and_file_icons() {
        let dir = TempDir::new();
        let sub = dir.create_dir("sub");
        let file = dir.write_file("file.txt", "");
        let entries = vec![sub, file];
        let mut state = TableState::default();

        let backend = draw(
            60,
            10,
            &mut state,
            dir.path(),
            &entries,
            &HashSet::new(),
            "",
        );

        let screen = screen(&backend);

        assert!(screen.contains('\u{e5ff}'), "screen was:\n{screen}");
        assert!(screen.contains('\u{f15b}'), "screen was:\n{screen}");
    }

    #[test]
    fn renders_with_selected_row() {
        let mut state = TableState::default();
        state.select(Some(1));
        let entries = entries(&["/tmp/a.txt", "/tmp/b.txt"]);

        let backend = draw(
            60,
            10,
            &mut state,
            Path::new("/tmp"),
            &entries,
            &HashSet::new(),
            "",
        );

        let screen = screen(&backend);

        assert!(screen.contains('>'), "screen was:\n{screen}");
        assert!(screen.contains("b.txt"), "screen was:\n{screen}");
    }
}
