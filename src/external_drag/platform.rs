//! Windows outgoing file-drag platform implementation.

mod com_apartment;
mod data_object;
mod drag_session;
mod drop_source;
mod formats;
mod hglobal_payload;

pub(super) use drag_session::start_file_drag;
