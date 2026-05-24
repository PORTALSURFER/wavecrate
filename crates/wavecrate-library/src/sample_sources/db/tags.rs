//! Durable source tag catalog queries and assignment mutations.

mod identity;
mod mutations;
mod queries;

pub(in crate::sample_sources::db) use identity::normalize_tag_identity;

#[cfg(test)]
mod tests;
