//! Retained encoded-scene storage keyed by static frame segment.

use super::*;

pub(in super::super) struct StaticSegmentSceneCacheEntry {
    pub(in super::super) scene: Scene,
}

impl Default for StaticSegmentSceneCacheEntry {
    fn default() -> Self {
        Self {
            scene: Scene::new(),
        }
    }
}

pub(in super::super) struct StaticSegmentSceneCache {
    entries: [StaticSegmentSceneCacheEntry; StaticFrameSegment::COUNT],
}

impl Default for StaticSegmentSceneCache {
    fn default() -> Self {
        Self {
            entries: std::array::from_fn(|_| StaticSegmentSceneCacheEntry::default()),
        }
    }
}

impl StaticSegmentSceneCache {
    pub(in super::super) fn scene(&self, segment: StaticFrameSegment) -> &Scene {
        &self.entries[segment.index()].scene
    }

    pub(in super::super) fn entry_mut(
        &mut self,
        segment: StaticFrameSegment,
    ) -> &mut StaticSegmentSceneCacheEntry {
        &mut self.entries[segment.index()]
    }
}
