use crate::app_core::actions::{
    NativeBrowserTagPillModel as BrowserTagPillModel,
    NativeBrowserTagSidebarModel as BrowserTagSidebarModel,
    NativeBrowserTagState as BrowserTagState,
};
use crate::app_core::controller::AppController;
use crate::app_core::view_model;
use crate::sample_sources::WavEntry;

/// Project the browser tag sidebar from current target selection and metadata.
///
/// The retained projection cache keeps this surface on its own invalidation
/// contract because tag metadata edits and same-cardinality target swaps should
/// not depend on unrelated browser chrome churn.
pub(crate) fn project_browser_tag_sidebar_model(
    controller: &mut AppController,
) -> BrowserTagSidebarModel {
    let sidebar_targets = controller.browser_tag_sidebar_target_snapshot();
    let target_entries = sidebar_targets.resolve_entries(controller);
    let selected_count = target_entries.len();
    let header_label = match selected_count {
        0 => String::from("Select samples"),
        1 => target_entries[0]
            .relative_path
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_owned)
            .unwrap_or_else(|| view_model::sample_display_label(&target_entries[0].relative_path)),
        count => format!("{count} samples selected"),
    };
    let exclusive_pills = [
        pill_model(
            "playback-loop",
            "Loop",
            bool_tag_state(&target_entries, |entry| entry.looped),
        ),
        pill_model(
            "playback-one-shot",
            "One-shot",
            bool_tag_state(&target_entries, |entry| !entry.looped),
        ),
    ];
    let (accepted_pills, option_pills, create_pill) =
        if let Some(source) = controller.current_source() {
            let target_paths = sidebar_targets.resolve_paths(controller);
            let tag_sidebar_input = controller.ui.browser.tag_sidebar_input.clone();
            let metadata = controller.browser_tag_sidebar_metadata_snapshot(
                &source,
                &target_paths,
                &target_entries,
                &tag_sidebar_input,
            );
            let accepted_pills =
                project_accepted_normal_tags_from_label_sets(&metadata.accepted_label_sets);
            let option_pills = metadata
                .candidate_tags
                .into_iter()
                .map(|candidate| pill_model(&candidate.label, &candidate.label, candidate.state))
                .collect();
            let create_pill = metadata.create_label.map(|display_label| {
                pill_model(
                    &display_label,
                    &format!("Create \"{display_label}\""),
                    BrowserTagState::Off,
                )
            });
            (accepted_pills, option_pills, create_pill)
        } else {
            (Vec::new(), Vec::new(), None)
        };
    BrowserTagSidebarModel {
        // The tag editor is now rendered in the left library sidebar. Keep the
        // existing projection payload for mutation/input compatibility without
        // opening the old browser-row overlay.
        open: false,
        selected_count,
        header_label,
        primary_action_enabled: controller.ui.browser.tag_sidebar_auto_rename,
        input_value: controller.ui.browser.tag_sidebar_input.clone(),
        input_placeholder: String::from("Add tag"),
        input_focused: false,
        input_caret: controller.ui.browser.tag_sidebar_input.chars().count(),
        input_selection: None,
        exclusive_pills,
        accepted_pills,
        option_pills,
        create_pill,
    }
}

fn pill_model(id: &str, label: &str, state: BrowserTagState) -> BrowserTagPillModel {
    BrowserTagPillModel {
        id: id.to_string(),
        label: label.to_string(),
        state,
    }
}

fn bool_tag_state(entries: &[WavEntry], predicate: impl Fn(&WavEntry) -> bool) -> BrowserTagState {
    if entries.is_empty() {
        return BrowserTagState::Off;
    }
    let on_count = entries.iter().filter(|entry| predicate(entry)).count();
    match on_count {
        0 => BrowserTagState::Off,
        count if count == entries.len() => BrowserTagState::On,
        _ => BrowserTagState::Mixed,
    }
}

fn project_accepted_normal_tags_from_label_sets(
    label_sets: &[Vec<String>],
) -> Vec<BrowserTagPillModel> {
    if label_sets.is_empty() {
        return Vec::new();
    }
    let mut counts = std::collections::BTreeMap::<String, usize>::new();
    let mut order = Vec::<String>::new();
    for labels in label_sets {
        for label in labels {
            let normalized = display_tag_input(label);
            if normalized.is_empty() {
                continue;
            }
            let count = counts.entry(normalized.clone()).or_insert_with(|| {
                order.push(normalized.clone());
                0
            });
            *count += 1;
        }
    }
    order
        .into_iter()
        .filter_map(|label| {
            let count = counts.get(&label).copied().unwrap_or_default();
            (count > 0).then(|| {
                pill_model(
                    &label,
                    &label,
                    if count == label_sets.len() {
                        BrowserTagState::On
                    } else {
                        BrowserTagState::Mixed
                    },
                )
            })
        })
        .collect()
}

fn display_tag_input(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels_with_state(pills: &[BrowserTagPillModel]) -> Vec<(&str, BrowserTagState)> {
        pills
            .iter()
            .map(|pill| (pill.label.as_str(), pill.state))
            .collect()
    }

    #[test]
    fn accepted_normal_tags_preserve_first_seen_order_and_project_mixed_state() {
        let labels = vec![
            vec![
                String::from(" Deep  Kick "),
                String::from("Texture"),
                String::from(""),
            ],
            vec![String::from("Deep Kick"), String::from("Vinyl")],
        ];

        let pills = project_accepted_normal_tags_from_label_sets(&labels);

        assert_eq!(
            labels_with_state(&pills),
            vec![
                ("Deep Kick", BrowserTagState::On),
                ("Texture", BrowserTagState::Mixed),
                ("Vinyl", BrowserTagState::Mixed),
            ]
        );
    }
}
