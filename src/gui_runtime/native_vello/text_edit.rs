//! Single-line text editing helpers shared by native runtime text fields.

use super::*;

/// Mutable selection/caret state for one single-line text field.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct SingleLineTextEditorState {
    anchor_byte: usize,
    cursor_byte: usize,
    scroll_start_byte: usize,
}

/// Precomputed visual layout for one active single-line text field.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct TextFieldLayoutState {
    pub(super) visible_text: String,
    pub(super) caret_offset: f32,
    pub(super) selection_offsets: Option<(f32, f32)>,
    visible_start_byte: usize,
    visible_stops: Vec<TextCursorStop>,
}

impl SingleLineTextEditorState {
    /// Create a collapsed editor state at the end of `text`.
    pub(super) fn collapsed_at_end(text: &str) -> Self {
        let byte = text.len();
        Self {
            anchor_byte: byte,
            cursor_byte: byte,
            scroll_start_byte: 0,
        }
    }

    /// Clamp caret, selection, and scroll state to valid UTF-8 boundaries.
    pub(super) fn clamp_to_text(&mut self, text: &str) {
        self.anchor_byte = clamp_to_char_boundary(text, self.anchor_byte);
        self.cursor_byte = clamp_to_char_boundary(text, self.cursor_byte);
        self.scroll_start_byte = clamp_to_char_boundary(text, self.scroll_start_byte);
    }

    /// Return the sorted active selection range in bytes.
    pub(super) fn selection_range(&self) -> (usize, usize) {
        (
            self.anchor_byte.min(self.cursor_byte),
            self.anchor_byte.max(self.cursor_byte),
        )
    }

    /// Return whether the editor currently has a non-empty selection.
    pub(super) fn has_selection(&self) -> bool {
        self.anchor_byte != self.cursor_byte
    }

    /// Move the caret/selection to `byte_index`.
    pub(super) fn set_cursor(&mut self, text: &str, byte_index: usize, extend_selection: bool) {
        let clamped = clamp_to_char_boundary(text, byte_index);
        if extend_selection {
            self.cursor_byte = clamped;
        } else {
            self.anchor_byte = clamped;
            self.cursor_byte = clamped;
        }
    }

    /// Select the entire text payload.
    pub(super) fn select_all(&mut self, text: &str) {
        self.anchor_byte = 0;
        self.cursor_byte = text.len();
        self.scroll_start_byte = 0;
    }

    /// Move the caret one character to the left.
    pub(super) fn move_left(&mut self, text: &str, extend_selection: bool) -> bool {
        self.clamp_to_text(text);
        let target = if !extend_selection && self.has_selection() {
            self.selection_range().0
        } else {
            previous_char_boundary(text, self.cursor_byte)
        };
        if target == self.cursor_byte && (!extend_selection || self.anchor_byte == target) {
            return false;
        }
        self.set_cursor(text, target, extend_selection);
        true
    }

    /// Move the caret one character to the right.
    pub(super) fn move_right(&mut self, text: &str, extend_selection: bool) -> bool {
        self.clamp_to_text(text);
        let target = if !extend_selection && self.has_selection() {
            self.selection_range().1
        } else {
            next_char_boundary(text, self.cursor_byte)
        };
        if target == self.cursor_byte && (!extend_selection || self.anchor_byte == target) {
            return false;
        }
        self.set_cursor(text, target, extend_selection);
        true
    }

    /// Move the caret to the start of the text.
    pub(super) fn move_home(&mut self, text: &str, extend_selection: bool) -> bool {
        self.clamp_to_text(text);
        if self.cursor_byte == 0 && (!extend_selection || self.anchor_byte == 0) {
            return false;
        }
        self.set_cursor(text, 0, extend_selection);
        true
    }

    /// Move the caret to the end of the text.
    pub(super) fn move_end(&mut self, text: &str, extend_selection: bool) -> bool {
        self.clamp_to_text(text);
        let end = text.len();
        if self.cursor_byte == end && (!extend_selection || self.anchor_byte == end) {
            return false;
        }
        self.set_cursor(text, end, extend_selection);
        true
    }

    /// Replace the selected range with `replacement` and collapse after it.
    pub(super) fn replace_selection(&mut self, text: &str, replacement: &str) -> String {
        self.clamp_to_text(text);
        let (start, end) = self.selection_range();
        let mut next =
            String::with_capacity(text.len() + replacement.len().saturating_sub(end - start));
        next.push_str(&text[..start]);
        next.push_str(replacement);
        next.push_str(&text[end..]);
        let caret = start + replacement.len();
        self.anchor_byte = caret;
        self.cursor_byte = caret;
        self.scroll_start_byte = self.scroll_start_byte.min(caret);
        next
    }

