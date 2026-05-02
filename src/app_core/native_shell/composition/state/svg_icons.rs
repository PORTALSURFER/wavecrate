//! Minimal SVG icon parser/rasterizer for native shell glyphs.
//!
//! The native shell paint model is backend-neutral and currently supports
//! rectangles/circles/images/text primitives. This module loads toolbar glyph
//! definitions from asset-backed SVG files and rasterizes them into RGBA images
//! so toolbar controls can render iconography without adding a new primitive
//! kind.

#[cfg(test)]
use self::sempal_crate::app as native_model;
use super::*;
#[cfg(test)]
use crate as sempal_crate;
use std::sync::Arc;

use crate::gui::svg::{parse_svg_document, point_in_svg_shapes};

/// Icon identifiers used by native shell controls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum WaveformToolbarIcon {
    /// Mono channel-view icon.
    Mono,
    /// Stereo channel-view icon.
    Stereo,
    /// Normalize audition toggle icon.
    Normalize,
    /// BPM snap icon.
    BpmSnap,
    /// Selection-relative BPM grid origin icon.
    RelativeBpmGrid,
    /// Transient snap icon.
    TransientSnap,
    /// Show transient markers icon.
    ShowTransients,
    /// Slice mode icon.
    Slice,
    /// Loop toggle icon.
    Loop,
    /// Lock overlay icon used by locked controls.
    Lock,
    /// Stop transport icon.
    Stop,
    /// Play transport icon used in both idle and running states.
    Play,
    /// Record icon placeholder.
    Record,
    /// Browser random-navigation toggle icon.
    Dice,
    /// Browser focused-row similarity-search icon.
    Similarity,
    /// Sidebar folder-visibility toggle icon.
    Filter,
    /// Sidebar flattened-view toggle icon.
    Flatten,
    /// Browser playback-age filter icon for samples with no playback history.
    BrowserNeverPlayed,
    /// Browser playback-age filter icon for samples older than one month.
    BrowserOlderThanMonth,
    /// Browser playback-age filter icon for samples older than one week.
    BrowserOlderThanWeek,
    /// Browser marked-only filter icon.
    BrowserMarked,
}

/// Return a toolbar icon for one waveform toolbar button.
pub(super) fn toolbar_icon_for_button(
    button: &WaveformToolbarButton,
) -> Option<WaveformToolbarIcon> {
    button.icon
}

/// Emit one SVG-backed toolbar icon into the primitive list.
pub(super) fn emit_toolbar_svg_icon(
    primitives: &mut impl PrimitiveSink,
    icon: WaveformToolbarIcon,
    rect: Rect,
    color: Rgba8,
) -> bool {
    let side = rect.width().min(rect.height()).round().clamp(8.0, 32.0) as usize;
    let Some(image) = rasterize_svg_icon(icon, side, color) else {
        return false;
    };
    emit_primitive(
        primitives,
        Primitive::Image(DrawImage {
            rect,
            image: Arc::new(image),
        }),
    );
    true
}

