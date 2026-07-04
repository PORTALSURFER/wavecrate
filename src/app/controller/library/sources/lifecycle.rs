use super::super::*;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use crate::logging::{ActionDebugEvent, emit_action_debug_event};

mod add;
mod open_folder;
mod remap;
mod remove;
mod telemetry;
mod validation;

#[cfg(test)]
pub(crate) use add::with_source_add_async_enabled_for_tests;

impl AppController {
    pub(crate) fn database_for(
        &mut self,
        source: &SampleSource,
    ) -> Result<Rc<SourceDatabase>, SourceDbError> {
        self.cache.database_for(source)
    }

    pub(crate) fn cache_db(
        &mut self,
        source: &SampleSource,
    ) -> Result<Rc<SourceDatabase>, SourceDbError> {
        self.database_for(source)
    }
}
