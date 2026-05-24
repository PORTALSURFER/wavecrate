use std::path::Path;

use tempfile::tempdir;

use super::super::SourceDatabase;

#[test]
fn normalization_prevents_case_and_spacing_duplicates() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    db.upsert_file(Path::new("one.wav"), 10, 5).unwrap();
    db.upsert_file(Path::new("two.wav"), 10, 5).unwrap();

    let first = db
        .assign_tag_to_path(Path::new("one.wav"), "  Deep   Kick ")
        .unwrap();
    let second = db
        .assign_tag_to_path(Path::new("two.wav"), "deep kick")
        .unwrap();

    assert_eq!(first.id, second.id);
    assert_eq!(first.display_label, "Deep Kick");
    assert_eq!(first.normalized_text, "deep kick");
    assert_eq!(db.search_tags("DEEP    KICK", 8).unwrap().len(), 1);
}

#[test]
fn most_used_tags_order_by_persisted_usage_then_label() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    for path in ["one.wav", "two.wav", "three.wav", "four.wav"] {
        db.upsert_file(Path::new(path), 10, 5).unwrap();
    }
    db.assign_tag_to_path(Path::new("one.wav"), "zeta").unwrap();
    db.assign_tag_to_path(Path::new("one.wav"), "alpha")
        .unwrap();
    db.assign_tag_to_path(Path::new("two.wav"), "alpha")
        .unwrap();
    db.assign_tag_to_path(Path::new("three.wav"), "beta")
        .unwrap();
    db.assign_tag_to_path(Path::new("four.wav"), "beta")
        .unwrap();

    let labels = db
        .most_used_tags(8)
        .unwrap()
        .into_iter()
        .map(|usage| (usage.tag.display_label, usage.assignment_count))
        .collect::<Vec<_>>();

    assert_eq!(
        labels,
        vec![
            ("alpha".to_string(), 2),
            ("beta".to_string(), 2),
            ("zeta".to_string(), 1),
        ]
    );
}

#[test]
fn assignment_api_creates_then_resolves_existing_tag() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    db.upsert_file(Path::new("one.wav"), 10, 5).unwrap();
    db.upsert_file(Path::new("two.wav"), 10, 5).unwrap();

    let created = db
        .assign_tag_to_path(Path::new("one.wav"), "Texture")
        .unwrap();
    let resolved = db
        .assign_tag_to_path(Path::new("two.wav"), " texture ")
        .unwrap();

    assert_eq!(created.id, resolved.id);
    assert_eq!(
        db.tags_for_path(Path::new("two.wav")).unwrap(),
        vec![created]
    );
    let removed = db
        .remove_tag_from_path(Path::new("two.wav"), "TEXTURE")
        .unwrap();
    assert!(removed);
    assert!(db.tags_for_path(Path::new("two.wav")).unwrap().is_empty());
}

#[test]
fn search_tags_orders_exact_match_before_usage_matches() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    for path in ["one.wav", "two.wav", "three.wav"] {
        db.upsert_file(Path::new(path), 10, 5).unwrap();
    }
    db.assign_tag_to_path(Path::new("one.wav"), "deep kick")
        .unwrap();
    db.assign_tag_to_path(Path::new("two.wav"), "kick").unwrap();
    db.assign_tag_to_path(Path::new("three.wav"), "deep kick")
        .unwrap();

    let labels = db
        .search_tags("kick", 8)
        .unwrap()
        .into_iter()
        .map(|usage| usage.tag.display_label)
        .collect::<Vec<_>>();

    assert_eq!(labels, vec!["kick".to_string(), "deep kick".to_string()]);
}

#[test]
fn search_tags_finds_less_common_fuzzy_matches() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    for path in ["one.wav", "two.wav", "three.wav"] {
        db.upsert_file(Path::new(path), 10, 5).unwrap();
    }
    db.assign_tag_to_path(Path::new("one.wav"), "Texture")
        .unwrap();
    db.assign_tag_to_path(Path::new("two.wav"), "Texture")
        .unwrap();
    db.assign_tag_to_path(Path::new("three.wav"), "Rare FX")
        .unwrap();

    let labels = db
        .search_tags("rfx", 8)
        .unwrap()
        .into_iter()
        .map(|usage| usage.tag.display_label)
        .collect::<Vec<_>>();

    assert_eq!(labels, vec!["Rare FX".to_string()]);
}
