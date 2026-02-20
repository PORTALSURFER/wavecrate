//! Derived-state dirty graph for native projection/update orchestration.

/// Stable node identifiers for incremental derived-state propagation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub(crate) enum DerivedNodeId {
    /// Source state for waveform position/selection/view edits.
    WaveformState = 0,
    /// Source state for browser filters/search/focus/selection.
    BrowserState = 1,
    /// Source state for map tab/query/pan/zoom selection.
    MapState = 2,
    /// Source state for transport/volume/playback toggles.
    TransportState = 3,
    /// Source state for status/progress/update messaging.
    StatusState = 4,
    /// Derived waveform render input stage.
    WaveformRenderInputs = 5,
    /// Derived waveform image signature stage.
    WaveformImageSignature = 6,
    /// Derived browser visible row list stage.
    BrowserVisibleRows = 7,
    /// Derived browser projection model stage.
    BrowserProjectionModel = 8,
    /// Derived map projection input stage.
    MapProjectionInputs = 9,
    /// Derived map projection model stage.
    MapProjectionModel = 10,
    /// Derived status projection stage.
    StatusProjectionModel = 11,
    /// Projection cache key invalidation stage.
    NativeAppProjectionKey = 12,
}

impl DerivedNodeId {
    /// Total number of graph nodes.
    pub(crate) const COUNT: usize = 13;

    /// Return true when the node is a mutable source-state node.
    pub(crate) fn is_source(self) -> bool {
        (self as usize) <= (DerivedNodeId::StatusState as usize)
    }

    /// Return true when the node is a derived/computed node.
    pub(crate) fn is_derived(self) -> bool {
        !self.is_source()
    }

    fn bit(self) -> u64 {
        1u64 << (self as u8)
    }

    fn index(self) -> usize {
        self as usize
    }
}

/// Reason categories used when marking source nodes dirty.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DirtyReason {
    /// Dirty due to waveform interaction/action flow.
    WaveformAction,
    /// Dirty due to browser interaction/action flow.
    BrowserAction,
    /// Dirty due to map interaction/action flow.
    MapAction,
    /// Dirty due to transport interaction/action flow.
    TransportAction,
    /// Dirty due to status/progress/prompt/update action flow.
    StatusAction,
    /// Dirty due to conservative broad invalidation.
    BroadInvalidation,
}

const TOPO_ORDER: [DerivedNodeId; DerivedNodeId::COUNT] = [
    DerivedNodeId::WaveformState,
    DerivedNodeId::BrowserState,
    DerivedNodeId::MapState,
    DerivedNodeId::TransportState,
    DerivedNodeId::StatusState,
    DerivedNodeId::WaveformRenderInputs,
    DerivedNodeId::WaveformImageSignature,
    DerivedNodeId::BrowserVisibleRows,
    DerivedNodeId::BrowserProjectionModel,
    DerivedNodeId::MapProjectionInputs,
    DerivedNodeId::MapProjectionModel,
    DerivedNodeId::StatusProjectionModel,
    DerivedNodeId::NativeAppProjectionKey,
];

/// Fixed DAG used for deterministic dirty propagation.
#[derive(Clone, Debug)]
pub(crate) struct DerivedStateGraph {
    dirty_mask: u64,
    last_reason_by_node: [Option<DirtyReason>; DerivedNodeId::COUNT],
}

impl DerivedStateGraph {
    /// Build an empty graph with no dirty nodes.
    pub(crate) fn new() -> Self {
        Self {
            dirty_mask: 0,
            last_reason_by_node: [None; DerivedNodeId::COUNT],
        }
    }

    /// Mark a source node dirty and propagate dirtiness to all descendants.
    pub(crate) fn mark_source_dirty(&mut self, source: DerivedNodeId, reason: DirtyReason) {
        debug_assert!(source.is_source());
        self.mark_dirty(source, reason);
    }

    /// Return true when any graph node is dirty.
    pub(crate) fn has_dirty_nodes(&self) -> bool {
        self.dirty_mask != 0
    }

    /// Return true when the given node is dirty.
    pub(crate) fn is_dirty(&self, node: DerivedNodeId) -> bool {
        (self.dirty_mask & node.bit()) != 0
    }

    /// Return dirty nodes in deterministic topological order.
    pub(crate) fn dirty_nodes_in_topo_order(&self) -> Vec<DerivedNodeId> {
        TOPO_ORDER
            .iter()
            .copied()
            .filter(|node| self.is_dirty(*node))
            .collect()
    }

    /// Clear one node's dirty bit and recorded dirty reason.
    pub(crate) fn clear_dirty(&mut self, node: DerivedNodeId) {
        self.dirty_mask &= !node.bit();
        self.last_reason_by_node[node.index()] = None;
    }

    /// Clear all dirty bits and remembered reasons.
    pub(crate) fn clear_all(&mut self) {
        self.dirty_mask = 0;
        self.last_reason_by_node.fill(None);
    }

