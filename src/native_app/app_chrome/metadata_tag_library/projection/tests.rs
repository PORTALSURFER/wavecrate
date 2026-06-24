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
    assert!(!projection.categories[0].drop_candidate);
    assert!(projection.categories[0].drop_target);
    assert!(projection.categories[0].drop_target_active);
    let tags = projected_tags(&projection.categories[0]);
    assert!(!tags[0].draggable);
    assert!(tags[0].drag_source);
    assert!(tags[0].active);
    assert_eq!(tags[0].style.prominence, ui::WidgetProminence::Strong);
}

#[test]
fn category_projection_marks_unlocked_drop_target_and_tag_selection() {
    let projection = MetadataTagLibraryProjection::from_groups(
        vec![
            MetadataTagCategoryGroup {
                id: "sound-type",
                label: "Sound Type",
                tags: vec![String::from("bass")],
                collapsed: false,
                locked: false,
            },
            MetadataTagCategoryGroup {
                id: "character",
                label: "Character",
                tags: vec![String::from("warm")],
                collapsed: false,
                locked: false,
            },
        ],
        true,
        Some("character"),
        Some("bass"),
        |tag| {
            if tag == "warm" {
                MetadataTagSelectionState::Mixed
            } else {
                MetadataTagSelectionState::None
            }
        },
    );

    let category = category_by_id(&projection, "character");
    assert_eq!(category.header_label, "Character (1)");
    assert!(category.drop_candidate);
    assert!(category.drop_target);
    assert!(category.drop_target_active);
    let tags = projected_tags(category);
    assert_eq!(tags[0].selection_state, MetadataTagSelectionState::Mixed);
    assert_eq!(tags[0].style.prominence, ui::WidgetProminence::Normal);
    assert!(tags[0].width >= 38.0);
    assert!(!tags[0].active);
    assert!(tags[0].draggable);
    assert!(tags[0].drop_candidate);
    assert!(tags[0].drop_target);
    assert!(tags[0].drop_target_active);
}

#[test]
fn empty_unlocked_category_projects_empty_drop_body() {
    let projection = MetadataTagLibraryProjection::from_groups(
        vec![
            MetadataTagCategoryGroup {
                id: "sound-type",
                label: "Sound Type",
                tags: vec![String::from("bass")],
                collapsed: false,
                locked: false,
            },
            MetadataTagCategoryGroup {
                id: "character",
                label: "Character",
                tags: Vec::new(),
                collapsed: false,
                locked: false,
            },
        ],
        true,
        Some("character"),
        Some("bass"),
        |_| MetadataTagSelectionState::None,
    );

    let category = category_by_id(&projection, "character");
    assert_eq!(category.header_label, "Character");
    assert_eq!(
        category.body,
        MetadataTagCategoryBodyProjection::Empty(MetadataTagEmptyCategoryProjection {
            category_id: "character",
            drag_active: true,
            drop_candidate: true,
            drop_target: true,
            drop_target_active: true,
        })
    );
}

#[test]
fn dragged_tags_source_category_is_non_candidate_while_other_target_is_active() {
    let projection = MetadataTagLibraryProjection::from_groups(
        vec![
            MetadataTagCategoryGroup {
                id: "sound-type",
                label: "Sound Type",
                tags: vec![String::from("bass")],
                collapsed: false,
                locked: false,
            },
            MetadataTagCategoryGroup {
                id: "character",
                label: "Character",
                tags: vec![String::from("warm")],
                collapsed: false,
                locked: false,
            },
        ],
        true,
        Some("character"),
        Some("bass"),
        |_| MetadataTagSelectionState::None,
    );

    let source = category_by_id(&projection, "sound-type");
    assert!(source.drag_active);
    assert!(!source.drop_candidate);
    assert!(!source.drop_target);
    assert!(source.drop_target_active);
    assert!(source.drop_tracking_active());
    let tags = projected_tags(source);
    assert!(tags[0].drag_source);
    assert!(!tags[0].drop_candidate);
    assert!(!tags[0].drop_target);
    assert!(tags[0].drop_target_active);
}

#[test]
fn collapsed_category_keeps_header_count_but_hides_body() {
    let projection = MetadataTagLibraryProjection::from_groups(
        vec![MetadataTagCategoryGroup {
            id: "character",
            label: "Character",
            tags: vec![String::from("warm")],
            collapsed: true,
            locked: false,
        }],
        true,
        None,
        None,
        |_| MetadataTagSelectionState::None,
    );

    assert_eq!(projection.categories[0].header_label, "Character (1)");
    assert!(matches!(
        projection.categories[0].body,
        MetadataTagCategoryBodyProjection::Collapsed
    ));
}

#[test]
fn empty_category_header_omits_count() {
    assert_eq!(category_header_label("Character", 0, false), "Character");
    assert_eq!(
        category_header_label("Playback Type", 0, true),
        "Playback Type [locked]"
    );
}

fn projected_tags(category: &MetadataTagCategoryProjection) -> &[MetadataTagProjection] {
    match &category.body {
        MetadataTagCategoryBodyProjection::Tags(group) => group.tags.as_slice(),
        _ => panic!("expected projected tag group"),
    }
}

fn category_by_id<'a>(
    projection: &'a MetadataTagLibraryProjection,
    id: &str,
) -> &'a MetadataTagCategoryProjection {
    projection
        .categories
        .iter()
        .find(|category| category.id == id)
        .expect("category should be projected")
}
