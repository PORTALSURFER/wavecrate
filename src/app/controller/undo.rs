use super::jobs::UndoFileJob;
use super::*;
use std::{
    collections::VecDeque,
    fs,
    path::{Path, PathBuf},
};
use uuid::Uuid;

/// Execution result for an undo/redo action.
pub(crate) enum UndoExecution {
    /// Action completed immediately.
    Applied,
    /// Action should be completed asynchronously by running the provided job.
    Deferred(UndoFileJob),
}

/// Result returned by undo/redo closures.
pub(crate) type UndoResult = Result<UndoExecution, String>;

/// Direction for a deferred undo/redo action.
#[derive(Clone, Copy, Debug)]
pub(crate) enum UndoDirection {
    /// Undo action direction.
    Undo,
    /// Redo action direction.
    Redo,
}

/// Deferred undo/redo action that will complete asynchronously.
pub(crate) struct DeferredUndo<T> {
    /// Pending undo/redo entry.
    pub(crate) entry: UndoEntry<T>,
    /// Direction to apply when the job completes.
    pub(crate) direction: UndoDirection,
    /// Job request to run on a background worker.
    pub(crate) job: UndoFileJob,
}

/// Outcome of an undo/redo request.
pub(crate) enum UndoOutcome<T> {
    /// No undo/redo entry was available.
    Empty,
    /// Action applied immediately with this label.
    Applied(String),
    /// Action deferred until background completion.
    Deferred(Box<DeferredUndo<T>>),
}

pub(crate) struct UndoEntry<T> {
    pub(crate) label: String,
    pub(crate) undo: Box<dyn Fn(&mut T) -> UndoResult>,
    pub(crate) redo: Box<dyn Fn(&mut T) -> UndoResult>,
    _cleanup: Vec<UndoCleanup>,
}

impl<T> UndoEntry<T> {
    pub(crate) fn new(
        label: impl Into<String>,
        undo: impl Fn(&mut T) -> UndoResult + 'static,
        redo: impl Fn(&mut T) -> UndoResult + 'static,
    ) -> Self {
        Self {
            label: label.into(),
            undo: Box::new(undo),
            redo: Box::new(redo),
            _cleanup: Vec::new(),
        }
    }

    pub(crate) fn with_cleanup_dir(mut self, path: PathBuf) -> Self {
        self._cleanup.push(UndoCleanup::dir(path));
        self
    }
}

pub(crate) struct UndoStack<T> {
    undo: VecDeque<UndoEntry<T>>,
    redo: VecDeque<UndoEntry<T>>,
    limit: usize,
}

impl<T> UndoStack<T> {
    pub(crate) fn new(limit: usize) -> Self {
        Self {
            undo: VecDeque::new(),
            redo: VecDeque::new(),
            limit: limit.max(1),
        }
    }

    pub(crate) fn push(&mut self, entry: UndoEntry<T>) {
        self.redo.clear();
        self.undo.push_back(entry);
        while self.undo.len() > self.limit {
            self.undo.pop_front();
        }
    }

    pub(crate) fn undo(&mut self, target: &mut T) -> Result<UndoOutcome<T>, String> {
        let Some(entry) = self.undo.pop_back() else {
            return Ok(UndoOutcome::Empty);
        };
        let label = entry.label.clone();
        match (entry.undo)(target) {
            Ok(UndoExecution::Applied) => {
                self.redo.push_back(entry);
                Ok(UndoOutcome::Applied(label))
            }
            Ok(UndoExecution::Deferred(job)) => Ok(UndoOutcome::Deferred(Box::new(DeferredUndo {
                entry,
                direction: UndoDirection::Undo,
                job,
            }))),
            Err(err) => {
                self.undo.push_back(entry);
                Err(err)
            }
        }
    }

    pub(crate) fn redo(&mut self, target: &mut T) -> Result<UndoOutcome<T>, String> {
        let Some(entry) = self.redo.pop_back() else {
            return Ok(UndoOutcome::Empty);
        };
        let label = entry.label.clone();
        match (entry.redo)(target) {
            Ok(UndoExecution::Applied) => {
                self.undo.push_back(entry);
                Ok(UndoOutcome::Applied(label))
            }
            Ok(UndoExecution::Deferred(job)) => Ok(UndoOutcome::Deferred(Box::new(DeferredUndo {
                entry,
                direction: UndoDirection::Redo,
                job,
            }))),
            Err(err) => {
                self.redo.push_back(entry);
                Err(err)
            }
        }
    }

