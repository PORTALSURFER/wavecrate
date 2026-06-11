mod app_state;
mod context;
#[cfg(test)]
mod generic;
mod native;
mod summary;

pub(in crate::native_app) use context::TransactionContext;
pub(in crate::native_app) use native::NativeTransactionHistory;
pub(in crate::native_app) use summary::{
    TransactionApplied, TransactionListItem, TransactionListState,
};

use crate::native_app::ui::ids as widget_ids;

pub(in crate::native_app) const TRANSACTION_LIST_MODAL_ID: u64 =
    widget_ids::TRANSACTION_LIST_MODAL_ID;

const DEFAULT_TRANSACTION_LIMIT: usize = 128;

pub(in crate::native_app) type TransactionResult = Result<(), String>;
