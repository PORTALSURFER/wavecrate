use super::*;

impl AppController {
    /// Queue loop disable after the current cycle boundary to avoid mid-cycle discontinuities.
    pub(crate) fn defer_loop_disable_after_cycle(&mut self) -> Result<(), String> {
        player::defer_loop_disable_after_cycle(self)
    }

    /// Queue one loop restart at the current cycle boundary using a new start position.
    pub(crate) fn defer_loop_retarget_after_cycle(
        &mut self,
        start_override: f64,
    ) -> Result<bool, String> {
        player::defer_loop_retarget_after_cycle(self, start_override)
    }
}
