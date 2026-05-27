use wavecrate::updater::UpdaterRunArgs;

/// Run the updater without launching the removed native-shell helper UI.
pub fn run_gui(args: UpdaterRunArgs) -> Result<(), String> {
    crate::run_headless(args)
}
