use super::*;

#[path = "chrome/folders.rs"]
mod folders;
#[path = "chrome/prompts.rs"]
mod prompts;
#[path = "chrome/source_controls.rs"]
mod source_controls;
#[path = "chrome/top_bar.rs"]
mod top_bar;

pub(in crate::gui::native_shell::state) use self::source_controls::sidebar_filter_dropdown_spec;