    /// Restore a popped entry back onto the undo stack.
    pub(crate) fn restore_undo_entry(&mut self, entry: UndoEntry<T>) {
        self.undo.push_back(entry);
        while self.undo.len() > self.limit {
            self.undo.pop_front();
        }
    }

    /// Restore a popped entry back onto the redo stack.
    pub(crate) fn restore_redo_entry(&mut self, entry: UndoEntry<T>) {
        self.redo.push_back(entry);
    }

    /// Push a completed deferred entry onto the redo stack.
    pub(crate) fn push_redo_entry(&mut self, entry: UndoEntry<T>) {
        self.redo.push_back(entry);
    }
}

impl AppController {
    pub(crate) fn undo(&mut self) {
        if self.history.pending_undo.is_some() {
            self.set_status("Undo already in progress", StatusTone::Warning);
            return;
        }
        if self.runtime.jobs.file_ops_in_progress() {
            self.set_status("File operation already in progress", StatusTone::Warning);
            return;
        }
        let mut stack = std::mem::replace(&mut self.history.undo_stack, UndoStack::new(UNDO_LIMIT));
        let result = stack.undo(self);
        self.history.undo_stack = stack;
        match result {
            Ok(UndoOutcome::Applied(label)) => {
                self.set_status(format!("Undid {label}"), StatusTone::Info);
            }
            Ok(UndoOutcome::Empty) => self.set_status("Nothing to undo", StatusTone::Info),
            Ok(UndoOutcome::Deferred(pending)) => {
                self.begin_deferred_undo_job(*pending);
            }
            Err(err) => self.set_status(format!("Undo failed: {err}"), StatusTone::Error),
        }
    }

    pub(crate) fn redo(&mut self) {
        if self.history.pending_undo.is_some() {
            self.set_status("Redo already in progress", StatusTone::Warning);
            return;
        }
        if self.runtime.jobs.file_ops_in_progress() {
            self.set_status("File operation already in progress", StatusTone::Warning);
            return;
        }
        let mut stack = std::mem::replace(&mut self.history.undo_stack, UndoStack::new(UNDO_LIMIT));
        let result = stack.redo(self);
        self.history.undo_stack = stack;
        match result {
            Ok(UndoOutcome::Applied(label)) => {
                self.set_status(format!("Redid {label}"), StatusTone::Info);
            }
            Ok(UndoOutcome::Empty) => self.set_status("Nothing to redo", StatusTone::Info),
            Ok(UndoOutcome::Deferred(pending)) => {
                self.begin_deferred_undo_job(*pending);
            }
            Err(err) => self.set_status(format!("Redo failed: {err}"), StatusTone::Error),
        }
    }

    pub(crate) fn push_undo_entry(&mut self, entry: UndoEntry<AppController>) {
        self.history.undo_stack.push(entry);
    }

    pub(crate) fn begin_selection_undo(&mut self, label: impl Into<String>) {
        if self.selection_state.pending_undo.is_some() {
            return;
        }
        let before = self
            .selection_state
            .range
            .range()
            .or(self.ui.waveform.selection);
        self.selection_state.pending_undo = Some(SelectionUndoState {
            label: label.into(),
            before,
        });
    }

    pub(crate) fn commit_selection_undo(&mut self) {
        let Some(pending) = self.selection_state.pending_undo.take() else {
            return;
        };
        let after = self
            .selection_state
            .range
            .range()
            .or(self.ui.waveform.selection);
        self.push_selection_undo(pending.label, pending.before, after);
    }

