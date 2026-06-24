use radiant::prelude as ui;
use std::collections::HashMap;
use wavecrate::sample_sources::{Rating, SampleCollection};

use super::super::{FileColumnKind, FileEntry};
use super::ordering::{sort_file_refs_by_column_kind_for_tests, sort_kind_for_details_sort};

struct TestFileEntry<'a> {
    stem: &'a str,
    id_prefix: &'a str,
    extension: &'a str,
    size_bytes: u64,
    modified_rank: u64,
    kind: &'a str,
    rating: Rating,
    collection: Option<SampleCollection>,
}

impl TestFileEntry<'_> {
    fn build(self) -> FileEntry {
        let Self {
            stem,
            id_prefix,
            extension,
            size_bytes,
            modified_rank,
            kind,
            rating,
            collection,
        } = self;

        FileEntry {
            id: format!("{id_prefix}/{stem}.{extension}"),
            name: format!("{stem}.{extension}"),
            stem: stem.to_owned(),
            extension: extension.to_owned(),
            kind: kind.to_owned(),
            size: format!("{size_bytes} B"),
            size_bytes,
            modified: modified_rank.to_string(),
            modified_rank,
            rating,
            rating_locked: false,
            collection,
            collections: collection.into_iter().collect(),
        }
    }
}

fn sort_names_by(kind: FileColumnKind) -> Vec<String> {
    let low_collection = SampleCollection::new(0).expect("collection 0");
    let high_collection = SampleCollection::new(1).expect("collection 1");
    let files = [
        TestFileEntry {
            stem: "alpha",
            id_prefix: "C:/z",
            extension: "wav",
            size_bytes: 20,
            modified_rank: 3,
            kind: "Audio",
            rating: Rating::NEUTRAL,
            collection: None,
        }
        .build(),
        TestFileEntry {
            stem: "bravo",
            id_prefix: "C:/a",
            extension: "aif",
            size_bytes: 10,
            modified_rank: 2,
            kind: "Loop",
            rating: Rating::KEEP_1,
            collection: Some(high_collection),
        }
        .build(),
        TestFileEntry {
            stem: "charlie",
            id_prefix: "C:/m",
            extension: "mp3",
            size_bytes: 30,
            modified_rank: 1,
            kind: "Drum",
            rating: Rating::TRASH_1,
            collection: Some(low_collection),
        }
        .build(),
    ];
    let mut file_refs = files.iter().collect::<Vec<_>>();
    sort_file_refs_by_column_kind_for_tests(kind, &mut file_refs, None);
    file_refs
        .into_iter()
        .map(|file| file.stem.clone())
        .collect::<Vec<_>>()
}

#[test]
fn playback_type_sort_groups_by_tags_before_name() {
    let files = [
        TestFileEntry {
            stem: "alpha-shot",
            id_prefix: "C:/samples",
            extension: "wav",
            size_bytes: 20,
            modified_rank: 3,
            kind: "Audio",
            rating: Rating::NEUTRAL,
            collection: None,
        }
        .build(),
        TestFileEntry {
            stem: "bravo-loop",
            id_prefix: "C:/samples",
            extension: "wav",
            size_bytes: 10,
            modified_rank: 2,
            kind: "Audio",
            rating: Rating::NEUTRAL,
            collection: None,
        }
        .build(),
        TestFileEntry {
            stem: "charlie-shot",
            id_prefix: "C:/samples",
            extension: "wav",
            size_bytes: 30,
            modified_rank: 1,
            kind: "Audio",
            rating: Rating::NEUTRAL,
            collection: None,
        }
        .build(),
        TestFileEntry {
            stem: "delta-unknown",
            id_prefix: "C:/samples",
            extension: "wav",
            size_bytes: 40,
            modified_rank: 4,
            kind: "Audio",
            rating: Rating::NEUTRAL,
            collection: None,
        }
        .build(),
    ];
    let tags_by_file = HashMap::from([
        (files[0].id.clone(), vec![String::from("one-shot")]),
        (files[1].id.clone(), vec![String::from("loop")]),
        (files[2].id.clone(), vec![String::from("oneshot")]),
    ]);
    let mut file_refs = files.iter().collect::<Vec<_>>();

    sort_file_refs_by_column_kind_for_tests(
        FileColumnKind::PlaybackType,
        &mut file_refs,
        Some(&tags_by_file),
    );

    assert_eq!(
        file_refs
            .into_iter()
            .map(|file| file.stem.as_str())
            .collect::<Vec<_>>(),
        vec!["bravo-loop", "alpha-shot", "charlie-shot", "delta-unknown"]
    );
}

#[test]
fn typed_file_column_kinds_map_to_sort_behavior() {
    let cases = [
        (FileColumnKind::Name, vec!["alpha", "bravo", "charlie"]),
        (FileColumnKind::Extension, vec!["bravo", "charlie", "alpha"]),
        (FileColumnKind::Size, vec!["bravo", "alpha", "charlie"]),
        (FileColumnKind::Modified, vec!["charlie", "bravo", "alpha"]),
        (FileColumnKind::Kind, vec!["alpha", "charlie", "bravo"]),
        (FileColumnKind::Rating, vec!["charlie", "alpha", "bravo"]),
        (
            FileColumnKind::PlaybackType,
            vec!["alpha", "bravo", "charlie"],
        ),
        (
            FileColumnKind::Collection,
            vec!["alpha", "charlie", "bravo"],
        ),
        (
            FileColumnKind::SourceFolder,
            vec!["bravo", "charlie", "alpha"],
        ),
        (FileColumnKind::Path, vec!["bravo", "charlie", "alpha"]),
        (
            FileColumnKind::Similarity,
            vec!["alpha", "bravo", "charlie"],
        ),
    ];

    for (kind, expected) in cases {
        assert_eq!(sort_names_by(kind), expected, "{kind:?}");
    }
}

#[test]
fn unknown_sort_id_falls_back_to_name_kind() {
    assert_eq!(
        sort_kind_for_details_sort(&ui::DetailsSort::new(
            "missing-column",
            ui::SortDirection::Ascending,
        )),
        FileColumnKind::Name
    );
}
