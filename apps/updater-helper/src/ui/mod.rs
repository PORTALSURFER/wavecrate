use wavecrate::updater::UpdaterRunArgs;

/// Run the updater without launching the removed ui-projection helper UI.
pub fn run_gui(args: UpdaterRunArgs) -> Result<(), String> {
    crate::run_headless(args)
}
