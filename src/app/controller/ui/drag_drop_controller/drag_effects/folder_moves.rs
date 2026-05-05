//! Folder-local and intra-source drag/drop move handling.
//!
//! The folder-move pipeline is split into:
//! - `plan`: request collection and worker kickoff
//! - `apply_result`: controller-side cache/UI application
//! - `worker`: background execution for folder and folder-sample moves

/// UI-state/result application routines after move workers complete.
mod apply_result;
/// Drag/drop planning and validation entrypoints for folder and sample moves.
mod plan;
/// Background worker tasks that execute filesystem/database move operations.
mod worker;

#[cfg(test)]
mod tests;
