use crate::native_app::app::NativeAppState;

impl NativeAppState {
    pub(in crate::native_app) fn next_folder_task_id(&mut self) -> u64 {
        self.background.next_task_id()
    }
}
