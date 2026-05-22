/// Local text-input target tracked after Wavecrate focus actions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum RetainedTextInputTarget {
    /// No retained text input owns keyboard editing.
    #[default]
    None,
    /// Browser search owns text editing.
    BrowserSearch,
    /// Folder search owns text editing.
    FolderSearch,
    /// Browser metadata tag sidebar input owns text editing.
    BrowserPillEditor,
    /// Inline folder create/rename input owns text editing.
    FolderCreate,
    /// Visible confirmation prompt input owns text editing.
    Prompt,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct RetainedTextEditState {
    pub(super) target: RetainedTextInputTarget,
    pub(super) value: String,
    pub(super) caret: usize,
    pub(super) selection_anchor: Option<usize>,
}

impl RetainedTextEditState {
    pub(super) fn selection(&self) -> Option<(usize, usize)> {
        let anchor = self.selection_anchor?;
        (anchor != self.caret).then_some((anchor, self.caret))
    }

    pub(super) fn has_selection(&self) -> bool {
        self.selection().is_some()
    }

    pub(super) fn clear_selection(&mut self) {
        self.selection_anchor = None;
    }

    pub(super) fn sync(&mut self, target: RetainedTextInputTarget, value: String) {
        if self.target != target || self.value != value {
            self.target = target;
            self.value = value;
            self.caret = self.value.chars().count();
            self.selection_anchor = None;
        }
    }

    fn selected_bounds(&self) -> Option<(usize, usize)> {
        self.selection()
            .map(|(a, b)| if a < b { (a, b) } else { (b, a) })
    }

    pub(super) fn replace_selection(&mut self, replacement: &str) -> bool {
        let Some((start, end)) = self.selected_bounds() else {
            return false;
        };
        replace_char_range(&mut self.value, start, end, replacement);
        self.caret = start + replacement.chars().count();
        self.clear_selection();
        true
    }

    pub(super) fn insert_char(&mut self, character: char) {
        self.replace_selection("");
        let index = byte_index_for_char(&self.value, self.caret);
        self.value.insert(index, character);
        self.caret += 1;
    }

    pub(super) fn backspace(&mut self) -> bool {
        if self.replace_selection("") {
            return true;
        }
        if self.caret == 0 {
            return false;
        }
        let end = self.caret;
        let start = self.caret - 1;
        replace_char_range(&mut self.value, start, end, "");
        self.caret = start;
        true
    }

    pub(super) fn delete(&mut self) -> bool {
        if self.replace_selection("") {
            return true;
        }
        if self.caret >= self.value.chars().count() {
            return false;
        }
        replace_char_range(&mut self.value, self.caret, self.caret + 1, "");
        true
    }

    pub(super) fn move_word_left(&mut self, selecting: bool) {
        self.move_caret(previous_word_boundary(&self.value, self.caret), selecting);
    }

    pub(super) fn move_word_right(&mut self, selecting: bool) {
        self.move_caret(next_word_boundary(&self.value, self.caret), selecting);
    }

    pub(super) fn delete_word_left(&mut self) -> bool {
        if self.replace_selection("") {
            return true;
        }
        let start = previous_word_boundary(&self.value, self.caret);
        if start == self.caret {
            return false;
        }
        replace_char_range(&mut self.value, start, self.caret, "");
        self.caret = start;
        true
    }

    pub(super) fn delete_word_right(&mut self) -> bool {
        if self.replace_selection("") {
            return true;
        }
        let end = next_word_boundary(&self.value, self.caret);
        if end == self.caret {
            return false;
        }
        replace_char_range(&mut self.value, self.caret, end, "");
        true
    }

    pub(super) fn move_caret(&mut self, caret: usize, selecting: bool) {
        let caret = caret.min(self.value.chars().count());
        if selecting {
            if self.selection_anchor.is_none() {
                self.selection_anchor = Some(self.caret);
            }
        } else {
            self.clear_selection();
        }
        self.caret = caret;
    }
}

fn byte_index_for_char(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map(|(index, _)| index)
        .unwrap_or(text.len())
}

fn previous_word_boundary(text: &str, caret: usize) -> usize {
    let chars = text.chars().collect::<Vec<_>>();
    let mut index = caret.min(chars.len());
    while index > 0 && !is_word_char(chars[index - 1]) {
        index -= 1;
    }
    while index > 0 && is_word_char(chars[index - 1]) {
        index -= 1;
    }
    index
}

fn next_word_boundary(text: &str, caret: usize) -> usize {
    let chars = text.chars().collect::<Vec<_>>();
    let mut index = caret.min(chars.len());
    while index < chars.len() && !is_word_char(chars[index]) {
        index += 1;
    }
    while index < chars.len() && is_word_char(chars[index]) {
        index += 1;
    }
    index
}

fn is_word_char(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

fn replace_char_range(text: &mut String, start: usize, end: usize, replacement: &str) {
    let start = byte_index_for_char(text, start);
    let end = byte_index_for_char(text, end);
    text.replace_range(start..end, replacement);
}
