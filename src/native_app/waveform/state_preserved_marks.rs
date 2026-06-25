use super::{WaveformState, similar_sections::SimilarSectionsState};
use wavecrate::selection::{SampleFrameRange, SelectionRange};

#[derive(Clone, Debug)]
pub(in crate::native_app) struct WaveformPreservedMarks {
    play_mark_ratio: Option<f32>,
    edit_mark_ratio: Option<f32>,
    play_selection: Option<SelectionRange>,
    edit_selection: Option<SelectionRange>,
    marked_play_ranges: Vec<SelectionRange>,
    extracted_ranges: Vec<SelectionRange>,
    similar_sections: SimilarSectionsState,
    play_selection_flash_frames: u8,
    edit_selection_flash_frames: u8,
}

impl WaveformState {
    pub(in crate::native_app) fn preserved_marks_unchanged(&self) -> WaveformPreservedMarks {
        self.preserved_marks_with_transform(FrameRangeTransform::identity(self.file.frames))
    }

    pub(in crate::native_app) fn preserved_marks_after_trim(
        &self,
        selection: SelectionRange,
    ) -> WaveformPreservedMarks {
        let transform = FrameRangeTransform::trim(self.file.frames, selection);
        self.preserved_marks_with_transform(transform)
    }

    pub(in crate::native_app) fn preserved_marks_after_crop(
        &self,
        selection: SelectionRange,
    ) -> WaveformPreservedMarks {
        let transform = FrameRangeTransform::crop(self.file.frames, selection);
        self.preserved_marks_with_transform(transform)
    }

    pub(in crate::native_app) fn restore_preserved_marks(&mut self, marks: WaveformPreservedMarks) {
        self.play_mark_ratio = marks.play_mark_ratio;
        self.edit_mark_ratio = marks.edit_mark_ratio;
        self.play_selection = marks.play_selection;
        self.edit_selection = marks.edit_selection;
        self.marked_play_ranges = marks.marked_play_ranges;
        self.extracted_ranges = marks.extracted_ranges;
        self.similar_sections = marks.similar_sections;
        self.play_selection_flash_frames = marks.play_selection_flash_frames;
        self.edit_selection_flash_frames = marks.edit_selection_flash_frames;
        self.active_drag = None;
        self.pending_playback_start = None;
    }

