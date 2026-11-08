use crossterm::event::KeyCode;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum PromptKind {
    Rename,
    CreateFile,
    CreateDir,
    Filter,
}

impl PromptKind {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Rename => "Rename",
            Self::CreateFile => "New file",
            Self::CreateDir => "New folder",
            Self::Filter => "Filter",
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct Prompt {
    kind: PromptKind,
    input: String,
}

impl Prompt {
    pub(crate) fn new(kind: PromptKind) -> Self {
        Self {
            kind,
            input: String::new(),
        }
    }

    pub(crate) fn prefilled(kind: PromptKind, input: String) -> Self {
        Self { kind, input }
    }

    pub(crate) fn kind(&self) -> PromptKind {
        self.kind
    }

    pub(crate) fn input(&self) -> &str {
        &self.input
    }

    pub(crate) fn push(&mut self, character: char) {
        self.input.push(character);
    }

    pub(crate) fn backspace(&mut self) {
        self.input.pop();
    }

    pub(crate) fn status(&self) -> String {
        format!("{}: {}", self.kind.label(), self.input)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum PromptAction {
    Continue,
    Cancel,
    Submit,
}

pub(crate) fn apply_key(prompt: &mut Prompt, key: KeyCode) -> PromptAction {
    match key {
        KeyCode::Esc => PromptAction::Cancel,

        KeyCode::Enter => PromptAction::Submit,

        KeyCode::Backspace => {
            prompt.backspace();
            PromptAction::Continue
        }

        KeyCode::Char(character) => {
            prompt.push(character);
            PromptAction::Continue
        }

        _ => PromptAction::Continue,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_prompt_starts_empty() {
        let prompt = Prompt::new(PromptKind::Rename);

        assert_eq!(prompt.kind(), PromptKind::Rename);
        assert_eq!(prompt.input(), "");
    }

    #[test]
    fn prefilled_prompt_keeps_input() {
        let prompt = Prompt::prefilled(PromptKind::Filter, String::from("txt"));

        assert_eq!(prompt.kind(), PromptKind::Filter);
        assert_eq!(prompt.input(), "txt");
    }

    #[test]
    fn push_appends_characters() {
        let mut prompt = Prompt::new(PromptKind::CreateFile);

        prompt.push('a');
        prompt.push('b');

        assert_eq!(prompt.input(), "ab");
    }

    #[test]
    fn backspace_removes_last_character() {
        let mut prompt = Prompt::prefilled(PromptKind::Rename, String::from("abc"));

        prompt.backspace();

        assert_eq!(prompt.input(), "ab");
    }

    #[test]
    fn backspace_on_empty_input_is_noop() {
        let mut prompt = Prompt::new(PromptKind::Rename);

        prompt.backspace();

        assert_eq!(prompt.input(), "");
    }

    #[test]
    fn status_uses_kind_label_and_input() {
        let prompt = Prompt::prefilled(PromptKind::CreateDir, String::from("docs"));

        assert_eq!(prompt.status(), "New folder: docs");
    }

    #[test]
    fn status_labels_each_kind() {
        assert_eq!(Prompt::new(PromptKind::Rename).status(), "Rename: ");
        assert_eq!(Prompt::new(PromptKind::CreateFile).status(), "New file: ");
        assert_eq!(Prompt::new(PromptKind::CreateDir).status(), "New folder: ");
        assert_eq!(Prompt::new(PromptKind::Filter).status(), "Filter: ");
    }

    #[test]
    fn apply_key_pushes_characters() {
        let mut prompt = Prompt::new(PromptKind::Rename);

        let action = apply_key(&mut prompt, KeyCode::Char('x'));

        assert_eq!(action, PromptAction::Continue);
        assert_eq!(prompt.input(), "x");
    }

    #[test]
    fn apply_key_backspace_removes_character() {
        let mut prompt = Prompt::prefilled(PromptKind::Rename, String::from("xy"));

        let action = apply_key(&mut prompt, KeyCode::Backspace);

        assert_eq!(action, PromptAction::Continue);
        assert_eq!(prompt.input(), "x");
    }

    #[test]
    fn apply_key_enter_submits() {
        let mut prompt = Prompt::new(PromptKind::Rename);

        assert_eq!(apply_key(&mut prompt, KeyCode::Enter), PromptAction::Submit);
    }

    #[test]
    fn apply_key_escape_cancels() {
        let mut prompt = Prompt::new(PromptKind::Rename);

        assert_eq!(apply_key(&mut prompt, KeyCode::Esc), PromptAction::Cancel);
    }

    #[test]
    fn apply_key_ignores_other_keys() {
        let mut prompt = Prompt::prefilled(PromptKind::Rename, String::from("abc"));

        let action = apply_key(&mut prompt, KeyCode::Left);

        assert_eq!(action, PromptAction::Continue);
        assert_eq!(prompt.input(), "abc");
    }
}
