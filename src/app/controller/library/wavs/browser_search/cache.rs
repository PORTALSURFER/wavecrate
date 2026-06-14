//! Browser search cache facade.

use super::*;

mod label_cache;
mod query_score_cache;
mod scoring;

pub(crate) use query_score_cache::BrowserSearchCache;
