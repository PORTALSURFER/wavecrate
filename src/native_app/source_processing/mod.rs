//! Always-on, resource-bounded processing for configured sample sources.

mod scheduler;
mod supervisor;
mod worker;

pub(in crate::native_app) use supervisor::SourceProcessingSupervisor;
pub(in crate::native_app) use worker::run_internal_source_analysis_from_args;
#[cfg(not(test))]
pub(in crate::native_app) use worker::wait_for_cancellable_child;
