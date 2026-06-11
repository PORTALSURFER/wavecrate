use std::collections::VecDeque;

use crate::native_app::app::NativeAppState;
use crate::native_app::transaction_history::{
    DEFAULT_TRANSACTION_LIMIT, TransactionApplied, TransactionContext, TransactionListItem,
    TransactionListState, TransactionResult,
};

type NativeTransactionClosure = dyn for<'a> Fn(&mut TransactionContext<'a>) -> TransactionResult;

struct NativeTransactionAction {
    label: String,
    undo: Box<NativeTransactionClosure>,
    redo: Box<NativeTransactionClosure>,
}

impl NativeTransactionAction {
    fn new(
        label: impl Into<String>,
        undo: impl for<'a> Fn(&mut TransactionContext<'a>) -> TransactionResult + 'static,
        redo: impl for<'a> Fn(&mut TransactionContext<'a>) -> TransactionResult + 'static,
    ) -> Self {
        Self {
            label: label.into(),
            undo: Box::new(undo),
            redo: Box::new(redo),
        }
    }
}

struct NativeTransaction {
    id: u64,
    label: String,
    actions: Vec<NativeTransactionAction>,
}

impl NativeTransaction {
    fn new(id: u64, label: String, actions: Vec<NativeTransactionAction>) -> Self {
        Self { id, label, actions }
    }

    fn undo(&self, state: &mut NativeAppState) -> TransactionResult {
        let mut context = TransactionContext { state };
        for action in self.actions.iter().rev() {
            (action.undo)(&mut context)?;
        }
        Ok(())
    }

    fn redo(&self, state: &mut NativeAppState) -> TransactionResult {
        let mut context = TransactionContext { state };
        for action in &self.actions {
            (action.redo)(&mut context)?;
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

struct NativeTransactionDraft {
    label: String,
    actions: Vec<NativeTransactionAction>,
    depth: usize,
}

impl NativeTransactionDraft {
    fn new(label: String) -> Self {
        Self {
            label,
            actions: Vec::new(),
            depth: 1,
        }
    }
}

pub(in crate::native_app) struct NativeTransactionHistory {
    undo: VecDeque<NativeTransaction>,
    redo: VecDeque<NativeTransaction>,
    active: Option<NativeTransactionDraft>,
    next_id: u64,
    limit: usize,
}

impl Default for NativeTransactionHistory {
    fn default() -> Self {
        Self::new()
    }
}

impl NativeTransactionHistory {
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
        self.active = Some(NativeTransactionDraft::new(label.into()));
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
        undo: impl for<'a> Fn(&mut TransactionContext<'a>) -> TransactionResult + 'static,
        redo: impl for<'a> Fn(&mut TransactionContext<'a>) -> TransactionResult + 'static,
    ) {
        let label = label.into();
        let action = NativeTransactionAction::new(label.clone(), undo, redo);
        if let Some(active) = self.active.as_mut() {
            active.actions.push(action);
        } else {
            self.push_transaction(label, vec![action]);
        }
    }

    fn push_transaction(
        &mut self,
        label: impl Into<String>,
        actions: Vec<NativeTransactionAction>,
    ) {
        if actions.is_empty() {
            return;
        }
        let transaction = NativeTransaction::new(self.next_id, label.into(), actions);
        self.next_id += 1;
        self.redo.clear();
        self.undo.push_back(transaction);
        while self.undo.len() > self.limit {
            self.undo.pop_front();
        }
    }

    pub(in crate::native_app) fn undo(
        &mut self,
        state: &mut NativeAppState,
    ) -> Result<Option<TransactionApplied>, String> {
        let Some(transaction) = self.undo.pop_back() else {
            return Ok(None);
        };
        if let Err(error) = transaction.undo(state) {
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
        state: &mut NativeAppState,
    ) -> Result<Option<TransactionApplied>, String> {
        let Some(transaction) = self.redo.pop_back() else {
            return Ok(None);
        };
        if let Err(error) = transaction.redo(state) {
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
