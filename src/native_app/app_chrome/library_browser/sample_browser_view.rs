use radiant::prelude as ui;

use crate::native_app::app::GuiMessage;
#[cfg(test)]
use crate::native_app::app::NativeAppState;
use crate::native_app::app_chrome::view_models::sample_browser::SampleBrowserViewModel;
#[cfg(test)]
use crate::native_app::app_chrome::view_models::sample_browser::SampleBrowserViewProjection;
use crate::native_app::sample_library::folder_browser::commands::FolderBrowserMessage;
use wavecrate_analysis::aspects::SimilarityAspect;

mod header;
mod hit_target;
#[cfg(test)]
pub(in crate::native_app) use hit_target::{
    SampleFileHitTargetModel, sample_file_hit_target_for_tests,
};
mod cells;
mod identity;
mod row_projection;
mod row_widgets;
mod rows;
use header::{SampleBrowserHeaderBar, sample_browser_header_bar, sample_similarity_controls_bar};
use rows::sample_browser_rows;

pub(super) const SAMPLE_SIMILARITY_SCORE_COLUMN_WIDTH: f32 = 190.0;

#[cfg(test)]
pub(in crate::native_app) fn sample_browser_from_state(
    state: &NativeAppState,
) -> ui::View<GuiMessage> {
    sample_browser(SampleBrowserViewModel::from_projection(
        SampleBrowserViewProjection::from_prepared_app_state(state),
    ))
}

pub(in crate::native_app) fn sample_browser(
    model: SampleBrowserViewModel<'_>,
) -> ui::View<GuiMessage> {
    let mut sections = Vec::with_capacity(4);
    sections.push(sample_browser_header_bar(SampleBrowserHeaderBar {
        columns: model.visible_samples.columns.as_slice(),
        sort: model.visible_samples.sort,
        drag_feedback: model.drag_feedback.as_ref(),
        mode: model.name_view_mode,
        random_navigation_enabled: model.random_navigation_enabled,
        similarity_mode_active: model.visible_samples.similarity_mode_active,
        similarity_controls: model.visible_samples.similarity_controls,
        help_tooltips_enabled: model.help_tooltips_enabled,
    }));
    if model.visible_samples.similarity_mode_active {
        sections.push(sample_similarity_controls_bar(
            model.visible_samples.similarity_controls,
        ));
    }
    sections.push(
        sample_browser_rows(
            &model.visible_samples,
            model.name_view_mode,
            model.metadata_tags_by_file,
            model.cut_file_ids,
            model.help_tooltips_enabled,
        )
        .fill(),
    );
    sections.push(sample_browser_status(
        model.visible_samples.total_count,
        model.visible_samples.includes_subfolders,
    ));
    ui::column(sections)
        .spacing(0.0)
        .style(ui::WidgetStyle::default())
        .fill()
        .pointer_target_if(
            model.file_drag_active,
            sample_list_browser_drag_cancel_target,
        )
        .pointer_target_if(
            model.extracted_file_drag_active,
            sample_list_waveform_drop_target,
        )
        .pointer_target_if(
            model.hovered_folder_drop_target,
            sample_list_clear_folder_drop_target,
        )
}

fn sample_list_browser_drag_cancel_target() -> ui::PointerTarget<GuiMessage> {
    ui::pointer_target(true)
        .pointer_move(false)
        .pointer_press(false)
        .wheel(false)
        .filter_map(|message| match message {
            ui::PointerShieldMessage::PointerRelease { .. }
            | ui::PointerShieldMessage::PointerDrop { .. } => {
                Some(GuiMessage::CancelBrowserDragOnSampleList)
            }
            ui::PointerShieldMessage::PointerMove { .. }
            | ui::PointerShieldMessage::PointerPress { .. }
            | ui::PointerShieldMessage::Wheel { .. } => None,
        })
        .key("sample-list-browser-drag-cancel-target")
}

fn sample_list_waveform_drop_target() -> ui::PointerTarget<GuiMessage> {
    ui::pointer_drop_target(true)
        .on_drop(GuiMessage::DropWaveformSelectionOnSampleList)
        .key("sample-list-waveform-drop-target")
}

