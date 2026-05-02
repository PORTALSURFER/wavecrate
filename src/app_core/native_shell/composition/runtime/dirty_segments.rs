use crate::gui::invalidation::{RetainedSegmentMask, RetainedSegmentRevisions};

/// Bitmask describing which projection segments changed during the last model pull.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DirtySegments {
    mask: RetainedSegmentMask<0x00ff, 0x003f, 0x00c0>,
}

impl DirtySegments {
    /// Status-bar content segment.
    pub const STATUS_BAR: u16 = 1 << 0;
    /// Browser metadata/chrome segment.
    pub const BROWSER_FRAME: u16 = 1 << 1;
    /// Browser row-window segment.
    pub const BROWSER_ROWS_WINDOW: u16 = 1 << 2;
    /// Map-panel segment.
    pub const MAP_PANEL: u16 = 1 << 3;
    /// Waveform panel/chrome segment.
    pub const WAVEFORM_OVERLAY: u16 = 1 << 4;
    /// Static content that is outside explicit segment buckets.
    pub const GLOBAL_STATIC: u16 = 1 << 5;
    /// State-overlay model fields.
    pub const STATE_OVERLAY: u16 = 1 << 6;
    /// Motion-overlay model fields.
    pub const MOTION_OVERLAY: u16 = 1 << 7;

    /// Return an empty segment mask.
    pub const fn empty() -> Self {
        Self {
            mask: RetainedSegmentMask::empty(),
        }
    }

    /// Return a full segment mask.
    pub const fn all() -> Self {
        Self {
            mask: RetainedSegmentMask::all(),
        }
    }

    /// Construct a segment mask from raw bits.
    pub const fn from_bits(bits: u16) -> Self {
        Self {
            mask: RetainedSegmentMask::from_bits(bits),
        }
    }

    /// Return raw bit contents for diagnostics and tests.
    pub const fn bits(self) -> u16 {
        self.mask.bits()
    }

    /// Return `true` when the mask contains no segments.
    pub const fn is_empty(self) -> bool {
        self.mask.is_empty()
    }

    /// Return `true` when any static segment requires rebuild.
    pub const fn requires_static_rebuild(self) -> bool {
        self.mask.requires_static_rebuild()
    }

    /// Return `true` when any overlay segment requires rebuild.
    pub const fn requires_overlay_rebuild(self) -> bool {
        self.mask.requires_overlay_rebuild()
    }

    /// Insert one or more segment bits into this mask.
    pub fn insert(&mut self, bits: u16) {
        self.mask.insert(bits);
    }
}

/// Monotonic revision counters for static projection segments.
///
/// Bridges bump the counters for segments whose projected model slices changed on
/// the most recent `pull_model`. Runtimes use these revisions in retained-scene
/// cache keys to avoid expensive segment hashing on every frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SegmentRevisions {
    /// Status-bar projection revision.
    pub status_bar: u64,
    /// Browser metadata/chrome projection revision.
    pub browser_frame: u64,
    /// Browser visible-row window projection revision.
    pub browser_rows_window: u64,
    /// Map-panel projection revision.
    pub map_panel: u64,
    /// Waveform panel/chrome projection revision.
    pub waveform_overlay: u64,
    /// Global static fields projection revision.
    pub global_static: u64,
}

impl SegmentRevisions {
    /// Return these named compatibility revisions as a generic retained segment array.
    pub const fn retained_revisions(self) -> RetainedSegmentRevisions<6> {
        RetainedSegmentRevisions::new([
            self.status_bar,
            self.browser_frame,
            self.browser_rows_window,
            self.map_panel,
            self.waveform_overlay,
            self.global_static,
        ])
    }

    /// Return whether any static-segment revision is non-zero.
    pub fn has_static_revisions(self) -> bool {
        self.retained_revisions().has_revisions()
    }

    /// Bump revisions for the static segments flagged in `dirty_segments`.
    pub fn bump_for_dirty_segments(&mut self, dirty_segments: DirtySegments) {
        let bits = dirty_segments.bits();
        let mut revisions = self.retained_revisions();
        revisions.bump_for_bits(
            bits,
            [
                DirtySegments::STATUS_BAR,
                DirtySegments::BROWSER_FRAME,
                DirtySegments::BROWSER_ROWS_WINDOW,
                DirtySegments::MAP_PANEL,
                DirtySegments::WAVEFORM_OVERLAY,
                DirtySegments::GLOBAL_STATIC,
            ],
        );
        let [
            status_bar,
            browser_frame,
            browser_rows_window,
            map_panel,
            waveform_overlay,
            global_static,
        ] = revisions.revisions;
        self.status_bar = status_bar;
        self.browser_frame = browser_frame;
        self.browser_rows_window = browser_rows_window;
        self.map_panel = map_panel;
        self.waveform_overlay = waveform_overlay;
        self.global_static = global_static;
    }
}
