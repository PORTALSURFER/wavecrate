use super::{
    DELETE_STAGING_DIR, DeleteJournal, DeleteRecoveryReport, JournaledRecoveryOutcome,
    SampleSource, SourceId, journaled::recover_journaled_entry, load_journal, remove_entry,
    unjournaled,
};
use std::path::Path;

pub(super) fn recover_staged_deletes(sources: &[SampleSource]) -> DeleteRecoveryReport {
    let mut report = DeleteRecoveryReport::default();
    for source in sources {
        recover_source(source, &mut report);
    }
    report
}

fn recover_source(source: &SampleSource, report: &mut DeleteRecoveryReport) {
    if !source.root.is_dir() {
        return;
    }
    let staging_root = source.root.join(DELETE_STAGING_DIR);
    if !staging_root.is_dir() {
        return;
    }
    let journal = match load_journal(&staging_root) {
        Ok(journal) => journal,
        Err(err) => {
            report.errors.push(format!(
                "Failed to read delete journal for {}: {err}; leaving staged deletes untouched until the journal is repaired",
                source.root.display()
            ));
            return;
        }
    };
    let journaled_roots = super::journaled_staged_roots(&journal);
    recover_journaled_entries(source, &staging_root, &journal, report);
    unjournaled::recover_unjournaled_entries(source, &staging_root, &journaled_roots, report);
    super::super::cleanup_staging_root(&staging_root);
}

fn recover_journaled_entries(
    source: &SampleSource,
    staging_root: &Path,
    journal: &DeleteJournal,
    report: &mut DeleteRecoveryReport,
) {
    for entry in journal.entries.clone() {
        match recover_journaled_entry(source, staging_root, &entry) {
            Some(JournaledRecoveryOutcome::Completed(result)) => {
                if result.needs_hard_sync {
                    push_unique_scan_source(
                        &mut report.scan_sources,
                        &result.report_entry.source_id,
                    );
                }
                report.entries.push(result.report_entry);
                if result.remove_from_journal
                    && let Err(err) = remove_entry(staging_root, &entry.id)
                {
                    report
                        .errors
                        .push(format!("Failed to update delete journal: {err}"));
                }
            }
            Some(JournaledRecoveryOutcome::Retained(result)) => {
                report.retained_entries.push(result.retained_entry);
            }
            None => {}
        }
    }
}

fn push_unique_scan_source(scan_sources: &mut Vec<SourceId>, source_id: &SourceId) {
    if !scan_sources.iter().any(|existing| existing == source_id) {
        scan_sources.push(source_id.clone());
    }
}
