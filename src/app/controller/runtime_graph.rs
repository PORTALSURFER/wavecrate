//! Controller helpers for derived-state dirty graph access.

use super::AppController;
use crate::app::controller::state::runtime::{DerivedNodeId, DirtyReason};

impl AppController {
    /// Mark one source node dirty and propagate to derived descendants.
    pub(crate) fn mark_derived_source_dirty(&mut self, source: DerivedNodeId, reason: DirtyReason) {
        self.runtime.derived_graph.mark_source_dirty(source, reason);
    }

    /// Mark multiple source nodes dirty under one reason category.
    pub(crate) fn mark_derived_sources_dirty(
        &mut self,
        sources: &[DerivedNodeId],
        reason: DirtyReason,
    ) {
        for source in sources.iter().copied() {
            self.mark_derived_source_dirty(source, reason);
        }
    }

    /// Return true when any derived graph node is currently dirty.
    pub(crate) fn has_dirty_derived_nodes(&self) -> bool {
        self.runtime.derived_graph.has_dirty_nodes()
    }

    /// Return dirty graph nodes in deterministic topological order.
    pub(crate) fn dirty_derived_nodes_in_topo_order(&self) -> Vec<DerivedNodeId> {
        self.runtime.derived_graph.dirty_nodes_in_topo_order()
    }

    /// Clear one dirty node after successful recompute.
    pub(crate) fn clear_derived_dirty_node(&mut self, node: DerivedNodeId) {
        self.runtime.derived_graph.clear_dirty(node);
    }

    /// Count how many source nodes are currently dirty.
    pub(crate) fn dirty_derived_source_count(&self) -> usize {
        self.runtime.derived_graph.dirty_source_count()
    }

    /// Count how many derived nodes are currently dirty.
    pub(crate) fn dirty_derived_computed_count(&self) -> usize {
        self.runtime.derived_graph.dirty_derived_count()
    }

    /// Return the last dirty reason recorded for one node.
    pub(crate) fn derived_dirty_reason(&self, node: DerivedNodeId) -> Option<DirtyReason> {
        self.runtime.derived_graph.last_reason(node)
    }

    #[cfg(test)]
    /// Return true when a specific derived graph node is dirty.
    pub(crate) fn is_derived_node_dirty_for_test(&self, node: DerivedNodeId) -> bool {
        self.runtime.derived_graph.is_dirty(node)
    }
}
