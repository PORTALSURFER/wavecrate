//! Source database handles and paged WAV-entry caches.

use super::WavEntriesState;
use crate::sample_sources::{SampleSource, SourceDatabase, SourceDbError, SourceId, WavEntry};
use std::collections::HashMap;
use std::rc::Rc;

pub(crate) struct WavCacheState {
    pub(crate) entries: HashMap<SourceId, WavEntriesState>,
}

impl WavCacheState {
    pub(crate) fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub(crate) fn insert_page(
        &mut self,
        source_id: SourceId,
        total: usize,
        page_size: usize,
        page_index: usize,
        entries: Vec<WavEntry>,
    ) {
        let cache = self
            .entries
            .entry(source_id)
            .or_insert_with(|| WavEntriesState::new(total, page_size));
        cache.total = total;
        cache.page_size = page_size;
        cache.insert_page(page_index, entries);
    }
}

pub(crate) struct LibraryCacheState {
    pub(crate) db: HashMap<SourceId, Rc<SourceDatabase>>,
    pub(crate) wav: WavCacheState,
}

impl LibraryCacheState {
    pub(crate) fn new() -> Self {
        Self {
            db: HashMap::new(),
            wav: WavCacheState::new(),
        }
    }

    /// Resolve or open the database for `source`, caching the handle.
    pub(crate) fn database_for(
        &mut self,
        source: &SampleSource,
    ) -> Result<Rc<SourceDatabase>, SourceDbError> {
        if let Some(existing) = self.db.get(&source.id) {
            return Ok(existing.clone());
        }
        let db = Rc::new(SourceDatabase::open_for_background_job(&source.root)?);
        self.db.insert(source.id.clone(), db.clone());
        Ok(db)
    }
}
