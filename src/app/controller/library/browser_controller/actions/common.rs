use super::super::helpers::TriageSampleContext;
use super::*;
use std::collections::HashSet;

impl BrowserController<'_> {
    pub(super) fn resolve_unique_browser_contexts(
        &mut self,
        rows: &[usize],
    ) -> (Vec<TriageSampleContext>, Option<String>) {
        let mut contexts = Vec::with_capacity(rows.len());
        let mut seen = HashSet::new();
        let mut last_error = None;
        for &row in rows {
            match self.resolve_browser_sample(row) {
                Ok(ctx) => {
                    if seen.insert(ctx.entry.relative_path.clone()) {
                        contexts.push(ctx);
                    }
                }
                Err(err) => last_error = Some(err),
            }
        }
        (contexts, last_error)
    }
}

pub(super) fn format_bpm_label(bpm: f32) -> String {
    let rounded = bpm.round();
    if (bpm - rounded).abs() < 0.01 {
        format!("{rounded:.0}")
    } else {
        format!("{bpm:.2}")
    }
}
