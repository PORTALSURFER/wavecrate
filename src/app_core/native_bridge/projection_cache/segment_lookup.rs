//! Segment lookup counters and probe output types for retained projections.

/// Projection segments tracked for retained model refresh and profiling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProjectionSegment {
    /// Footer/status string projection.
    StatusBar,
    /// Browser metadata/chrome/action projection.
    BrowserFrame,
    /// Browser visible-row window projection.
    BrowserRowsWindow,
    /// Similarity map panel projection.
    MapPanel,
    /// Waveform panel/chrome projection.
    WaveformOverlay,
}

/// Hit/miss counters for one retained projection segment.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProjectionSegmentLookupCount {
    /// Number of model pulls that reused retained projection output.
    pub hit_count: u64,
    /// Number of model pulls that recomputed this projection segment.
    pub miss_count: u64,
}

/// Aggregated hit/miss counters for all retained projection segments.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProjectionSegmentLookupCounts {
    /// Status-bar segment counters.
    pub status_bar: ProjectionSegmentLookupCount,
    /// Browser-frame segment counters.
    pub browser_frame: ProjectionSegmentLookupCount,
    /// Browser rows-window segment counters.
    pub browser_rows_window: ProjectionSegmentLookupCount,
    /// Map-panel segment counters.
    pub map_panel: ProjectionSegmentLookupCount,
    /// Waveform-overlay segment counters.
    pub waveform_overlay: ProjectionSegmentLookupCount,
}

impl ProjectionSegmentLookupCounts {
    /// Record one segment-level lookup decision for the current projection pull.
    pub(crate) fn record_lookup(&mut self, segment: ProjectionSegment, hit: bool) {
        let counts = match segment {
            ProjectionSegment::StatusBar => &mut self.status_bar,
            ProjectionSegment::BrowserFrame => &mut self.browser_frame,
            ProjectionSegment::BrowserRowsWindow => &mut self.browser_rows_window,
            ProjectionSegment::MapPanel => &mut self.map_panel,
            ProjectionSegment::WaveformOverlay => &mut self.waveform_overlay,
        };
        if hit {
            counts.hit_count = counts.hit_count.saturating_add(1);
        } else {
            counts.miss_count = counts.miss_count.saturating_add(1);
        }
    }
}

/// Measured output from one fixed retained-projection probe loop.
///
/// The lookup counters reflect the segment reuse decisions observed during the
/// measured iterations only. `projection_p95_us` captures the measured
/// projection-stage latency of those same iterations, excluding warmup passes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProjectionSegmentProbeMeasurement {
    /// Aggregated hit/miss counters observed during measured iterations.
    pub lookup_counts: ProjectionSegmentLookupCounts,
    /// Measured retained-projection p95 latency in microseconds.
    pub projection_p95_us: u64,
}
