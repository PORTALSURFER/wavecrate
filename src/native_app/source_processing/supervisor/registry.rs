use super::{Arc, AtomicBool, Ordering, SampleSource, Shared, source_descriptors_match};

pub(super) fn register_source_for_scan_locked(
    shared: &Shared,
    source: SampleSource,
) -> Result<u64, String> {
    let source_id = source.id.as_str().to_string();
    let mut control = shared.control();
    if control.shutdown || shared.cancel.load(Ordering::Acquire) {
        return Err("Source processing supervisor is shutting down".to_string());
    }
    if let Some(current) = control.sources.get(&source_id) {
        if !source_descriptors_match(current, &source) {
            return Err(format!(
                "Source {source_id} is already registered with a different descriptor"
            ));
        }
        let lifecycle_generation = control.source_lifecycle_generations[&source_id];
        if control.quarantined_sources.remove(&source_id) {
            control
                .source_work_cancels
                .insert(source_id.clone(), Arc::new(AtomicBool::new(false)));
            control.mark_source_dirty(&source_id, "source_scan_registration_reactivated");
            drop(control);
            shared.budget_wake.notify_all();
            shared.wake.notify_one();
        }
        return Ok(lifecycle_generation);
    }

    control.sources.insert(source_id.clone(), source);
    control
        .source_work_cancels
        .insert(source_id.clone(), Arc::new(AtomicBool::new(false)));
    let lifecycle_generation = control.allocate_lifecycle_generation();
    control
        .source_lifecycle_generations
        .insert(source_id.clone(), lifecycle_generation);
    control
        .force_manifest_audit_sources
        .insert(source_id.clone());
    control.mark_source_dirty(&source_id, "source_registered_for_scan");
    drop(control);
    shared.budget_wake.notify_all();
    shared.wake.notify_one();
    Ok(lifecycle_generation)
}

pub(super) fn resolve_registered_source_for_scan_locked(
    shared: &Shared,
    source: &SampleSource,
) -> Result<u64, String> {
    let source_id = source.id.as_str();
    let control = shared.control();
    if control.shutdown || shared.cancel.load(Ordering::Acquire) {
        return Err("Source processing supervisor is shutting down".to_string());
    }
    let Some(authoritative) = control.sources.get(source_id) else {
        return Err(format!(
            "Source {source_id} is no longer present in the configured source set"
        ));
    };
    if !source_descriptors_match(authoritative, source) {
        return Err(format!(
            "Source {source_id} is registered with a different descriptor"
        ));
    }
    if !control.source_is_active(source_id) {
        return Err(format!("Source {source_id} is not active"));
    }
    control
        .source_lifecycle_generations
        .get(source_id)
        .copied()
        .ok_or_else(|| format!("Source {source_id} has no active lifecycle generation"))
}