    fn preserved_marks_with_transform(
        &self,
        transform: FrameRangeTransform,
    ) -> WaveformPreservedMarks {
        let similar_anchor = self
            .similar_sections
            .anchor
            .and_then(|anchor| transform.map_range(anchor));
        let similar_ranges = self
            .similar_sections
            .ranges
            .iter()
            .filter_map(|range| transform.map_range(*range))
            .collect();
        WaveformPreservedMarks {
            play_mark_ratio: self
                .play_mark_ratio
                .and_then(|ratio| transform.map_ratio(ratio)),
            edit_mark_ratio: self
                .edit_mark_ratio
                .and_then(|ratio| transform.map_ratio(ratio)),
            play_selection: self
                .play_selection
                .and_then(|selection| transform.map_range(selection)),
            edit_selection: self
                .edit_selection
                .and_then(|selection| transform.map_range(selection)),
            marked_play_ranges: self
                .marked_play_ranges
                .iter()
                .filter_map(|range| transform.map_range(*range))
                .collect(),
            extracted_ranges: self
                .extracted_ranges
                .iter()
                .filter_map(|range| transform.map_range(*range))
                .collect(),
            similar_sections: SimilarSectionsState {
                enabled: self.similar_sections.enabled && similar_anchor.is_some(),
                anchor: similar_anchor,
                ranges: similar_ranges,
            },
            play_selection_flash_frames: self.play_selection_flash_frames,
            edit_selection_flash_frames: self.edit_selection_flash_frames,
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum FrameRangeTransform {
    Identity {
        total_frames: usize,
    },
    Trim {
        old_total_frames: usize,
        new_total_frames: usize,
        removed: SampleFrameRange,
    },
    Crop {
        old_total_frames: usize,
        new_total_frames: usize,
        kept: SampleFrameRange,
    },
}

impl FrameRangeTransform {
    fn identity(total_frames: usize) -> Self {
        Self::Identity { total_frames }
    }

    fn trim(old_total_frames: usize, selection: SelectionRange) -> Self {
        let removed = selection.frame_bounds(old_total_frames);
        let removed_width = removed.end_frame.saturating_sub(removed.start_frame);
        Self::Trim {
            old_total_frames,
            new_total_frames: old_total_frames.saturating_sub(removed_width),
            removed,
        }
    }

    fn crop(old_total_frames: usize, selection: SelectionRange) -> Self {
        let kept = selection.frame_bounds(old_total_frames);
        Self::Crop {
            old_total_frames,
            new_total_frames: kept.end_frame.saturating_sub(kept.start_frame),
            kept,
        }
    }

    fn map_ratio(self, ratio: f32) -> Option<f32> {
        let old_total_frames = self.old_total_frames();
        let new_total_frames = self.new_total_frames();
        if old_total_frames == 0 || new_total_frames == 0 {
            return None;
        }
        let frame = ratio_frame(ratio, old_total_frames);
        let mapped = match self {
            Self::Identity { .. } => frame,
            Self::Trim { removed, .. } => {
                if frame < removed.start_frame {
                    frame
                } else if frame >= removed.end_frame {
                    frame.saturating_sub(removed.end_frame - removed.start_frame)
                } else {
                    return None;
                }
            }
            Self::Crop { kept, .. } => {
                if frame < kept.start_frame || frame > kept.end_frame {
                    return None;
                }
                frame.saturating_sub(kept.start_frame)
            }
        };
        Some((mapped as f64 / new_total_frames as f64) as f32)
    }

    fn map_range(self, range: SelectionRange) -> Option<SelectionRange> {
        let old_total_frames = self.old_total_frames();
        let new_total_frames = self.new_total_frames();
        if old_total_frames == 0 || new_total_frames == 0 {
            return None;
        }
        let bounds = range.frame_bounds(old_total_frames);
        let (start_frame, end_frame) = match self {
            Self::Identity { .. } => (bounds.start_frame, bounds.end_frame),
            Self::Trim { removed, .. } => map_trimmed_range(bounds, removed)?,
            Self::Crop { kept, .. } => map_cropped_range(bounds, kept)?,
        };
        Some(range.with_bounds_precise(
            start_frame as f64 / new_total_frames as f64,
            end_frame as f64 / new_total_frames as f64,
        ))
    }

    fn old_total_frames(self) -> usize {
        match self {
            Self::Identity { total_frames } => total_frames,
            Self::Trim {
                old_total_frames, ..
            }
            | Self::Crop {
                old_total_frames, ..
            } => old_total_frames,
        }
    }

    fn new_total_frames(self) -> usize {
        match self {
            Self::Identity { total_frames } => total_frames,
            Self::Trim {
                new_total_frames, ..
            }
            | Self::Crop {
                new_total_frames, ..
            } => new_total_frames,
        }
    }
}

fn map_trimmed_range(
    bounds: SampleFrameRange,
    removed: SampleFrameRange,
) -> Option<(usize, usize)> {
    if bounds.end_frame <= removed.start_frame {
        return Some((bounds.start_frame, bounds.end_frame));
    }
    if bounds.start_frame >= removed.end_frame {
        let removed_width = removed.end_frame - removed.start_frame;
        return Some((
            bounds.start_frame - removed_width,
            bounds.end_frame - removed_width,
        ));
    }
    None
}

fn map_cropped_range(bounds: SampleFrameRange, kept: SampleFrameRange) -> Option<(usize, usize)> {
    let start_frame = bounds.start_frame.max(kept.start_frame);
    let end_frame = bounds.end_frame.min(kept.end_frame);
    if end_frame <= start_frame {
        return None;
    }
    Some((start_frame - kept.start_frame, end_frame - kept.start_frame))
}

fn ratio_frame(ratio: f32, total_frames: usize) -> usize {
    let ratio = f64::from(ratio.clamp(0.0, 1.0));
    (ratio * total_frames as f64).round() as usize
}
