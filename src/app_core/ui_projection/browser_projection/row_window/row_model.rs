use crate::app_core::actions::{
    NativeBrowserRowModel as BrowserRowModel,
    NativeBrowserRowProcessingState as BrowserRowProcessingState, NativePlaybackAgeBucket,
    NativeRetainedVec as RetainedVec,
};
use crate::app_core::state::PlaybackAgeBucket;
use std::sync::Arc;

pub(super) struct BrowserRowProjection<'a> {
    pub(super) visible_row: usize,
    pub(super) row_label: &'a str,
    pub(super) column_index: usize,
    pub(super) rating_level: i8,
    pub(super) playback_age_bucket: PlaybackAgeBucket,
    pub(super) bucket_label: &'a str,
    pub(super) flags: BrowserRowFlags,
    pub(super) processing_state: BrowserRowProcessingState,
    pub(super) similarity_display_strength: Option<u8>,
}

#[derive(Clone, Copy)]
pub(super) struct BrowserRowFlags {
    pub(super) selected: bool,
    pub(super) focused: bool,
    pub(super) missing: bool,
    pub(super) locked: bool,
}

/// Write one browser row into `rows[offset]`, reusing existing `String` buffers.
pub(super) fn write_browser_row_into_slot(
    rows: &mut RetainedVec<BrowserRowModel>,
    offset: usize,
    projection: BrowserRowProjection<'_>,
) {
    let rows = rows.make_mut();
    let bucket_label = (!projection.bucket_label.is_empty()).then_some(projection.bucket_label);
    let native_playback_age_bucket = native_playback_age_bucket(projection.playback_age_bucket);
    if let Some(row) = rows.get_mut(offset) {
        if update_existing_row(row, &projection, bucket_label, native_playback_age_bucket) {
            return;
        }
        return rewrite_existing_row(row, &projection, bucket_label, native_playback_age_bucket);
    }
    rows.push(new_browser_row(
        &projection,
        bucket_label,
        native_playback_age_bucket,
    ));
}

fn update_existing_row(
    row: &mut BrowserRowModel,
    projection: &BrowserRowProjection<'_>,
    bucket_label: Option<&str>,
    native_playback_age_bucket: NativePlaybackAgeBucket,
) -> bool {
    if row.visible_row != projection.visible_row || row.column != projection.column_index.min(2) {
        return false;
    }
    row.selected = projection.flags.selected;
    row.focused = projection.flags.focused;
    row.missing = projection.flags.missing;
    row.locked = projection.flags.locked;
    row.processing_state = projection.processing_state;
    row.similarity_display_strength = projection.similarity_display_strength;
    row.rating_level = projection.rating_level.clamp(-3, 3);
    row.playback_age_bucket = native_playback_age_bucket;
    row.label.as_ref() == projection.row_label && row.bucket_label.as_deref() == bucket_label
}

fn rewrite_existing_row(
    row: &mut BrowserRowModel,
    projection: &BrowserRowProjection<'_>,
    bucket_label: Option<&str>,
    native_playback_age_bucket: NativePlaybackAgeBucket,
) {
    row.visible_row = projection.visible_row;
    if row.label.as_ref() != projection.row_label {
        row.label = Arc::<str>::from(projection.row_label);
    }
    row.column = projection.column_index.min(2);
    row.rating_level = projection.rating_level.clamp(-3, 3);
    row.playback_age_bucket = native_playback_age_bucket;
    row.selected = projection.flags.selected;
    row.focused = projection.flags.focused;
    row.missing = projection.flags.missing;
    row.locked = projection.flags.locked;
    row.processing_state = projection.processing_state;
    row.similarity_display_strength = projection.similarity_display_strength;
    set_bucket_label(row, bucket_label);
}

fn new_browser_row(
    projection: &BrowserRowProjection<'_>,
    bucket_label: Option<&str>,
    native_playback_age_bucket: NativePlaybackAgeBucket,
) -> BrowserRowModel {
    let mut row = BrowserRowModel::new(
        projection.visible_row,
        projection.row_label,
        projection.column_index,
        projection.flags.selected,
        projection.flags.focused,
    )
    .with_rating_level(projection.rating_level)
    .with_playback_age_bucket(native_playback_age_bucket)
    .with_missing(projection.flags.missing)
    .with_locked(projection.flags.locked)
    .with_processing_state(projection.processing_state);
    if let Some(similarity_display_strength) = projection.similarity_display_strength {
        row.similarity_display_strength = Some(similarity_display_strength);
    }
    if let Some(bucket_label) = bucket_label {
        row = row.with_bucket_label(bucket_label);
    }
    row
}

fn set_bucket_label(row: &mut BrowserRowModel, bucket_label: Option<&str>) {
    if let Some(bucket_label) = bucket_label {
        let next_bucket_label = Arc::<str>::from(bucket_label);
        if row.bucket_label.as_deref() != Some(next_bucket_label.as_ref()) {
            row.bucket_label = Some(next_bucket_label);
        }
    } else {
        row.bucket_label = None;
    }
}

/// Convert one app-core playback-age bucket into the native radiant contract enum.
fn native_playback_age_bucket(bucket: PlaybackAgeBucket) -> NativePlaybackAgeBucket {
    match bucket {
        PlaybackAgeBucket::Fresh => NativePlaybackAgeBucket::Fresh,
        PlaybackAgeBucket::OlderThanWeek => NativePlaybackAgeBucket::OlderThanWeek,
        PlaybackAgeBucket::OlderThanMonth => NativePlaybackAgeBucket::OlderThanMonth,
        PlaybackAgeBucket::NeverPlayed => NativePlaybackAgeBucket::NeverPlayed,
    }
}
