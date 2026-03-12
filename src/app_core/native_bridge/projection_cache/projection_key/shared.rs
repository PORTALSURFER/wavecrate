use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Hash one configured path into a compact projection-key scalar.
pub(super) fn hash_path_for_projection_key(path: &std::path::Path) -> u64 {
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    hasher.finish()
}

/// Hash one projected string into a compact projection-key scalar.
pub(super) fn hash_string_for_projection_key(value: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}
