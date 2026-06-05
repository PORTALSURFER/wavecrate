//! Compatibility exports for Wavecrate's reusable audio foundation.
//!
//! Generic audio infrastructure lives in the `reson` crate. This module keeps
//! existing Wavecrate imports stable and owns conversion from Wavecrate-specific
//! selection state into neutral realtime audio fade ranges.

pub use reson::*;

use crate::selection::SelectionRange;

/// Convert a Wavecrate waveform selection into a reusable `reson` edit-fade range.
pub fn edit_fade_range_from_selection(range: Option<SelectionRange>) -> Option<EditFadeRange> {
    range.map(|range| {
        EditFadeRange::new(
            range.start(),
            range.end(),
            range.gain(),
            range
                .fade_in()
                .map(|fade| FadeParams::new(fade.length, fade.curve, fade.mute)),
            range
                .fade_out()
                .map(|fade| FadeParams::new(fade.length, fade.curve, fade.mute)),
        )
    })
}