    /// Count dirty source nodes.
    pub(crate) fn dirty_source_count(&self) -> usize {
        TOPO_ORDER
            .iter()
            .copied()
            .filter(|node| node.is_source() && self.is_dirty(*node))
            .count()
    }

    /// Count dirty derived nodes.
    pub(crate) fn dirty_derived_count(&self) -> usize {
        TOPO_ORDER
            .iter()
            .copied()
            .filter(|node| node.is_derived() && self.is_dirty(*node))
            .count()
    }

    /// Return the last recorded dirty reason for the node, if present.
    pub(crate) fn last_reason(&self, node: DerivedNodeId) -> Option<DirtyReason> {
        self.last_reason_by_node[node.index()]
    }

    fn mark_dirty(&mut self, node: DerivedNodeId, reason: DirtyReason) {
        let mut stack = vec![node];
        while let Some(current) = stack.pop() {
            if !self.is_dirty(current) {
                self.dirty_mask |= current.bit();
                self.last_reason_by_node[current.index()] = Some(reason);
                stack.extend(graph_dependents(current));
            }
        }
    }
}

impl Default for DerivedStateGraph {
    fn default() -> Self {
        Self::new()
    }
}

fn graph_dependents(node: DerivedNodeId) -> &'static [DerivedNodeId] {
    use DerivedNodeId as Node;
    match node {
        Node::WaveformState => &[Node::WaveformRenderInputs],
        Node::BrowserState => &[Node::BrowserVisibleRows],
        Node::MapState => &[Node::MapProjectionInputs],
        Node::TransportState => &[Node::NativeAppProjectionKey],
        Node::StatusState => &[Node::StatusProjectionModel],
        Node::WaveformRenderInputs => &[Node::WaveformImageSignature],
        Node::WaveformImageSignature => &[Node::NativeAppProjectionKey],
        Node::BrowserVisibleRows => &[Node::BrowserProjectionModel],
        Node::BrowserProjectionModel => &[Node::NativeAppProjectionKey],
        Node::MapProjectionInputs => &[Node::MapProjectionModel],
        Node::MapProjectionModel => &[Node::NativeAppProjectionKey],
        Node::StatusProjectionModel => &[Node::NativeAppProjectionKey],
        Node::NativeAppProjectionKey => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::{DerivedNodeId, DerivedStateGraph, DirtyReason};

    #[test]
    fn waveform_source_marks_projection_descendants_dirty() {
        let mut graph = DerivedStateGraph::new();
        graph.mark_source_dirty(DerivedNodeId::WaveformState, DirtyReason::WaveformAction);
        assert!(graph.is_dirty(DerivedNodeId::WaveformState));
        assert!(graph.is_dirty(DerivedNodeId::WaveformRenderInputs));
        assert!(graph.is_dirty(DerivedNodeId::WaveformImageSignature));
        assert!(graph.is_dirty(DerivedNodeId::NativeAppProjectionKey));
        assert_eq!(
            graph.last_reason(DerivedNodeId::WaveformState),
            Some(DirtyReason::WaveformAction)
        );
    }

    #[test]
    fn multiple_sources_merge_dirty_sets_and_counts() {
        let mut graph = DerivedStateGraph::new();
        graph.mark_source_dirty(DerivedNodeId::BrowserState, DirtyReason::BrowserAction);
        graph.mark_source_dirty(DerivedNodeId::MapState, DirtyReason::MapAction);
        assert_eq!(graph.dirty_source_count(), 2);
        assert!(graph.dirty_derived_count() >= 3);
        assert!(graph.is_dirty(DerivedNodeId::NativeAppProjectionKey));
    }

    #[test]
    fn dirty_nodes_return_in_topological_order() {
        let mut graph = DerivedStateGraph::new();
        graph.mark_source_dirty(DerivedNodeId::StatusState, DirtyReason::StatusAction);
        graph.mark_source_dirty(DerivedNodeId::TransportState, DirtyReason::TransportAction);
        let dirty = graph.dirty_nodes_in_topo_order();
        let status_ix = dirty
            .iter()
            .position(|node| *node == DerivedNodeId::StatusState)
            .expect("status node");
        let proj_ix = dirty
            .iter()
            .position(|node| *node == DerivedNodeId::NativeAppProjectionKey)
            .expect("projection node");
        assert!(status_ix < proj_ix);
    }

    #[test]
    fn clear_dirty_and_clear_all_remove_nodes() {
        let mut graph = DerivedStateGraph::new();
        graph.mark_source_dirty(DerivedNodeId::StatusState, DirtyReason::StatusAction);
        graph.clear_dirty(DerivedNodeId::StatusState);
        assert!(!graph.is_dirty(DerivedNodeId::StatusState));
        assert!(graph.is_dirty(DerivedNodeId::StatusProjectionModel));
        graph.clear_all();
        assert!(!graph.has_dirty_nodes());
    }
}
