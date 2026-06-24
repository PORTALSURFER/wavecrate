use crate::native_app::{
    app::NativeAppState,
    metadata::{MetadataTagCategoryGroup, MetadataTagSelectionState},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct MetadataTagLibraryProjection {
    pub(super) categories: Vec<MetadataTagCategoryProjection>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct MetadataTagCategoryProjection {
    pub(super) id: &'static str,
    pub(super) header_label: String,
    pub(super) collapsed: bool,
    pub(super) accepts_drop: bool,
    pub(super) drop_hover: bool,
    pub(super) tags: Vec<MetadataTagProjection>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct MetadataTagProjection {
    pub(super) label: String,
    pub(super) category_id: &'static str,
    pub(super) selection_state: MetadataTagSelectionState,
    pub(super) draggable: bool,
    pub(super) drag_active: bool,
    pub(super) drag_source: bool,
    pub(super) drop_hover: bool,
}

impl MetadataTagLibraryProjection {
    pub(super) fn from_state(state: &NativeAppState) -> Self {
        Self::from_groups(
            state.categorized_metadata_tags(),
            state.metadata_tag_drag_active(),
            state.metadata_tag_drop_hover(),
            state.dragged_metadata_tag(),
            |tag| state.metadata_tag_selection_state(tag),
        )
    }

    fn from_groups(
        groups: Vec<MetadataTagCategoryGroup>,
        drag_active: bool,
        drop_hover: Option<&str>,
        dragged_tag: Option<&str>,
        mut selection_state: impl FnMut(&str) -> MetadataTagSelectionState,
    ) -> Self {
        let categories = groups
            .into_iter()
            .map(|group| {
                MetadataTagCategoryProjection::from_group(
                    group,
                    drag_active,
                    drop_hover,
                    dragged_tag,
                    &mut selection_state,
                )
            })
            .collect();
        Self { categories }
    }
}

impl MetadataTagCategoryProjection {
    fn from_group(
        group: MetadataTagCategoryGroup,
        drag_active: bool,
        drop_hover: Option<&str>,
        dragged_tag: Option<&str>,
        selection_state: &mut impl FnMut(&str) -> MetadataTagSelectionState,
    ) -> Self {
        let drop_hover = drop_hover == Some(group.id);
        let accepts_drop = drag_active && !group.locked;
        let tags = group
            .tags
            .into_iter()
            .map(|tag| MetadataTagProjection {
                selection_state: selection_state(&tag),
                drag_source: dragged_tag == Some(tag.as_str()),
                label: tag,
                category_id: group.id,
                draggable: !group.locked,
                drag_active,
                drop_hover,
            })
            .collect::<Vec<_>>();

        Self {
            id: group.id,
            header_label: category_header_label(group.label, tags.len(), group.locked),
            collapsed: group.collapsed,
            accepts_drop,
            drop_hover,
            tags,
        }
    }
}

fn category_header_label(label: &str, tag_count: usize, locked: bool) -> String {
    let count_label = if tag_count == 0 {
        String::new()
    } else {
        format!(" ({tag_count})")
    };
    format!(
        "{label}{count_label}{}",
        if locked { " [locked]" } else { "" }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_projection_carries_header_count_and_locked_state() {
        let projection = MetadataTagLibraryProjection::from_groups(
            vec![MetadataTagCategoryGroup {
                id: "playback-type",
                label: "Playback Type",
                tags: vec![String::from("loop"), String::from("one-shot")],
                collapsed: false,
                locked: true,
            }],
            true,
            Some("playback-type"),
            Some("loop"),
            |_| MetadataTagSelectionState::All,
        );

        assert_eq!(
            projection.categories[0].header_label,
            "Playback Type (2) [locked]"
        );
        assert!(!projection.categories[0].accepts_drop);
        assert!(projection.categories[0].drop_hover);
        assert!(!projection.categories[0].tags[0].draggable);
        assert!(projection.categories[0].tags[0].drag_source);
    }

    #[test]
    fn category_projection_marks_unlocked_drop_target_and_tag_selection() {
        let projection = MetadataTagLibraryProjection::from_groups(
            vec![MetadataTagCategoryGroup {
                id: "character",
                label: "Character",
                tags: vec![String::from("warm")],
                collapsed: false,
                locked: false,
            }],
            true,
            Some("character"),
            None,
            |tag| {
                if tag == "warm" {
                    MetadataTagSelectionState::Mixed
                } else {
                    MetadataTagSelectionState::None
                }
            },
        );

        let category = &projection.categories[0];
        assert_eq!(category.header_label, "Character (1)");
        assert!(category.accepts_drop);
        assert_eq!(
            category.tags[0].selection_state,
            MetadataTagSelectionState::Mixed
        );
        assert!(category.tags[0].draggable);
        assert!(category.tags[0].drop_hover);
    }

    #[test]
    fn empty_category_header_omits_count() {
        assert_eq!(category_header_label("Character", 0, false), "Character");
        assert_eq!(
            category_header_label("Playback Type", 0, true),
            "Playback Type [locked]"
        );
    }
}
