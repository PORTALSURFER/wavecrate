//! Decoded sample-frame conversion for normalized selection ranges.

use super::range::SelectionRange;

/// Inclusive/exclusive decoded sample-frame bounds for one waveform range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SampleFrameRange {
    /// First decoded frame included in the range.
    pub start_frame: usize,
    /// First decoded frame after the range.
    pub end_frame: usize,
    /// Total decoded frames in the loaded sample used for the conversion.
    pub total_frames: usize,
}

impl SelectionRange {
    /// Create a normalized range whose endpoints exactly represent decoded frame bounds.
    pub fn from_frame_bounds(total_frames: usize, start_frame: usize, end_frame: usize) -> Self {
        if total_frames == 0 {
            return Self::new_precise(0.0, 0.0);
        }
        let start_frame = start_frame.min(total_frames.saturating_sub(1));
        let mut end_frame = end_frame.min(total_frames);
        if end_frame <= start_frame {
            end_frame = (start_frame + 1).min(total_frames);
        }
        Self::new_precise(
            start_frame as f64 / total_frames as f64,
            end_frame as f64 / total_frames as f64,
        )
    }

    /// Convert normalized bounds to decoded sample-frame bounds.
    ///
    /// The start is floored and the end is ceiled so a non-empty authored range
    /// always covers the frames touched by that range.
    pub fn frame_bounds(&self, total_frames: usize) -> SampleFrameRange {
        if total_frames == 0 {
            return SampleFrameRange {
                start_frame: 0,
                end_frame: 0,
                total_frames,
            };
        }
        let start_frame = (self.start_f64() * total_frames as f64).floor() as usize;
        let start_frame = start_frame.min(total_frames.saturating_sub(1));
        let mut end_frame = (self.end_f64() * total_frames as f64).ceil() as usize;
        end_frame = end_frame.min(total_frames);
        if end_frame <= start_frame {
            end_frame = (start_frame + 1).min(total_frames);
        }
        SampleFrameRange {
            start_frame,
            end_frame,
            total_frames,
        }
    }

    /// Create a range by resolving high-precision normalized bounds to decoded frames first.
    pub fn from_precise_normalized_frame_bounds(total_frames: usize, start: f64, end: f64) -> Self {
        let range = Self::new_precise(start, end);
        let frames = range.frame_bounds(total_frames);
        Self::from_frame_bounds(total_frames, frames.start_frame, frames.end_frame)
    }
}
