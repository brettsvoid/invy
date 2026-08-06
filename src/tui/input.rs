//! Single-line text input used by the TUI prompts.

/// A single-line editable buffer with a character-indexed cursor.
#[derive(Debug, Default, Clone)]
pub struct TextInput {
    value: String,
    cursor: usize,
}

impl TextInput {
    /// Create an input holding `value`, with the cursor at the end.
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        let cursor = value.chars().count();
        Self { value, cursor }
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    /// Cursor position as a character offset from the start.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Byte offset of a character index, for slicing `value`.
    fn byte_offset(&self, char_index: usize) -> usize {
        self.value
            .char_indices()
            .nth(char_index)
            .map(|(offset, _)| offset)
            .unwrap_or(self.value.len())
    }

    pub fn insert(&mut self, c: char) {
        let offset = self.byte_offset(self.cursor);
        self.value.insert(offset, c);
        self.cursor += 1;
    }

    pub fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        self.cursor -= 1;
        let offset = self.byte_offset(self.cursor);
        self.value.remove(offset);
    }

    pub fn delete(&mut self) {
        if self.cursor >= self.value.chars().count() {
            return;
        }
        let offset = self.byte_offset(self.cursor);
        self.value.remove(offset);
    }

    pub fn left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub fn right(&mut self) {
        self.cursor = (self.cursor + 1).min(self.value.chars().count());
    }

    pub fn home(&mut self) {
        self.cursor = 0;
    }

    pub fn end(&mut self) {
        self.cursor = self.value.chars().count();
    }

    pub fn clear(&mut self) {
        self.value.clear();
        self.cursor = 0;
    }
}
