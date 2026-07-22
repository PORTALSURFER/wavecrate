use crate::native_app::app::{GuiMessage, NativeAppState};
use crate::native_app::app_chrome::center_panel::{
    library_sidebar_overlays, library_sidebar_region, metadata_tag_library_region,
    metadata_tag_library_region_visible, sample_workspace_overlays, sample_workspace_region,
};
use crate::native_app::app_chrome::settings::top_control_bar;
use crate::native_app::app_chrome::status_bar::bottom_status_area;
use radiant::prelude as ui;

pub(in crate::native_app) fn shell(state: &NativeAppState) -> ui::View<GuiMessage> {
    let mut shell = ui::workspace_shell(sample_workspace_region(state))
        .top_bar(top_control_bar(state))
        .leading_sidebar(library_sidebar_region(state).overlays(library_sidebar_overlays(state)))
        .status_bar(bottom_status_area(state))
        .overlays(sample_workspace_overlays(state))
        .outer_spacing(0.0)
        .workspace_spacing(0.0);

    if metadata_tag_library_region_visible(state) {
        shell = shell.leading_sidebar(metadata_tag_library_region(state));
    }

    shell.build()
}