    /// Delete the current selection or the previous character.
    pub(super) fn backspace(&mut self, text: &str) -> Option<String> {
        self.clamp_to_text(text);
        if self.has_selection() {
            return Some(self.replace_selection(text, ""));
        }
        let previous = previous_char_boundary(text, self.cursor_byte);
        if previous == self.cursor_byte {
            return None;
        }
        self.anchor_byte = previous;
        Some(self.replace_selection(text, ""))
    }

    /// Delete the current selection or the next character.
    pub(super) fn delete_forward(&mut self, text: &str) -> Option<String> {
        self.clamp_to_text(text);
        if self.has_selection() {
            return Some(self.replace_selection(text, ""));
        }
        let next = next_char_boundary(text, self.cursor_byte);
        if next == self.cursor_byte {
            return None;
        }
        self.anchor_byte = self.cursor_byte;
        self.cursor_byte = next;
        Some(self.replace_selection(text, ""))
    }

    /// Return the currently selected text, if any.
    pub(super) fn selected_text(&self, text: &str) -> Option<String> {
        let (start, end) = self.selection_range();
        (start < end).then(|| text[start..end].to_string())
    }
}

/// Build one visible text/caret/selection layout for an active text field.
pub(super) fn build_text_field_layout(
    renderer: &mut NativeTextRenderer,
    editor: &mut SingleLineTextEditorState,
    text: &str,
    font_size: f32,
    available_width: f32,
) -> TextFieldLayoutState {
    editor.clamp_to_text(text);
    let width = available_width.max(1.0);
    let layout = renderer
        .layout_text(text, font_size)
        .cloned()
        .unwrap_or_else(|| TextLayout::empty_for(text));
    let left_padding = (font_size * 0.35).clamp(4.0, 12.0);
    let right_padding = left_padding;
    let caret_x = cursor_stop_x(&layout.cursor_stops, editor.cursor_byte);
    let mut scroll_x = cursor_stop_x(&layout.cursor_stops, editor.scroll_start_byte);
    if caret_x - scroll_x > width - right_padding {
        scroll_x = (caret_x - (width - right_padding)).max(0.0);
    } else if caret_x - scroll_x < left_padding {
        scroll_x = (caret_x - left_padding).max(0.0);
    }
    let scroll_start_byte = last_stop_at_or_before_x(&layout.cursor_stops, scroll_x);
    editor.scroll_start_byte = scroll_start_byte;
    let scroll_start_x = cursor_stop_x(&layout.cursor_stops, scroll_start_byte);
    let visible_start_index = stop_index_for_byte(&layout.cursor_stops, scroll_start_byte);
    let visible_end_index = visible_end_stop_index(
        &layout.cursor_stops,
        visible_start_index,
        scroll_start_x,
        width,
    );
    let visible_start_byte = layout.cursor_stops[visible_start_index].byte_index;
    let visible_end_byte = layout.cursor_stops[visible_end_index].byte_index;
    let visible_text = text[visible_start_byte..visible_end_byte].to_string();
    let visible_stops = build_visible_cursor_stops(
        &layout.cursor_stops,
        visible_start_index,
        visible_end_index,
        visible_start_byte,
        scroll_start_x,
        width,
    );
    let caret_offset = (caret_x - scroll_start_x).clamp(0.0, width);
    let (selection_start, selection_end) = editor.selection_range();
    let selection_offsets = if selection_start < selection_end {
        let start = (cursor_stop_x(&layout.cursor_stops, selection_start) - scroll_start_x)
            .clamp(0.0, width);
        let end =
            (cursor_stop_x(&layout.cursor_stops, selection_end) - scroll_start_x).clamp(0.0, width);
        Some((start.min(end), start.max(end)))
    } else {
        None
    };
    TextFieldLayoutState {
        visible_text,
        caret_offset,
        selection_offsets,
        visible_start_byte,
        visible_stops,
    }
}

/// Resolve the nearest text byte index for a pointer x-offset inside the field.
pub(super) fn byte_index_for_local_x(layout: &TextFieldLayoutState, local_x: f32) -> usize {
    let mut best = layout.visible_start_byte;
    let mut best_distance = f32::INFINITY;
    for stop in &layout.visible_stops {
        let distance = (stop.x - local_x).abs();
        if distance < best_distance {
            best = layout.visible_start_byte + stop.byte_index;
            best_distance = distance;
        }
    }
    best
}

/// Sanitize pasted or typed text for single-line fields.
pub(super) fn sanitize_single_line_insert(text: &str) -> String {
    let mut sanitized = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\r' | '\n' => {}
            '\t' => sanitized.push(' '),
            _ if ch.is_control() => {}
            _ => sanitized.push(ch),
        }
    }
    sanitized
}

