use std::collections::VecDeque;

use crate::native_app::transaction_history::{
    DEFAULT_TRANSACTION_LIMIT, TransactionApplied, TransactionResult,
};

struct TransactionAction<T> {
    undo: Box<dyn Fn(&mut T) -> TransactionResult>,
    redo: Box<dyn Fn(&mut T) -> TransactionResult>,
}

impl<T> TransactionAction<T> {
    fn new(
        undo: impl Fn(&mut T) -> TransactionResult + 'static,
        redo: impl Fn(&mut T) -> TransactionResult + 'static,
    ) -> Self {
        Self {
            undo: Box::new(undo),
            redo: Box::new(redo),
        }
    }
}

struct Transaction<T> {
    label: String,
    actions: Vec<TransactionAction<T>>,
}

impl<T> Transaction<T> {
    fn new(label: String, actions: Vec<TransactionAction<T>>) -> Self {
        Self { label, actions }
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

struct TransactionHistory<T> {
    undo: VecDeque<Transaction<T>>,
    redo: VecDeque<Transaction<T>>,
    active: Option<TransactionDraft<T>>,
    limit: usize,
}

impl<T> Default for TransactionHistory<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> TransactionHistory<T> {
    fn new() -> Self {
        Self {
            undo: VecDeque::new(),
            redo: VecDeque::new(),
            active: None,
            limit: DEFAULT_TRANSACTION_LIMIT,
        }
    }

    fn begin_transaction(&mut self, label: impl Into<String>) {
        if let Some(active) = self.active.as_mut() {
            active.depth += 1;
            return;
        }
        self.active = Some(TransactionDraft::new(label.into()));
    }

    fn commit_transaction(&mut self) -> bool {
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

    fn register_action(
        &mut self,
        label: impl Into<String>,
        undo: impl Fn(&mut T) -> TransactionResult + 'static,
        redo: impl Fn(&mut T) -> TransactionResult + 'static,
    ) {
        let label = label.into();
        let action = TransactionAction::new(undo, redo);
        if let Some(active) = self.active.as_mut() {
            active.actions.push(action);
        } else {
            self.push_transaction(label, vec![action]);
        }
    }

    fn push_transaction(&mut self, label: impl Into<String>, actions: Vec<TransactionAction<T>>) {
        if actions.is_empty() {
            return;
        }
        self.redo.clear();
        self.undo.push_back(Transaction::new(label.into(), actions));
        while self.undo.len() > self.limit {
            self.undo.pop_front();
        }
    }

    fn undo(&mut self, target: &mut T) -> Result<Option<TransactionApplied>, String> {
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

    fn redo(&mut self, target: &mut T) -> Result<Option<TransactionApplied>, String> {
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

    fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    fn is_transaction_open(&self) -> bool {
        self.active.is_some()
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
