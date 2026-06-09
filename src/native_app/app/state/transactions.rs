use crate::native_app::transaction_history::NativeTransactionHistory;

#[derive(Default)]
pub(in crate::native_app) struct TransactionState {
    pub(in crate::native_app) history: NativeTransactionHistory,
    pub(in crate::native_app) restoring: bool,
}