fn rasterize_svg_icon(icon: WaveformToolbarIcon, side: usize, color: Rgba8) -> Option<ImageRgba> {
    let svg = icon_svg_asset(icon);
    let document = parse_svg_document(svg)?;
    let mut pixels = vec![0_u8; side.saturating_mul(side).saturating_mul(4)];
    let sample_offsets = [
        (0.25_f32, 0.25_f32),
        (0.75, 0.25),
        (0.25, 0.75),
        (0.75, 0.75),
    ];

    for y in 0..side {
        for x in 0..side {
            let mut hits = 0_u8;
            for (offset_x, offset_y) in sample_offsets {
                let world_x = document.view_box_min_x
                    + ((x as f32 + offset_x) / side as f32) * document.view_box_width;
                let world_y = document.view_box_min_y
                    + ((y as f32 + offset_y) / side as f32) * document.view_box_height;
                if point_in_svg_shapes(world_x, world_y, &document.shapes) {
                    hits = hits.saturating_add(1);
                }
            }
            if hits == 0 {
                continue;
            }
            let coverage = hits as f32 / sample_offsets.len() as f32;
            let alpha = ((color.a as f32) * coverage).round().clamp(0.0, 255.0) as u8;
            let index = (y * side + x) * 4;
            pixels[index] = color.r;
            pixels[index + 1] = color.g;
            pixels[index + 2] = color.b;
            pixels[index + 3] = alpha;
        }
    }

    ImageRgba::new(side, side, pixels)
}
fn icon_svg_asset(icon: WaveformToolbarIcon) -> &'static str {
    match icon {
        WaveformToolbarIcon::Mono => {
            include_str!("../assets/icons/waveform_toolbar/mono.svg")
        }
        WaveformToolbarIcon::Stereo => {
            include_str!("../assets/icons/waveform_toolbar/stereo.svg")
        }
        WaveformToolbarIcon::Normalize => {
            include_str!("../assets/icons/waveform_toolbar/normalize.svg")
        }
        WaveformToolbarIcon::BpmSnap => {
            include_str!("../assets/icons/waveform_toolbar/bpm_snap.svg")
        }
        WaveformToolbarIcon::RelativeBpmGrid => {
            include_str!("../assets/icons/waveform_toolbar/relative_bpm_grid.svg")
        }
        WaveformToolbarIcon::TransientSnap => {
            include_str!("../assets/icons/waveform_toolbar/transient_snap.svg")
        }
        WaveformToolbarIcon::ShowTransients => {
            include_str!("../assets/icons/waveform_toolbar/show_transients.svg")
        }
        WaveformToolbarIcon::Slice => {
            include_str!("../assets/icons/waveform_toolbar/slice.svg")
        }
        WaveformToolbarIcon::Play => {
            include_str!("../assets/icons/waveform_toolbar/play.svg")
        }
        WaveformToolbarIcon::Stop => {
            include_str!("../assets/icons/waveform_toolbar/stop.svg")
        }
        WaveformToolbarIcon::Record => {
            include_str!("../assets/icons/waveform_toolbar/record.svg")
        }
        WaveformToolbarIcon::Loop => {
            include_str!("../assets/icons/waveform_toolbar/loop.svg")
        }
        WaveformToolbarIcon::Lock => {
            include_str!("../assets/icons/ui/lock.svg")
        }
        WaveformToolbarIcon::Dice => {
            include_str!("../assets/icons/ui/dice.svg")
        }
        WaveformToolbarIcon::Similarity => {
            include_str!("../assets/icons/ui/similarity.svg")
        }
        WaveformToolbarIcon::Filter => {
            include_str!("../assets/icons/ui/filter.svg")
        }
        WaveformToolbarIcon::Flatten => {
            include_str!("../assets/icons/ui/flatten.svg")
        }
        WaveformToolbarIcon::BrowserNeverPlayed => {
            include_str!("../assets/icons/ui/browser_never_played.svg")
        }
        WaveformToolbarIcon::BrowserOlderThanMonth => {
            include_str!("../assets/icons/ui/browser_older_than_month.svg")
        }
        WaveformToolbarIcon::BrowserOlderThanWeek => {
            include_str!("../assets/icons/ui/browser_older_than_week.svg")
        }
        WaveformToolbarIcon::BrowserMarked => {
            include_str!("../assets/icons/ui/browser_marked.svg")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::types::{Point, Rect, Rgba8};
    use native_model::UiAction;

    fn waveform_toolbar_button(label: &'static str, active: bool) -> WaveformToolbarButton {
        WaveformToolbarButton {
            rect: Rect::from_min_max(Point::new(0.0, 0.0), Point::new(18.0, 18.0)),
            label,
            icon: toolbar_icon_for_label(label),
            overlay_icon: None,
            display_text: None,
            enabled: true,
            active,
            action: Some(UiAction::ToggleTransport),
            text_color: Rgba8 {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
        }
    }

    fn toolbar_icon_for_label(label: &'static str) -> Option<WaveformToolbarIcon> {
        match label {
            "Channel Mono" => Some(WaveformToolbarIcon::Mono),
            "Channel Stereo" => Some(WaveformToolbarIcon::Stereo),
            "Play" => Some(WaveformToolbarIcon::Play),
            "Stop" => Some(WaveformToolbarIcon::Stop),
            _ => None,
        }
    }

    #[test]
    fn transport_button_swaps_between_play_and_stop_icons() {
        let idle_button = waveform_toolbar_button("Play", false);
        let running_button = waveform_toolbar_button("Stop", true);

        assert_eq!(
            toolbar_icon_for_button(&idle_button),
            Some(WaveformToolbarIcon::Play)
        );
        assert_eq!(
            toolbar_icon_for_button(&running_button),
            Some(WaveformToolbarIcon::Stop)
        );
    }

    #[test]
    fn channel_button_swaps_icons_between_mono_and_stereo_states() {
        let mono_button = waveform_toolbar_button("Channel Mono", false);
        let stereo_button = waveform_toolbar_button("Channel Stereo", false);

        assert_eq!(
            toolbar_icon_for_button(&mono_button),
            Some(WaveformToolbarIcon::Mono)
        );
        assert_eq!(
            toolbar_icon_for_button(&stereo_button),
            Some(WaveformToolbarIcon::Stereo)
        );
    }

    #[test]
    fn asset_backed_svg_icons_parse_successfully() {
        for icon in [
            WaveformToolbarIcon::Mono,
            WaveformToolbarIcon::Stereo,
            WaveformToolbarIcon::Normalize,
            WaveformToolbarIcon::BpmSnap,
            WaveformToolbarIcon::RelativeBpmGrid,
            WaveformToolbarIcon::TransientSnap,
            WaveformToolbarIcon::ShowTransients,
            WaveformToolbarIcon::Slice,
            WaveformToolbarIcon::Loop,
            WaveformToolbarIcon::Lock,
            WaveformToolbarIcon::Stop,
            WaveformToolbarIcon::Play,
            WaveformToolbarIcon::Record,
            WaveformToolbarIcon::Dice,
            WaveformToolbarIcon::Similarity,
            WaveformToolbarIcon::Filter,
            WaveformToolbarIcon::Flatten,
            WaveformToolbarIcon::BrowserNeverPlayed,
            WaveformToolbarIcon::BrowserOlderThanMonth,
            WaveformToolbarIcon::BrowserOlderThanWeek,
            WaveformToolbarIcon::BrowserMarked,
        ] {
            let document = parse_svg_document(icon_svg_asset(icon));
            assert!(document.is_some(), "svg asset for {icon:?} should parse");
            assert!(
                !document.expect("document should exist").shapes.is_empty(),
                "svg asset for {icon:?} should yield visible shapes"
            );
        }
    }
}
