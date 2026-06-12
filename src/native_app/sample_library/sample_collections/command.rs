use std::path::PathBuf;

use wavecrate::sample_sources::SampleCollection;

use crate::native_app::sample_library::folder_browser::view_contract::SelectedFileCollectionCandidate;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct CollectionUpdate {
    pub(super) root: PathBuf,
    pub(super) relative_path: PathBuf,
    pub(super) absolute_path: PathBuf,
    pub(super) collection: SampleCollection,
    pub(super) operation: CollectionOperation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CollectionOperation {
    Add,
    Remove,
}

impl CollectionOperation {
    fn inverted(self) -> Self {
        match self {
            Self::Add => Self::Remove,
            Self::Remove => Self::Add,
        }
    }
}

impl CollectionUpdate {
    pub(super) fn inverted(mut self) -> Self {
        self.operation = self.operation.inverted();
        self
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct CollectionUpdateCounts {
    pub(super) added: usize,
    pub(super) removed: usize,
}

impl CollectionUpdateCounts {
    pub(super) fn changed(self) -> bool {
        self.added > 0 || self.removed > 0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct CollectionSourcePath {
    pub(super) root: PathBuf,
    pub(super) relative_path: PathBuf,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CollectionCommand {
    Add,
    Remove,
    Toggle,
}

impl CollectionCommand {
    pub(super) fn action_name(self) -> &'static str {
        match self {
            Self::Add => "browser.collection.assign",
            Self::Remove => "browser.collection.remove",
            Self::Toggle => "browser.collection.toggle",
        }
    }
}

pub(super) fn plan_collection_update(
    candidate: SelectedFileCollectionCandidate,
    source_path: CollectionSourcePath,
    collection: SampleCollection,
    command: CollectionCommand,
) -> Option<CollectionUpdate> {
    let operation = match command {
        CollectionCommand::Add => {
            if candidate.assigned {
                return None;
            }
            CollectionOperation::Add
        }
        CollectionCommand::Remove => {
            if !candidate.assigned {
                return None;
            }
            CollectionOperation::Remove
        }
        CollectionCommand::Toggle => {
            if candidate.assigned {
                CollectionOperation::Remove
            } else {
                CollectionOperation::Add
            }
        }
    };
    Some(CollectionUpdate {
        root: source_path.root,
        relative_path: source_path.relative_path,
        absolute_path: candidate.path,
        collection,
        operation,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collection() -> SampleCollection {
        SampleCollection::new(0).expect("collection")
    }

    fn candidate(assigned: bool) -> SelectedFileCollectionCandidate {
        SelectedFileCollectionCandidate {
            path: PathBuf::from("C:/samples/kick.wav"),
            assigned,
        }
    }

    fn source_path() -> CollectionSourcePath {
        CollectionSourcePath {
            root: PathBuf::from("C:/samples"),
            relative_path: PathBuf::from("kick.wav"),
        }
    }

    #[test]
    fn add_command_skips_already_assigned_candidate() {
        assert_eq!(
            plan_collection_update(
                candidate(true),
                source_path(),
                collection(),
                CollectionCommand::Add
            ),
            None
        );
    }

    #[test]
    fn remove_command_skips_unassigned_candidate() {
        assert_eq!(
            plan_collection_update(
                candidate(false),
                source_path(),
                collection(),
                CollectionCommand::Remove
            ),
            None
        );
    }

    #[test]
    fn toggle_command_chooses_inverse_membership_operation() {
        let add = plan_collection_update(
            candidate(false),
            source_path(),
            collection(),
            CollectionCommand::Toggle,
        )
        .expect("add update");
        let remove = plan_collection_update(
            candidate(true),
            source_path(),
            collection(),
            CollectionCommand::Toggle,
        )
        .expect("remove update");

        assert_eq!(add.operation, CollectionOperation::Add);
        assert_eq!(remove.operation, CollectionOperation::Remove);
    }
}
