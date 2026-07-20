use super::{Arc, AtomicBool, BTreeMap, PendingSourceRetirement, SampleSource};

pub(super) fn sources_by_id(sources: Vec<SampleSource>) -> BTreeMap<String, SampleSource> {
    sources
        .into_iter()
        .map(|source| (source.id.as_str().to_string(), source))
        .collect()
}

pub(super) fn recovered_source_retirements(
    active_sources: &BTreeMap<String, SampleSource>,
    next_lifecycle_generation: &mut u64,
) -> Result<(BTreeMap<u64, PendingSourceRetirement>, u64), String> {
    let retained_sources = wavecrate::sample_sources::library::retained_sources()
        .map_err(|error| error.to_string())?;
    let mut pending = BTreeMap::new();
    let mut next_retirement_id = 1_u64;
    for retained in retained_sources.into_iter().filter(|retained| {
        !active_sources
            .values()
            .any(|active| source_storage_identity_matches(active, retained))
    }) {
        let lifecycle_generation = *next_lifecycle_generation;
        *next_lifecycle_generation = (*next_lifecycle_generation).wrapping_add(1).max(1);
        pending.insert(
            next_retirement_id,
            PendingSourceRetirement {
                source: retained,
                lifecycle_generation,
                cancel: Arc::new(AtomicBool::new(false)),
                retry_at: 0,
                attempts: 0,
                terminal_offline: false,
            },
        );
        next_retirement_id = next_retirement_id.wrapping_add(1).max(1);
    }
    Ok((pending, next_retirement_id))
}

pub(super) fn source_maps_match(
    current: &BTreeMap<String, SampleSource>,
    replacement: &BTreeMap<String, SampleSource>,
) -> bool {
    current.len() == replacement.len()
        && current.iter().all(|(source_id, source)| {
            replacement
                .get(source_id)
                .is_some_and(|other| source_descriptors_match(source, other))
        })
}

pub(super) fn source_descriptors_match(left: &SampleSource, right: &SampleSource) -> bool {
    left.id == right.id
        && left.root == right.root
        && left.role == right.role
        && left.metadata_storage == right.metadata_storage
        && left.primary_import_folder == right.primary_import_folder
}

pub(super) fn source_storage_identity_matches(left: &SampleSource, right: &SampleSource) -> bool {
    if left.id != right.id {
        return false;
    }
    match (left.database_root(), right.database_root()) {
        (Ok(left_root), Ok(right_root)) => left_root == right_root,
        _ => false,
    }
}
