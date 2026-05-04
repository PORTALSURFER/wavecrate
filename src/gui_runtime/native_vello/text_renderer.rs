//! Native text and primitive color/geometry encoding helpers for Vello scenes.

use super::*;

mod cache;
mod font;
mod layout;

#[derive(Clone, Debug)]
pub(super) struct GlyphLayout {
    id: u32,
    x: f32,
}

#[derive(Clone, Debug)]
pub(super) struct TextLayout {
    pub(super) width: f32,
    pub(super) glyphs: Vec<GlyphLayout>,
    pub(super) cursor_stops: Vec<TextCursorStop>,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct TextCursorStop {
    pub(super) byte_index: usize,
    pub(super) x: f32,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub(super) struct TextLayoutKey {
    text: Arc<str>,
    font_size_bits: u32,
}

const TEXT_LAYOUT_CACHE_CAPACITY: usize = 2_048;
const TEXT_ATOM_CACHE_CAPACITY: usize = 4_096;

#[derive(Clone)]
pub(super) struct LoadedFont {
    font: FontData,
}

pub(super) struct NativeTextRenderer {
    loaded_font: Option<LoadedFont>,
    layout_cache: HashMap<TextLayoutKey, TextLayout>,
    layout_cache_order: VecDeque<TextLayoutKey>,
    atom_cache: HashMap<Arc<str>, u64>,
    atom_cache_order: VecDeque<(Arc<str>, u64)>,
    atom_cache_clock: u64,
    text_layout_hits: u64,
    text_layout_misses: u64,
    text_layout_evictions: u64,
    text_atom_hits: u64,
    text_atom_misses: u64,
    text_atom_evictions: u64,
}

impl NativeTextRenderer {
    pub(super) fn new() -> Self {
        let loaded_font = font::load_native_font().map(|font| LoadedFont { font });
        if loaded_font.is_none() {
            eprintln!(
                "Native vello text renderer: no fallback font found; text runs will be skipped"
            );
        }
        Self {
            loaded_font,
            layout_cache: HashMap::with_capacity(TEXT_LAYOUT_CACHE_CAPACITY / 2),
            layout_cache_order: VecDeque::with_capacity(TEXT_LAYOUT_CACHE_CAPACITY),
            atom_cache: HashMap::with_capacity(TEXT_ATOM_CACHE_CAPACITY / 2),
            atom_cache_order: VecDeque::with_capacity(TEXT_ATOM_CACHE_CAPACITY),
            atom_cache_clock: 0,
            text_layout_hits: 0,
            text_layout_misses: 0,
            text_layout_evictions: 0,
            text_atom_hits: 0,
            text_atom_misses: 0,
            text_atom_evictions: 0,
        }
    }

    pub(super) fn draw_text_runs(&mut self, scene: &mut Scene, text_runs: &[TextRun]) {
        let Some(loaded_font) = self.loaded_font.as_ref() else {
            return;
        };
        let font_data = loaded_font.font.clone();
        for run in text_runs {
            if run.text.is_empty() || run.font_size <= 0.0 {
                continue;
            }
            let Some(layout) = self.layout_for(&font_data, &run.text, run.font_size) else {
                continue;
            };
            let mut origin_x = run.position.x;
            if let Some(max_width) = run.max_width {
                let extra = (max_width - layout.width).max(0.0);
                origin_x += match run.align {
                    TextAlign::Left => 0.0,
                    TextAlign::Center => extra * 0.5,
                    TextAlign::Right => extra,
                };
            }
            let clip_width = run.max_width.unwrap_or(f32::INFINITY);
            let baseline = run.position.y + run.font_size;
            let glyph_iter = layout
                .glyphs
                .iter()
                .take_while(|glyph| glyph.x <= clip_width)
                .map(|glyph| Glyph {
                    id: glyph.id,
                    x: origin_x + glyph.x,
                    y: baseline,
                });
            scene
                .draw_glyphs(&font_data)
                .font_size(run.font_size)
                .brush(color_from_rgba(run.color))
                .draw(Fill::NonZero, glyph_iter);
        }
    }

    pub(super) fn layout_text(&mut self, text: &str, font_size: f32) -> Option<&TextLayout> {
        let font = self.loaded_font.as_ref()?.font.clone();
        self.layout_for(&font, text, font_size)
    }
}

impl TextLayout {
    pub(super) fn empty_for(text: &str) -> Self {
        Self {
            width: 0.0,
            glyphs: Vec::new(),
            cursor_stops: vec![TextCursorStop {
                byte_index: text.len(),
                x: 0.0,
            }],
        }
    }
}

pub(super) fn to_kurbo_rect(rect: UiRect) -> KurboRect {
    KurboRect::new(
        rect.min.x as f64,
        rect.min.y as f64,
        rect.max.x as f64,
        rect.max.y as f64,
    )
}

pub(super) fn color_from_rgba(color: Rgba8) -> Color {
    Color::from_rgba8(color.r, color.g, color.b, color.a)
}

pub(super) fn icon_from_rgba(icon: &WindowIconRgba) -> Option<Icon> {
    Icon::from_rgba(icon.rgba.clone(), icon.width, icon.height).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_layout_preserves_terminal_cursor_stop() {
        let layout = TextLayout::empty_for("tempo");
        assert_eq!(layout.width, 0.0);
        assert!(layout.glyphs.is_empty());
        assert_eq!(
            layout.cursor_stops,
            vec![TextCursorStop {
                byte_index: 5,
                x: 0.0,
            }]
        );
    }

    #[test]
    fn intern_text_reuses_cached_atom_and_tracks_hits() {
        let mut renderer = NativeTextRenderer::new();

        let first = renderer.intern_text("browser row");
        let second = renderer.intern_text("browser row");

        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(renderer.take_layout_profile_counters(), (0, 0, 0, 1, 1, 0));
    }

    #[test]
    fn atom_cache_eviction_drops_old_entries_once_capacity_is_exceeded() {
        let mut renderer = NativeTextRenderer::new();
        for index in 0..=TEXT_ATOM_CACHE_CAPACITY {
            let _ = renderer.intern_text(format!("label-{index}").as_str());
        }

        let (_, _, _, _, misses, evictions) = renderer.take_layout_profile_counters();
        assert_eq!(misses, (TEXT_ATOM_CACHE_CAPACITY as u64) + 1);
        assert!(evictions > 0);
        assert!(renderer.atom_cache.len() <= TEXT_ATOM_CACHE_CAPACITY);
    }

    #[test]
    fn atom_cache_hit_queue_compacts_after_repeated_reuse() {
        let mut renderer = NativeTextRenderer::new();
        let _ = renderer.intern_text("browser row");
        for _ in 0..=TEXT_ATOM_CACHE_CAPACITY.saturating_mul(2) {
            let _ = renderer.intern_text("browser row");
        }

        assert_eq!(renderer.atom_cache.len(), 1);
        assert!(renderer.atom_cache_order.len() <= TEXT_ATOM_CACHE_CAPACITY);
    }
}
