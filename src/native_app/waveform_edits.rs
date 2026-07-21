mod atomic_write;
mod completion;
mod entrypoints;
mod prompt;
mod protected_copy;
mod queue;
mod transaction;
mod worker;

pub(in crate::native_app) use worker::WaveformDestructiveEditResult;
#[cfg(test)]
pub(in crate::native_app) use worker::{
    destructive_edit_before_backup_path_for_tests, execute_destructive_edit_for_tests,
};
