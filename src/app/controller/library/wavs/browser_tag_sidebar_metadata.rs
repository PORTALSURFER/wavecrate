use super::*;
use crate::app_core::actions::NativeBrowserTagState;
use crate::sample_sources::db::SourceTagUsage;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct BrowserTagSidebarMetadataSnapshot {
    pub(crate) accepted_label_sets: Vec<Vec<String>>,
    pub(crate) candidate_tags: Vec<BrowserTagSidebarCandidateTag>,
    pub(crate) create_label: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BrowserTagSidebarCandidateTag {
    pub(crate) label: String,
    pub(crate) state: NativeBrowserTagState,
}

impl AppController {
    pub(crate) fn browser_tag_sidebar_metadata_snapshot(
        &mut self,
        source: &SampleSource,
        paths: &[PathBuf],
        fallback_entries: &[WavEntry],
        input: &str,
    ) -> BrowserTagSidebarMetadataSnapshot {
        let accepted_label_sets = self
            .browser_tag_sidebar_accepted_label_sets(source, paths, fallback_entries)
            .unwrap_or_else(|_| normal_tag_label_sets_from_entries(fallback_entries));
        let (candidate_tags, create_label) = self
            .browser_tag_sidebar_candidate_tags(source, paths, input)
            .unwrap_or_default();
        BrowserTagSidebarMetadataSnapshot {
            accepted_label_sets,
            candidate_tags,
            create_label,
        }
    }

    fn browser_tag_sidebar_accepted_label_sets(
        &mut self,
        source: &SampleSource,
        paths: &[PathBuf],
        fallback_entries: &[WavEntry],
    ) -> Result<Vec<Vec<String>>, String> {
        if paths.is_empty() {
            return Ok(Vec::new());
        }
        let labels_by_path = {
            let db = self.database_for(source).map_err(|err| err.to_string())?;
            paths
                .iter()
                .map(|path| db.tag_labels_for_path(path).map_err(|err| err.to_string()))
                .collect::<Result<Vec<_>, _>>()?
        };
        if labels_by_path.iter().all(Vec::is_empty) && !fallback_entries.is_empty() {
            return Ok(normal_tag_label_sets_from_entries(fallback_entries));
        }
        Ok(labels_by_path)
    }

    fn browser_tag_sidebar_candidate_tags(
        &mut self,
        source: &SampleSource,
        paths: &[PathBuf],
        input: &str,
    ) -> Result<(Vec<BrowserTagSidebarCandidateTag>, Option<String>), String> {
        let normalized_input = normalize_tag_input(input);
        let usages = {
            let db = self.database_for(source).map_err(|err| err.to_string())?;
            if normalized_input.is_empty() {
                db.most_used_tags(18).map_err(|err| err.to_string())?
            } else {
                db.search_tags(input, 18).map_err(|err| err.to_string())?
            }
        };
        let mut candidates = Vec::new();
        for usage in usages.into_iter().filter(normal_tag_visible) {
            let label = usage.tag.display_label;
            let state = self.normal_tag_state_for_source(source, paths, &label)?;
            candidates.push(BrowserTagSidebarCandidateTag { label, state });
        }
        let create_label = if normalized_input.is_empty() || !candidates.is_empty() {
            None
        } else {
            Some(display_tag_input(input))
        };
        Ok((candidates, create_label))
    }
}

fn normal_tag_label_sets_from_entries(entries: &[WavEntry]) -> Vec<Vec<String>> {
    entries
        .iter()
        .map(|entry| entry.normal_tags.clone())
        .collect()
}

fn normal_tag_visible(usage: &SourceTagUsage) -> bool {
    !matches!(
        usage.tag.normalized_text.as_str(),
        "loop" | "looped" | "one-shot" | "one shot" | "oneshot"
    )
}

fn normalize_tag_input(input: &str) -> String {
    display_tag_input(input).to_ascii_lowercase()
}

fn display_tag_input(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_sidebar_display_normalization_collapses_whitespace() {
        assert_eq!(
            display_tag_input("  Deep\tKick \n Tight  "),
            "Deep Kick Tight"
        );
        assert_eq!(
            normalize_tag_input("  Deep\tKick \n Tight  "),
            "deep kick tight"
        );
        assert_eq!(display_tag_input(" \t\n "), "");
    }
}
