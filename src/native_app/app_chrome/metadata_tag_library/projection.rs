use crate::native_app::{
    app::NativeAppState,
    metadata::{
        MetadataTagCategoryGroup, MetadataTagSelectionState, metadata_tag_pill_selection_style,
        metadata_tag_pill_width,
    },
};
use radiant::prelude as ui;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct MetadataTagLibraryProjection {
    pub(super) categories: Vec<MetadataTagCategoryProjection>,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct MetadataTagCategoryProjection {
    pub(super) id: &'static str,
    pub(super) header_label: String,
    pub(super) expanded: bool,
    pub(super) drag_active: bool,
    pub(super) drop_candidate: bool,
    pub(super) drop_target: bool,
    pub(super) drop_target_active: bool,
    pub(super) body: MetadataTagCategoryBodyProjection,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum MetadataTagCategoryBodyProjection {
    Collapsed,
    Empty(MetadataTagEmptyCategoryProjection),
    Tags(MetadataTagPillGroupProjection),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct MetadataTagEmptyCategoryProjection {
    pub(super) category_id: &'static str,
    pub(super) drag_active: bool,
    pub(super) drop_candidate: bool,
    pub(super) drop_target: bool,
    pub(super) drop_target_active: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct MetadataTagPillGroupProjection {
    pub(super) category_id: &'static str,
    pub(super) tags: Vec<MetadataTagProjection>,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct MetadataTagProjection {
    pub(super) label: String,
    pub(super) category_id: &'static str,
    pub(super) selection_state: MetadataTagSelectionState,
    pub(super) style: ui::WidgetStyle,
    pub(super) width: f32,
    pub(super) active: bool,
    pub(super) draggable: bool,
    pub(super) drag_active: bool,
    pub(super) drag_source: bool,
    pub(super) drop_candidate: bool,
    pub(super) drop_target: bool,
    pub(super) drop_target_active: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MetadataTagCategoryProjectionContext<'a> {
    category_id: &'static str,
    locked: bool,
    drag_active: bool,
    drop_target: bool,
    drop_target_active: bool,
    dragged_tag: Option<&'a str>,
    dragged_category_id: Option<&'static str>,
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
        let dragged_category_id = dragged_tag.and_then(|dragged| {
            groups
                .iter()
                .find(|group| group.tags.iter().any(|tag| tag == dragged))
                .map(|group| group.id)
        });
        let categories = groups
            .into_iter()
            .map(|group| {
                MetadataTagCategoryProjection::from_group(
                    group,
                    drag_active,
                    drop_hover,
                    dragged_tag,
                    dragged_category_id,
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
        dragged_category_id: Option<&'static str>,
        selection_state: &mut impl FnMut(&str) -> MetadataTagSelectionState,
    ) -> Self {
        let MetadataTagCategoryGroup {
            id: category_id,
            label,
            tags,
            collapsed,
            locked,
        } = group;
        let context = MetadataTagCategoryProjectionContext {
            category_id,
            locked,
            drag_active,
            drop_target: drop_hover == Some(category_id),
            drop_target_active: drop_hover.is_some(),
            dragged_tag,
            dragged_category_id,
        };
        let tags = tags
            .into_iter()
            .map(|tag| {
                let selection_state = selection_state(&tag);
                context.project_tag(tag, selection_state)
            })
            .collect::<Vec<_>>();
        let tag_count = tags.len();
        let body = MetadataTagCategoryBodyProjection::from_category_state(
            category_id,
            collapsed,
            context.drag_active,
            context.drop_candidate(),
            context.drop_target,
            context.drop_target_active,
            tags,
        );

        Self {
            id: category_id,
            header_label: category_header_label(label, tag_count, locked),
            expanded: !collapsed,
            drag_active: context.drag_active,
            drop_candidate: context.drop_candidate(),
            drop_target: context.drop_target,
            drop_target_active: context.drop_target_active,
            body,
        }
    }
}

impl MetadataTagCategoryProjectionContext<'_> {
    fn drop_candidate(self) -> bool {
        self.drag_active
            && !self.locked
            && self
                .dragged_category_id
                .is_some_and(|dragged_category_id| dragged_category_id != self.category_id)
    }

    fn project_tag(
        self,
        tag: String,
        selection_state: MetadataTagSelectionState,
    ) -> MetadataTagProjection {
        MetadataTagProjection {
            style: metadata_tag_pill_selection_style(self.category_id, selection_state),
            width: metadata_tag_pill_width(&tag),
            active: selection_state.is_all(),
            selection_state,
            drag_source: self.dragged_tag == Some(tag.as_str()),
            label: tag,
            category_id: self.category_id,
            draggable: !self.locked,
            drag_active: self.drag_active,
            drop_candidate: self.drop_candidate(),
            drop_target: self.drop_target,
            drop_target_active: self.drop_target_active,
        }
    }
}

impl MetadataTagCategoryBodyProjection {
    fn from_category_state(
        category_id: &'static str,
        collapsed: bool,
        drag_active: bool,
        drop_candidate: bool,
        drop_target: bool,
        drop_target_active: bool,
        tags: Vec<MetadataTagProjection>,
    ) -> Self {
        if collapsed {
            return Self::Collapsed;
        }
        if tags.is_empty() {
            return Self::Empty(MetadataTagEmptyCategoryProjection {
                category_id,
                drag_active,
                drop_candidate,
                drop_target,
                drop_target_active,
            });
        }
        Self::Tags(MetadataTagPillGroupProjection { category_id, tags })
    }
}

impl MetadataTagCategoryProjection {
    pub(super) fn drop_tracking_active(&self) -> bool {
        self.drag_active && (self.drop_candidate || self.drop_target || self.drop_target_active)
    }
}

impl MetadataTagEmptyCategoryProjection {
    pub(super) fn drop_tracking_active(&self) -> bool {
        self.drag_active && (self.drop_candidate || self.drop_target || self.drop_target_active)
    }
}

impl MetadataTagProjection {
    pub(super) fn drop_tracking_active(&self) -> bool {
        self.drag_active && (self.drop_candidate || self.drop_target || self.drop_target_active)
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
#[path = "projection/tests.rs"]
mod tests;
