use std::{collections::HashSet, path::PathBuf};

use ratatui::widgets::TableState;

pub(crate) fn move_selection(state: &mut TableState, len: usize, delta: isize) {
    //len - 1 is the last index to acount for overflow
    if len == 0 {
        state.select(None);
        return;
    }

    let current = state.selected().unwrap_or(0).min(len - 1);

    let mut next = if delta > 0 {
        (current + 1).min(len - 1)
    } else {
        current.saturating_sub(1)
    };

    if current as isize + delta > len as isize - 1 {
        // first index
        next = 0;
    }

    if current as isize + delta < 0 {
        next = len - 1;
    }

    state.select(Some(next));
}

pub(crate) fn keep_selection_valid(state: &mut TableState, len: usize) {
    match (state.selected(), len) {
        (_, 0) => state.select(None),

        (Some(index), _) if index < len => {}

        (_, _) => state.select(Some(0)),
    }
}

pub(crate) fn selected_path<'a>(state: &TableState, entries: &'a [PathBuf]) -> Option<&'a PathBuf> {
    state.selected().and_then(|index| entries.get(index))
}

pub(crate) fn paths_for_action(
    state: &TableState,
    entries: &[PathBuf],
    marked: &HashSet<PathBuf>,
) -> Vec<PathBuf> {
    if marked.is_empty() {
        return selected_path(state, entries).cloned().into_iter().collect();
    }

    marked.iter().cloned().collect()
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    fn state_with(selection: Option<usize>) -> TableState {
        let mut state = TableState::default();
        state.select(selection);
        state
    }

    fn entries() -> Vec<PathBuf> {
        vec![
            PathBuf::from("/a"),
            PathBuf::from("/b"),
            PathBuf::from("/c"),
        ]
    }

    #[test]
    fn move_selection_down_advances() {
        let mut state = state_with(Some(0));

        move_selection(&mut state, 3, 1);

        assert_eq!(state.selected(), Some(1));
    }

    #[test]
    fn move_selection_down_wraps_from_last_entry_to_first() {
        let mut state = state_with(Some(2));

        move_selection(&mut state, 3, 1);

        assert_eq!(state.selected(), Some(0));
    }

    #[test]
    fn move_selection_up_moves_backwards() {
        let mut state = state_with(Some(2));

        move_selection(&mut state, 3, -1);

        assert_eq!(state.selected(), Some(1));
    }

    #[test]
    fn move_selection_up_wraps_from_first_entry_to_last() {
        let mut state = state_with(Some(0));

        move_selection(&mut state, 3, -1);

        assert_eq!(state.selected(), Some(2));
    }

    #[test]
    fn move_selection_clears_selection_when_empty() {
        let mut state = state_with(Some(0));

        move_selection(&mut state, 0, 1);

        assert_eq!(state.selected(), None);
    }

    #[test]
    fn move_selection_down_without_selection_starts_at_second_entry() {
        let mut state = state_with(None);

        move_selection(&mut state, 3, 1);

        assert_eq!(state.selected(), Some(1));
    }

    #[test]
    fn move_selection_up_without_selection_wraps_to_last_entry() {
        let mut state = state_with(None);

        move_selection(&mut state, 3, -1);

        assert_eq!(state.selected(), Some(2));
    }

    #[test]
    fn move_selection_wraps_after_clamping_stale_selection() {
        let mut state = state_with(Some(10));

        move_selection(&mut state, 3, 1);

        assert_eq!(state.selected(), Some(0));
    }

    #[test]
    fn move_selection_down_wraps_around_single_entry() {
        let mut state = state_with(Some(0));

        move_selection(&mut state, 1, 1);

        assert_eq!(state.selected(), Some(0));
    }

    #[test]
    fn move_selection_up_wraps_around_single_entry() {
        let mut state = state_with(Some(0));

        move_selection(&mut state, 1, -1);

        assert_eq!(state.selected(), Some(0));
    }

    #[test]
    fn move_selection_down_cycles_through_all_entries() {
        let mut state = state_with(Some(0));

        move_selection(&mut state, 3, 1);
        assert_eq!(state.selected(), Some(1));

        move_selection(&mut state, 3, 1);
        assert_eq!(state.selected(), Some(2));

        move_selection(&mut state, 3, 1);
        assert_eq!(state.selected(), Some(0));
    }

    #[test]
    fn move_selection_up_cycles_through_all_entries() {
        let mut state = state_with(Some(0));

        move_selection(&mut state, 3, -1);
        assert_eq!(state.selected(), Some(2));

        move_selection(&mut state, 3, -1);
        assert_eq!(state.selected(), Some(1));

        move_selection(&mut state, 3, -1);
        assert_eq!(state.selected(), Some(0));
    }

    #[test]
    fn keep_selection_valid_clears_selection_when_empty() {
        let mut state = state_with(Some(0));

        keep_selection_valid(&mut state, 0);

        assert_eq!(state.selected(), None);
    }

    #[test]
    fn keep_selection_valid_keeps_in_bounds_selection() {
        let mut state = state_with(Some(1));

        keep_selection_valid(&mut state, 3);

        assert_eq!(state.selected(), Some(1));
    }

    #[test]
    fn keep_selection_valid_resets_out_of_bounds_selection() {
        let mut state = state_with(Some(5));

        keep_selection_valid(&mut state, 3);

        assert_eq!(state.selected(), Some(0));
    }

    #[test]
    fn keep_selection_valid_selects_first_when_none_with_entries() {
        let mut state = state_with(None);

        keep_selection_valid(&mut state, 3);

        assert_eq!(state.selected(), Some(0));
    }

    #[test]
    fn selected_path_returns_selected_entry() {
        let state = state_with(Some(1));
        let entries = entries();

        assert_eq!(selected_path(&state, &entries), Some(&PathBuf::from("/b")));
    }

    #[test]
    fn selected_path_returns_none_without_selection() {
        let state = state_with(None);

        assert_eq!(selected_path(&state, &entries()), None);
    }

    #[test]
    fn selected_path_returns_none_when_out_of_bounds() {
        let state = state_with(Some(5));

        assert_eq!(selected_path(&state, &entries()), None);
    }

    #[test]
    fn paths_for_action_uses_selection_when_nothing_marked() {
        let state = state_with(Some(1));

        let paths = paths_for_action(&state, &entries(), &HashSet::new());

        assert_eq!(paths, vec![PathBuf::from("/b")]);
    }

    #[test]
    fn paths_for_action_returns_empty_without_selection_or_marks() {
        let state = state_with(None);

        let paths = paths_for_action(&state, &entries(), &HashSet::new());

        assert!(paths.is_empty());
    }

    #[test]
    fn paths_for_action_returns_marked_entries() {
        let state = state_with(Some(0));
        let marked = HashSet::from([PathBuf::from("/b"), PathBuf::from("/c")]);

        let mut paths = paths_for_action(&state, &entries(), &marked);
        paths.sort();

        assert_eq!(paths, vec![PathBuf::from("/b"), PathBuf::from("/c")]);
    }

    #[test]
    fn paths_for_action_prefers_marks_over_selection() {
        let state = state_with(Some(0));
        let marked = HashSet::from([PathBuf::from("/c")]);

        let paths = paths_for_action(&state, &entries(), &marked);

        assert_eq!(paths, vec![PathBuf::from("/c")]);
    }
}