fn sample_list_clear_folder_drop_target() -> ui::PointerTarget<GuiMessage> {
    ui::pointer_move_target(true)
        .on_pointer_move(|position| {
            GuiMessage::FolderBrowser(FolderBrowserMessage::ClearDropTarget(position))
        })
        .key("sample-list-clear-folder-drop-target")
}

pub(super) fn similarity_aspect_color(aspect: SimilarityAspect) -> ui::Rgba8 {
    match aspect {
        SimilarityAspect::Overall => ui::Rgba8::new(105, 172, 116, 230),
        SimilarityAspect::Spectrum => ui::Rgba8::new(233, 211, 98, 235),
        SimilarityAspect::Timbre => ui::Rgba8::new(235, 149, 73, 235),
        SimilarityAspect::Pitch => ui::Rgba8::new(226, 82, 111, 235),
        SimilarityAspect::Amplitude => ui::Rgba8::new(93, 158, 221, 235),
    }
}

fn sample_browser_status(audio_count: usize, includes_subfolders: bool) -> ui::View<GuiMessage> {
    let scope = if includes_subfolders {
        "selected folder + subfolders"
    } else {
        "selected folder"
    };
    ui::row([
        ui::text("Listed").height(20.0).width(90.0),
        ui::text(format!(
            "{audio_count} audio file{} in {scope}",
            if audio_count == 1 { "" } else { "s" }
        ))
        .height(20.0)
        .fill_width(),
    ])
    .padding_x(3.0)
    .fill_width()
    .height(28.0)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use radiant::{
        layout::Vector2,
        prelude::IntoView,
        runtime::{DeclarativeOwnedRuntimeBridge, SurfaceRuntime},
        widgets::WidgetInput,
    };
    use wavecrate::sample_sources::config::SimilarityAspectSettings;

    use super::*;
    use crate::native_app::app::SampleNameViewMode;
    use crate::native_app::sample_library::folder_browser::commands::FolderBrowserMessage;
    use crate::native_app::sample_library::folder_browser::projection::VisibleSampleList;

    #[test]
    fn sample_list_stacked_pointer_targets_route_each_event_to_matching_domain_action() {
        let metadata_tags_by_file = HashMap::<String, Vec<String>>::new();
        let sort = ui::DetailsSort::new("name", ui::SortDirection::Ascending);
        let similarity_controls = SimilarityAspectSettings::default();
        let columns = Vec::new();
        let position = ui::Point::new(12.0, 12.0);

        let bridge = DeclarativeOwnedRuntimeBridge::new(
            Vec::<GuiMessage>::new(),
            |_| {
                sample_browser(SampleBrowserViewModel {
                    visible_samples: VisibleSampleList {
                        total_count: 0,
                        includes_subfolders: false,
                        window: ui::VirtualListWindow::default(),
                        rows: Vec::new(),
                        columns: columns.clone(),
                        sort: &sort,
                        similarity_mode_active: false,
                        similarity_controls: &similarity_controls,
                    },
                    name_view_mode: SampleNameViewMode::DiskFilename,
                    random_navigation_enabled: false,
                    metadata_tags_by_file: &metadata_tags_by_file,
                    cut_file_ids: None,
                    file_drag_active: true,
                    extracted_file_drag_active: true,
                    hovered_folder_drop_target: true,
                    drag_feedback: None,
                    help_tooltips_enabled: false,
                })
                .into_surface()
            },
            |messages, message| messages.push(message),
        );
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(240.0, 160.0));

        runtime.dispatch_pointer_move_with_outcome(position);
        runtime.dispatch_input_at(position, WidgetInput::primary_release(position));
        runtime.dispatch_input_at(position, WidgetInput::primary_drop(position));

        assert_eq!(
            runtime.bridge().state(),
            &[
                GuiMessage::FolderBrowser(FolderBrowserMessage::ClearDropTarget(position)),
                GuiMessage::CancelBrowserDragOnSampleList,
                GuiMessage::DropWaveformSelectionOnSampleList,
            ]
        );
    }
}