    pub(crate) fn push_selection_undo(
        &mut self,
        label: impl Into<String>,
        before: Option<SelectionRange>,
        after: Option<SelectionRange>,
    ) {
        if before == after {
            return;
        }
        let label = label.into();
        self.push_undo_entry(UndoEntry::<AppController>::new(
            label,
            move |controller| {
                controller.selection_state.range.set_range(before);
                controller.apply_selection(before);
                Ok(UndoExecution::Applied)
            },
            move |controller| {
                controller.selection_state.range.set_range(after);
                controller.apply_selection(after);
                Ok(UndoExecution::Applied)
            },
        ));
    }
}

struct UndoCleanup {
    dir: Option<PathBuf>,
}

impl UndoCleanup {
    fn dir(dir: PathBuf) -> Self {
        Self { dir: Some(dir) }
    }
}

impl Drop for UndoCleanup {
    fn drop(&mut self) {
        let Some(dir) = self.dir.take() else {
            return;
        };
        let _ = fs::remove_dir_all(dir);
    }
}

pub(crate) struct OverwriteBackup {
    pub(crate) dir: PathBuf,
    pub(crate) before: PathBuf,
    pub(crate) after: PathBuf,
}

impl OverwriteBackup {
    pub(crate) fn capture_before(target: &Path) -> Result<Self, String> {
        let dir = std::env::temp_dir().join(format!("sempal_undo_{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).map_err(|err| format!("Failed to create undo folder: {err}"))?;
        let before = dir.join("before.wav");
        let after = dir.join("after.wav");
        fs::copy(target, &before).map_err(|err| format!("Failed to snapshot audio file: {err}"))?;
        Ok(Self { dir, before, after })
    }

    pub(crate) fn capture_after(&self, target: &Path) -> Result<(), String> {
        fs::copy(target, &self.after)
            .map_err(|err| format!("Failed to snapshot edited audio file: {err}"))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Counter {
        value: i32,
    }

    #[test]
    fn undo_stack_respects_limit() {
        let mut stack: UndoStack<Counter> = UndoStack::new(3);
        let mut counter = Counter::default();

        for i in 1..=4 {
            counter.value = i;
            let before = i - 1;
            stack.push(UndoEntry::new(
                format!("set {i}"),
                move |c: &mut Counter| {
                    c.value = before;
                    Ok(UndoExecution::Applied)
                },
                move |c: &mut Counter| {
                    c.value = i;
                    Ok(UndoExecution::Applied)
                },
            ));
        }

        assert_eq!(counter.value, 4);
        assert!(matches!(
            stack.undo(&mut counter).unwrap(),
            UndoOutcome::Applied(label) if label == "set 4"
        ));
        assert_eq!(counter.value, 3);
        assert!(matches!(
            stack.undo(&mut counter).unwrap(),
            UndoOutcome::Applied(label) if label == "set 3"
        ));
        assert_eq!(counter.value, 2);
        assert!(matches!(
            stack.undo(&mut counter).unwrap(),
            UndoOutcome::Applied(label) if label == "set 2"
        ));
        assert_eq!(counter.value, 1);
        assert!(matches!(
            stack.undo(&mut counter).unwrap(),
            UndoOutcome::Empty
        ));
        assert_eq!(counter.value, 1);
    }

    #[test]
    fn pushing_new_action_clears_redo_stack() {
        let mut stack: UndoStack<Counter> = UndoStack::new(10);
        let mut counter = Counter { value: 1 };
        stack.push(UndoEntry::new(
            "set 1",
            |c: &mut Counter| {
                c.value = 0;
                Ok(UndoExecution::Applied)
            },
            |c: &mut Counter| {
                c.value = 1;
                Ok(UndoExecution::Applied)
            },
        ));

        assert!(matches!(
            stack.redo(&mut counter).unwrap(),
            UndoOutcome::Empty
        ));

        stack.undo(&mut counter).unwrap();
        assert_eq!(counter.value, 0);

        counter.value = 2;
        stack.push(UndoEntry::new(
            "set 2",
            |c: &mut Counter| {
                c.value = 1;
                Ok(UndoExecution::Applied)
            },
            |c: &mut Counter| {
                c.value = 2;
                Ok(UndoExecution::Applied)
            },
        ));

        assert!(matches!(
            stack.redo(&mut counter).unwrap(),
            UndoOutcome::Empty
        ));
    }
}
