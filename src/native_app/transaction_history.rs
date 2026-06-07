use std::collections::VecDeque;

use crate::native_app::app_scope::NativeAppState;
use crate::native_app::widget_ids;

pub(in crate::native_app) const TRANSACTION_LIST_MODAL_ID: u64 =
    widget_ids::TRANSACTION_LIST_MODAL_ID;

const DEFAULT_TRANSACTION_LIMIT: usize = 128;

pub(in crate::native_app) type TransactionResult = Result<(), String>;

pub(in crate::native_app) struct TransactionAction<T> {
    label: String,
    undo: Box<dyn Fn(&mut T) -> TransactionResult>,
    redo: Box<dyn Fn(&mut T) -> TransactionResult>,
}

impl<T> TransactionAction<T> {
    pub(in crate::native_app) fn new(
        label: impl Into<String>,
        undo: impl Fn(&mut T) -> TransactionResult + 'static,
        redo: impl Fn(&mut T) -> TransactionResult + 'static,
    ) -> Self {
        Self {
            label: label.into(),
            undo: Box::new(undo),
            redo: Box::new(redo),
        }
    }
}

pub(in crate::native_app) struct Transaction<T> {
    id: u64,
    label: String,
    actions: Vec<TransactionAction<T>>,
}

impl<T> Transaction<T> {
    fn new(id: u64, label: String, actions: Vec<TransactionAction<T>>) -> Self {
        Self { id, label, actions }
    }

    fn undo(&self, target: &mut T) -> TransactionResult {
        for action in self.actions.iter().rev() {
            (action.undo)(target)?;
        }
        Ok(())
    }

    fn redo(&self, target: &mut T) -> TransactionResult {
        for action in &self.actions {
            (action.redo)(target)?;
        }
        Ok(())
    }

    fn snapshot(&self, state: TransactionListState) -> TransactionListItem {
        TransactionListItem {
            id: self.id,
            label: self.label.clone(),
            action_count: self.actions.len(),
            action_labels: self
                .actions
                .iter()
                .map(|action| action.label.clone())
                .collect(),
            state,
        }
    }
}

struct TransactionDraft<T> {
    label: String,
    actions: Vec<TransactionAction<T>>,
    depth: usize,
}

