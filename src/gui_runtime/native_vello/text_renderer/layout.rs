//! Glyph-layout helpers for the native text renderer.

use super::*;

impl NativeTextRenderer {
    pub(super) fn compute_layout(
        font: &FontData,
        text: &str,
        font_size: f32,
    ) -> Option<TextLayout> {
        let font_ref = skrifa::FontRef::from_index(font.data.as_ref(), font.index).ok()?;
        let charmap = font_ref.charmap();
        let metrics = font_ref.glyph_metrics(FontSize::new(font_size), LocationRef::default());
        let fallback_glyph = charmap.map('?');

        let mut x = 0.0_f32;
        let mut glyphs = Vec::with_capacity(text.len());
        let mut cursor_stops = Vec::with_capacity(text.chars().count() + 1);
        cursor_stops.push(TextCursorStop {
            byte_index: 0,
            x: 0.0,
        });
        for (byte_index, ch) in text.char_indices() {
            if ch == '\n' || ch == '\r' {
                break;
            }
            if ch == '\t' {
                x += font_size * 2.0;
                cursor_stops.push(TextCursorStop {
                    byte_index: byte_index + ch.len_utf8(),
                    x,
                });
                continue;
            }
            if ch == ' ' {
                x += font_size * 0.33;
                cursor_stops.push(TextCursorStop {
                    byte_index: byte_index + ch.len_utf8(),
                    x,
                });
                continue;
            }
            if ch.is_control() {
                cursor_stops.push(TextCursorStop {
                    byte_index: byte_index + ch.len_utf8(),
                    x,
                });
                continue;
            }
            let glyph_id = charmap.map(ch).or(fallback_glyph);
            let Some(glyph_id) = glyph_id else {
                x += font_size * 0.5;
                cursor_stops.push(TextCursorStop {
                    byte_index: byte_index + ch.len_utf8(),
                    x,
                });
                continue;
            };
            glyphs.push(GlyphLayout {
                id: glyph_id.to_u32(),
                x,
            });
            let advance = metrics
                .advance_width(glyph_id)
                .unwrap_or(font_size * 0.55)
                .max(0.0);
            x += advance;
            cursor_stops.push(TextCursorStop {
                byte_index: byte_index + ch.len_utf8(),
                x,
            });
        }

        Some(TextLayout {
            width: x,
            glyphs,
            cursor_stops,
        })
    }
}
