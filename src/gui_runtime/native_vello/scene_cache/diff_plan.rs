//! Static-scene fingerprint diff planning for retained segment rebuilds.

use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
struct StaticSegmentStateNode {
    fingerprint: StaticSegmentCacheFingerprint,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in super::super) struct StaticSegmentDiffPlan {
    fingerprints: [StaticSegmentCacheFingerprint; StaticFrameSegment::COUNT],
    rebuild_bits: u8,
}

impl StaticSegmentDiffPlan {
    pub(in super::super) fn should_rebuild(&self, segment: StaticFrameSegment) -> bool {
        (self.rebuild_bits & (1 << segment.index())) != 0
    }

    pub(in super::super) fn fingerprint(
        &self,
        segment: StaticFrameSegment,
    ) -> &StaticSegmentCacheFingerprint {
        &self.fingerprints[segment.index()]
    }
}

pub(in super::super) struct StaticSegmentStateGraph {
    nodes: [Option<StaticSegmentStateNode>; StaticFrameSegment::COUNT],
}

impl Default for StaticSegmentStateGraph {
    fn default() -> Self {
        Self {
            nodes: std::array::from_fn(|_| None),
        }
    }
}

impl StaticSegmentStateGraph {
    pub(in super::super) fn clear(&mut self) {
        for node in &mut self.nodes {
            *node = None;
        }
    }

    pub(in super::super) fn diff(
        &self,
        dirty_segments: DirtySegments,
        force_rebuild: bool,
        fingerprints: [StaticSegmentCacheFingerprint; StaticFrameSegment::COUNT],
    ) -> StaticSegmentDiffPlan {
        let mut rebuild_bits = 0u8;
        for segment in StaticFrameSegment::ALL {
            let idx = segment.index();
            let explicit_dirty = (dirty_segments.bits() & segment.dirty_mask()) != 0;
            let fingerprint_changed =
                self.nodes[idx].as_ref().map(|node| &node.fingerprint) != Some(&fingerprints[idx]);
            if force_rebuild || explicit_dirty || fingerprint_changed {
                rebuild_bits |= 1 << idx;
            }
        }
        StaticSegmentDiffPlan {
            fingerprints,
            rebuild_bits,
        }
    }

    pub(in super::super) fn commit_segment(
        &mut self,
        segment: StaticFrameSegment,
        fingerprint: &StaticSegmentCacheFingerprint,
    ) {
        self.nodes[segment.index()] = Some(StaticSegmentStateNode {
            fingerprint: fingerprint.clone(),
        });
    }
}
