//! Always-on, resource-bounded processing for configured sample sources.

mod scheduler;
mod supervisor;

pub(in crate::native_app) use supervisor::SourceProcessingSupervisor;