fn clamp_to_char_boundary(text: &str, byte_index: usize) -> usize {
    let clamped = byte_index.min(text.len());
    if text.is_char_boundary(clamped) {
        return clamped;
    }
    let mut last = 0;
    for (idx, _) in text.char_indices() {
        if idx >= clamped {
            break;
        }
        last = idx;
    }
    last
}

fn previous_char_boundary(text: &str, byte_index: usize) -> usize {
    let clamped = clamp_to_char_boundary(text, byte_index);
    text[..clamped]
        .char_indices()
        .last()
        .map(|(idx, _)| idx)
        .unwrap_or(0)
}

fn next_char_boundary(text: &str, byte_index: usize) -> usize {
    let clamped = clamp_to_char_boundary(text, byte_index);
    if clamped >= text.len() {
        return text.len();
    }
    let mut iter = text[clamped..].char_indices();
    let _ = iter.next();
    iter.next()
        .map(|(idx, _)| clamped + idx)
        .unwrap_or(text.len())
}

fn cursor_stop_x(stops: &[TextCursorStop], byte_index: usize) -> f32 {
    stops
        .iter()
        .find(|stop| stop.byte_index == byte_index)
        .map(|stop| stop.x)
        .unwrap_or_else(|| stops.last().map(|stop| stop.x).unwrap_or(0.0))
}

fn stop_index_for_byte(stops: &[TextCursorStop], byte_index: usize) -> usize {
    stops
        .iter()
        .position(|stop| stop.byte_index == byte_index)
        .unwrap_or_else(|| stops.len().saturating_sub(1))
}

fn last_stop_at_or_before_x(stops: &[TextCursorStop], x: f32) -> usize {
    stops
        .iter()
        .take_while(|stop| stop.x <= x)
        .last()
        .map(|stop| stop.byte_index)
        .unwrap_or(0)
}

fn visible_end_stop_index(
    stops: &[TextCursorStop],
    visible_start_index: usize,
    scroll_start_x: f32,
    width: f32,
) -> usize {
    let mut end = visible_start_index;
    while end + 1 < stops.len() && (stops[end + 1].x - scroll_start_x) <= width {
        end += 1;
    }
    if end == visible_start_index && end + 1 < stops.len() {
        end += 1;
    }
    end
}

fn build_visible_cursor_stops(
    stops: &[TextCursorStop],
    visible_start_index: usize,
    visible_end_index: usize,
    visible_start_byte: usize,
    scroll_start_x: f32,
    width: f32,
) -> Vec<TextCursorStop> {
    stops[visible_start_index..=visible_end_index]
        .iter()
        .map(|stop| TextCursorStop {
            byte_index: stop.byte_index.saturating_sub(visible_start_byte),
            x: (stop.x - scroll_start_x).clamp(0.0, width),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replace_selection_replaces_selected_range_and_collapses_caret() {
        let mut editor = SingleLineTextEditorState::collapsed_at_end("item alpha");
        editor.set_cursor("item alpha", 4, false);
        editor.set_cursor("item alpha", 10, true);

        let value = editor.replace_selection("item alpha", "beta");

        assert_eq!(value, "itembeta");
        assert_eq!(editor.selection_range(), (8, 8));
    }

    #[test]
    fn backspace_deletes_selection_before_single_character() {
        let mut editor = SingleLineTextEditorState::collapsed_at_end("item");
        editor.set_cursor("item", 1, false);
        editor.set_cursor("item", 3, true);
        assert_eq!(editor.backspace("item").as_deref(), Some("im"));
        assert_eq!(editor.selection_range(), (1, 1));

        assert_eq!(editor.backspace("im").as_deref(), Some("m"));
        assert_eq!(editor.selection_range(), (0, 0));
    }

    #[test]
    fn sanitize_single_line_insert_strips_line_breaks_and_controls() {
        assert_eq!(
            sanitize_single_line_insert("it\ne\tm\u{7}"),
            String::from("ite m")
        );
    }

    #[test]
    fn build_text_field_layout_uses_one_full_layout_pass() {
        let mut renderer = NativeTextRenderer::new();
        let mut editor = SingleLineTextEditorState::collapsed_at_end("item alpha beta");

        let layout =
            build_text_field_layout(&mut renderer, &mut editor, "item alpha beta", 14.0, 48.0);

        let (layout_hits, layout_misses, _, _, atom_misses, _) =
            renderer.take_layout_profile_counters();
        assert_eq!((layout_hits, layout_misses), (0, 1));
        assert_eq!(atom_misses, 1);
        assert!(!layout.visible_text.is_empty());
        assert!(!layout.visible_stops.is_empty());
    }
}
