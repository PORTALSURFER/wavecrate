use crate::native_app::app::NativeAppState;
use crate::native_app::transaction_history::{TransactionContext, TransactionResult};

impl NativeAppState {
    pub(in crate::native_app) fn begin_transaction(&mut self, label: impl Into<String>) {
        if !self.transactions.restoring {
            self.transactions.history.begin_transaction(label);
        }
    }

    pub(in crate::native_app) fn commit_transaction(&mut self) -> bool {
        if self.transactions.restoring {
            return false;
        }
        self.transactions.history.commit_transaction()
    }

    pub(in crate::native_app) fn register_transaction_action(
        &mut self,
        label: impl Into<String>,
        undo: impl for<'a> Fn(&mut TransactionContext<'a>) -> TransactionResult + 'static,
        redo: impl for<'a> Fn(&mut TransactionContext<'a>) -> TransactionResult + 'static,
    ) {
        if self.transactions.restoring {
            return;
        }
        self.transactions.history.register_action(label, undo, redo);
    }

    pub(in crate::native_app) fn undo_transaction(&mut self) {
        let mut history = std::mem::take(&mut self.transactions.history);
        let was_restoring = self.transactions.restoring;
        self.transactions.restoring = true;
        let result = history.undo(self);
        self.transactions.restoring = was_restoring;
        self.transactions.history = history;
        match result {
            Ok(Some(applied)) => {
                self.ui.status.sample = format!("Undid {}", applied.label);
            }
            Ok(None) => {
                self.ui.status.sample = String::from("Nothing to undo");
            }
            Err(error) => {
                self.ui.status.sample = format!("Undo failed: {error}");
            }
        }
    }

    pub(in crate::native_app) fn redo_transaction(&mut self) {
        let mut history = std::mem::take(&mut self.transactions.history);
        let was_restoring = self.transactions.restoring;
        self.transactions.restoring = true;
        let result = history.redo(self);
        self.transactions.restoring = was_restoring;
        self.transactions.history = history;
        match result {
            Ok(Some(applied)) => {
                self.ui.status.sample = format!("Redid {}", applied.label);
            }
            Ok(None) => {
                self.ui.status.sample = String::from("Nothing to redo");
            }
            Err(error) => {
                self.ui.status.sample = format!("Redo failed: {error}");
            }
        }
    }

    pub(in crate::native_app) fn toggle_transaction_list(&mut self) {
        self.ui.chrome.transaction_list_open = !self.ui.chrome.transaction_list_open;
    }
}
