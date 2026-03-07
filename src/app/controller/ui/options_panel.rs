use super::*;

impl AppController {
    /// Open the native-shell options panel.
    pub fn open_options_panel(&mut self) {
        self.ui.options_panel.open = true;
    }

    /// Close the native-shell options panel.
    pub fn close_options_panel(&mut self) {
        self.ui.options_panel.open = false;
    }

    /// Toggle the native-shell options panel visibility.
    pub fn toggle_options_panel(&mut self) {
        self.ui.options_panel.open = !self.ui.options_panel.open;
    }
}
