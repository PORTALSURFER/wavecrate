mod completion;
mod entrypoints;
mod prompt;
mod protected_copy;
mod queue;
mod transaction;
mod worker;

pub(in crate::native_app) use worker::WaveformDestructiveEditResult;
