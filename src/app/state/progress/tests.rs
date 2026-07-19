use super::{ProgressOverlayState, ProgressTaskKind};

#[test]
fn progress_fraction_handles_zero_total() {
    let progress = ProgressOverlayState::new(ProgressTaskKind::TrashMove, "Task", 0, false);
    assert_eq!(progress.fraction(), 0.0);
}

#[test]
fn progress_reset_clears_visibility() {
    let mut progress = ProgressOverlayState::new(ProgressTaskKind::TrashMove, "Task", 2, true);
    progress.completed = 3;
    assert!(progress.fraction() <= 1.0);
    progress.reset();
    assert!(!progress.visible);
    assert_eq!(progress.task, None);
    assert_eq!(progress.completed, 0);
    assert_eq!(progress.total, 0);
}

#[test]
fn higher_priority_background_task_wins_footer_lane() {
    let mut progress = ProgressOverlayState::default();
    progress.show_task(
        ProgressTaskKind::Normalization,
        false,
        "Normalizing",
        10,
        true,
    );
    progress.show_task(
        ProgressTaskKind::WavLoad,
        false,
        "Loading samples",
        0,
        false,
    );

    assert_eq!(progress.task, Some(ProgressTaskKind::Normalization));
    assert_eq!(progress.title, "Normalizing");
}

#[test]
fn clearing_visible_task_reveals_next_contender() {
    let mut progress = ProgressOverlayState::default();
    progress.show_task(
        ProgressTaskKind::Normalization,
        false,
        "Normalizing",
        10,
        true,
    );
    progress.show_task(
        ProgressTaskKind::WavLoad,
        false,
        "Loading samples",
        0,
        false,
    );

    progress.clear_task(ProgressTaskKind::Normalization);

    assert_eq!(progress.task, Some(ProgressTaskKind::WavLoad));
    assert_eq!(progress.title, "Loading samples");
}