impl<T> TransactionDraft<T> {
    fn new(label: String) -> Self {
        Self {
            label,
            actions: Vec::new(),
            depth: 1,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::native_app) enum TransactionListState {
    Active,
    Undoable,
    Redoable,
}

impl TransactionListState {
    pub(in crate::native_app) fn label(self) -> &'static str {
        match self {
            Self::Active => "Open",
            Self::Undoable => "Undo",
            Self::Redoable => "Redo",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct TransactionListItem {
    pub(in crate::native_app) id: u64,
    pub(in crate::native_app) label: String,
    pub(in crate::native_app) action_count: usize,
    pub(in crate::native_app) action_labels: Vec<String>,
    pub(in crate::native_app) state: TransactionListState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct TransactionApplied {
    pub(in crate::native_app) label: String,
    pub(in crate::native_app) action_count: usize,
}

pub(in crate::native_app) struct TransactionHistory<T> {
    undo: VecDeque<Transaction<T>>,
    redo: VecDeque<Transaction<T>>,
    active: Option<TransactionDraft<T>>,
    next_id: u64,
    limit: usize,
}

impl<T> Default for TransactionHistory<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> TransactionHistory<T> {
    pub(in crate::native_app) fn new() -> Self {
        Self {
            undo: VecDeque::new(),
            redo: VecDeque::new(),
            active: None,
            next_id: 1,
            limit: DEFAULT_TRANSACTION_LIMIT,
        }
    }

    pub(in crate::native_app) fn begin_transaction(&mut self, label: impl Into<String>) {
        if let Some(active) = self.active.as_mut() {
            active.depth += 1;
            return;
        }
        self.active = Some(TransactionDraft::new(label.into()));
    }

    pub(in crate::native_app) fn commit_transaction(&mut self) -> bool {
        let Some(active) = self.active.as_mut() else {
            return false;
        };
        active.depth = active.depth.saturating_sub(1);
        if active.depth > 0 {
            return false;
        }
        let Some(active) = self.active.take() else {
            return false;
        };
        if active.actions.is_empty() {
            return false;
        }
        self.push_transaction(active.label, active.actions);
        true
    }

    pub(in crate::native_app) fn register_action(
        &mut self,
        label: impl Into<String>,
        undo: impl Fn(&mut T) -> TransactionResult + 'static,
        redo: impl Fn(&mut T) -> TransactionResult + 'static,
    ) {
        let label = label.into();
        let action = TransactionAction::new(label.clone(), undo, redo);
        if let Some(active) = self.active.as_mut() {
            active.actions.push(action);
        } else {
            self.push_transaction(label, vec![action]);
        }
    }

    pub(in crate::native_app) fn push_transaction(
        &mut self,
        label: impl Into<String>,
        actions: Vec<TransactionAction<T>>,
    ) {
        if actions.is_empty() {
            return;
        }
        let transaction = Transaction::new(self.next_id, label.into(), actions);
        self.next_id += 1;
        self.redo.clear();
        self.undo.push_back(transaction);
        while self.undo.len() > self.limit {
            self.undo.pop_front();
        }
    }

    pub(in crate::native_app) fn undo(
        &mut self,
        target: &mut T,
    ) -> Result<Option<TransactionApplied>, String> {
        let Some(transaction) = self.undo.pop_back() else {
            return Ok(None);
        };
        if let Err(error) = transaction.undo(target) {
            self.undo.push_back(transaction);
            return Err(error);
        }
        let applied = TransactionApplied {
            label: transaction.label.clone(),
            action_count: transaction.actions.len(),
        };
        self.redo.push_back(transaction);
        Ok(Some(applied))
    }

    pub(in crate::native_app) fn redo(
        &mut self,
        target: &mut T,
    ) -> Result<Option<TransactionApplied>, String> {
        let Some(transaction) = self.redo.pop_back() else {
            return Ok(None);
        };
        if let Err(error) = transaction.redo(target) {
            self.redo.push_back(transaction);
            return Err(error);
        }
        let applied = TransactionApplied {
            label: transaction.label.clone(),
            action_count: transaction.actions.len(),
        };
        self.undo.push_back(transaction);
        Ok(Some(applied))
    }

    pub(in crate::native_app) fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub(in crate::native_app) fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub(in crate::native_app) fn is_transaction_open(&self) -> bool {
        self.active.is_some()
    }

    pub(in crate::native_app) fn list_items(&self) -> Vec<TransactionListItem> {
        let active = self.active.iter().map(|draft| TransactionListItem {
            id: 0,
            label: draft.label.clone(),
            action_count: draft.actions.len(),
            action_labels: draft
                .actions
                .iter()
                .map(|action| action.label.clone())
                .collect(),
            state: TransactionListState::Active,
        });
        let undo = self
            .undo
            .iter()
            .rev()
            .map(|transaction| transaction.snapshot(TransactionListState::Undoable));
        let redo = self
            .redo
            .iter()
            .rev()
            .map(|transaction| transaction.snapshot(TransactionListState::Redoable));
        active.chain(undo).chain(redo).collect()
    }
}

impl NativeAppState {
    pub(in crate::native_app) fn begin_transaction(&mut self, label: impl Into<String>) {
        if !self.transaction_restoring {
            self.transaction_history.begin_transaction(label);
        }
    }

    pub(in crate::native_app) fn commit_transaction(&mut self) -> bool {
        if self.transaction_restoring {
            return false;
        }
        self.transaction_history.commit_transaction()
    }

    pub(in crate::native_app) fn register_transaction_action(
        &mut self,
        label: impl Into<String>,
        undo: impl Fn(&mut NativeAppState) -> TransactionResult + 'static,
        redo: impl Fn(&mut NativeAppState) -> TransactionResult + 'static,
    ) {
        if self.transaction_restoring {
            return;
        }
        self.transaction_history.register_action(label, undo, redo);
    }

    pub(in crate::native_app) fn undo_transaction(&mut self) {
        let mut history = std::mem::take(&mut self.transaction_history);
        let was_restoring = self.transaction_restoring;
        self.transaction_restoring = true;
        let result = history.undo(self);
        self.transaction_restoring = was_restoring;
        self.transaction_history = history;
        match result {
            Ok(Some(applied)) => {
                self.sample_status = format!("Undid {}", applied.label);
            }
            Ok(None) => {
                self.sample_status = String::from("Nothing to undo");
            }
            Err(error) => {
                self.sample_status = format!("Undo failed: {error}");
            }
        }
    }

    pub(in crate::native_app) fn redo_transaction(&mut self) {
        let mut history = std::mem::take(&mut self.transaction_history);
        let was_restoring = self.transaction_restoring;
        self.transaction_restoring = true;
        let result = history.redo(self);
        self.transaction_restoring = was_restoring;
        self.transaction_history = history;
        match result {
            Ok(Some(applied)) => {
                self.sample_status = format!("Redid {}", applied.label);
            }
            Ok(None) => {
                self.sample_status = String::from("Nothing to redo");
            }
            Err(error) => {
                self.sample_status = format!("Redo failed: {error}");
            }
        }
    }

    pub(in crate::native_app) fn toggle_transaction_list(&mut self) {
        self.transaction_list_open = !self.transaction_list_open;
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
    fn standalone_action_undoes_and_redoes_one_transaction() {
        let mut history = TransactionHistory::new();
        let mut counter = Counter { value: 3 };
        history.register_action(
            "set to 3",
            |counter: &mut Counter| {
                counter.value = 0;
                Ok(())
            },
            |counter: &mut Counter| {
                counter.value = 3;
                Ok(())
            },
        );

        let undone = history.undo(&mut counter).expect("undo").expect("entry");
        assert_eq!(undone.label, "set to 3");
        assert_eq!(counter.value, 0);
        assert!(history.can_redo());

        history.redo(&mut counter).expect("redo");
        assert_eq!(counter.value, 3);
    }

    #[test]
    fn open_transaction_groups_multiple_actions() {
        let mut history = TransactionHistory::new();
        let mut counter = Counter { value: 2 };
        history.begin_transaction("compound");
        history.register_action(
            "first",
            |counter: &mut Counter| {
                counter.value -= 1;
                Ok(())
            },
            |counter: &mut Counter| {
                counter.value += 1;
                Ok(())
            },
        );
        history.register_action(
            "second",
            |counter: &mut Counter| {
                counter.value -= 10;
                Ok(())
            },
            |counter: &mut Counter| {
                counter.value += 10;
                Ok(())
            },
        );
        assert!(history.is_transaction_open());
        assert!(history.commit_transaction());

        counter.value = 13;
        let undone = history.undo(&mut counter).expect("undo").expect("entry");
        assert_eq!(undone.action_count, 2);
        assert_eq!(counter.value, 2);

        history.redo(&mut counter).expect("redo");
        assert_eq!(counter.value, 13);
    }

    #[test]
    fn registering_after_undo_clears_redo_branch() {
        let mut history = TransactionHistory::new();
        let mut counter = Counter { value: 1 };
        history.register_action(
            "one",
            |counter: &mut Counter| {
                counter.value = 0;
                Ok(())
            },
            |counter: &mut Counter| {
                counter.value = 1;
                Ok(())
            },
        );
        history.undo(&mut counter).expect("undo");
        assert!(history.can_redo());

        history.register_action(
            "branch",
            |counter: &mut Counter| {
                counter.value = 0;
                Ok(())
            },
            |counter: &mut Counter| {
                counter.value = 2;
                Ok(())
            },
        );

        assert!(!history.can_redo());
    }
}
