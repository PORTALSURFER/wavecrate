mod execution;
mod filesystem;
mod messages;

#[cfg(not(test))]
pub(crate) use execution::run_trash_move_task;
#[cfg(test)]
pub(crate) use execution::run_trash_move_task_with_progress;
pub(crate) use filesystem::move_to_trash;
pub(crate) use messages::{TrashMoveFinished, TrashMoveMessage};

#[cfg(test)]
#[path = "trash_move/tests.rs"]
mod tests;
