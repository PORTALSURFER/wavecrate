use radiant::prelude as ui;

#[cfg(test)]
use crate::native_app::app::NativeAppState;
use crate::native_app::app::{GuiMessage, MetadataMessage, SampleNameViewMode};
use crate::native_app::app_chrome::view_models::sample_browser::SampleBrowserViewModel;
#[cfg(test)]
use crate::native_app::app_chrome::view_models::sample_browser::SampleBrowserViewProjection;
use crate::native_app::sample_library::folder_browser::commands::FolderBrowserMessage;
use crate::native_app::sample_library::folder_browser::model::{FileColumn, FileColumnKind};
use crate::native_app::sample_library::folder_browser::projection::FileColumnDragFeedback;
use crate::native_app::ui::ids as widget_ids;
use wavecrate::sample_sources::config::SimilarityAspectSettings;
use wavecrate_analysis::aspects::SimilarityAspect;

mod hit_target;
#[cfg(test)]
pub(in crate::native_app) use hit_target::{
    SampleFileHitTargetModel, sample_file_hit_target_for_tests,
};
mod row_projection;
mod row_widgets;
mod rows;
use rows::sample_browser_rows;

const SAMPLE_HEADER_SORT_DRAG_SCOPE: u64 = widget_ids::SAMPLE_HEADER_SORT_DRAG_ID;
const SAMPLE_HEADER_RESIZE_SCOPE: u64 = widget_ids::SAMPLE_HEADER_RESIZE_ID;
const SAMPLE_SIMILARITY_ASPECT_TOGGLE_SCOPE: u64 =
    widget_ids::SAMPLE_SIMILARITY_ASPECT_TOGGLE_SCOPE;
const SAMPLE_SIMILARITY_ASPECT_WEIGHT_SCOPE: u64 =
    widget_ids::SAMPLE_SIMILARITY_ASPECT_WEIGHT_SCOPE;
const SAMPLE_SIMILARITY_TOGGLE_HEADER_WIDTH: f32 = 22.0;
pub(super) const SAMPLE_SIMILARITY_SCORE_COLUMN_WIDTH: f32 = 190.0;
const SAMPLE_SIMILARITY_ASPECT_HEADER_WIDTH: f32 = 14.0;
const SAMPLE_BROWSER_ICON_ACTIVE_COLOR: ui::Rgba8 = ui::Rgba8::new(255, 160, 82, 255);
const SAMPLE_BROWSER_ICON_ENABLED_COLOR: ui::Rgba8 = ui::Rgba8::new(238, 238, 238, 255);
const SAMPLE_BROWSER_ICON_TINTS: ui::SvgIconTintPalette = ui::SvgIconTintPalette::new(
    SAMPLE_BROWSER_ICON_ENABLED_COLOR,
    SAMPLE_BROWSER_ICON_ACTIVE_COLOR,
    SAMPLE_BROWSER_ICON_ENABLED_COLOR,
);

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
            | ui::PointerShieldMessage::PointerPress { .. } => {
                Some(GuiMessage::CancelBrowserDragOnSampleList)
            }
            ui::PointerShieldMessage::Wheel { .. } => None,
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

struct SampleBrowserHeaderBar<'a> {
    columns: &'a [&'a FileColumn],
    sort: &'a ui::DetailsSort,
    drag_feedback: Option<&'a FileColumnDragFeedback>,
    mode: SampleNameViewMode,
    random_navigation_enabled: bool,
    similarity_mode_active: bool,
    similarity_controls: &'a SimilarityAspectSettings,
    help_tooltips_enabled: bool,
}

fn sample_browser_header_bar(model: SampleBrowserHeaderBar<'_>) -> ui::View<GuiMessage> {
    ui::row([
        sample_browser_header(
            model.columns,
            model.sort,
            model.drag_feedback,
            model.similarity_mode_active,
            model.similarity_controls,
        )
        .fill_width(),
        random_navigation_button(model.random_navigation_enabled).tooltip_opt(
            model
                .help_tooltips_enabled
                .then_some("Random audition within the selected folder or active filter."),
        ),
        sample_name_view_mode_button(model.mode).tooltip_opt(
            model
                .help_tooltips_enabled
                .then_some("Switch sample names between disk filenames and metadata labels."),
        ),
    ])
    .fill_width()
    .height(24.0)
    .spacing(6.0)
}

fn random_navigation_button(active: bool) -> ui::View<GuiMessage> {
    ui::icon_button(random_navigation_icon(active))
        .active(active)
        .message(GuiMessage::ToggleRandomNavigationMode)
        .id(widget_ids::SAMPLE_RANDOM_NAVIGATION_TOGGLE_ID)
        .key("sample-random-navigation-toggle")
        .size(28.0, 22.0)
}

fn random_navigation_icon(active: bool) -> ui::SvgIcon {
    DICE_ICON.icon_for_state(SAMPLE_BROWSER_ICON_TINTS, true, active)
}

fn sample_name_view_mode_button(mode: SampleNameViewMode) -> ui::View<GuiMessage> {
    let label = match mode {
        SampleNameViewMode::DiskFilename => "Disk",
        SampleNameViewMode::MetadataLabel => "Label",
    };
    ui::button(label)
        .message(GuiMessage::Metadata(
            MetadataMessage::ToggleSampleNameViewMode,
        ))
        .key("sample-name-view-mode-toggle")
        .size(58.0, 22.0)
}

static DICE_ICON: ui::SvgIconTintCache = ui::SvgIconTintCache::new(
    r#"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <rect x="2.75" y="2.75" width="10.5" height="10.5" rx="2" fill="none" stroke="currentColor" stroke-width="1.5"/>
  <circle cx="5.4" cy="5.4" r="1.15" fill="currentColor"/>
  <circle cx="10.6" cy="5.4" r="1.15" fill="currentColor"/>
  <circle cx="8" cy="8" r="1.15" fill="currentColor"/>
  <circle cx="5.4" cy="10.6" r="1.15" fill="currentColor"/>
  <circle cx="10.6" cy="10.6" r="1.15" fill="currentColor"/>
</svg>"#,
);

fn sample_browser_header(
    columns: &[&FileColumn],
    sort: &ui::DetailsSort,
    drag_feedback: Option<&FileColumnDragFeedback>,
    similarity_mode_active: bool,
    similarity_controls: &SimilarityAspectSettings,
) -> ui::View<GuiMessage> {
    let header_cells = columns.iter().flat_map(|column| {
        sample_header_cells(column, sort, similarity_mode_active, similarity_controls)
    });
    let header = ui::row([
        ui::spacer()
            .width(SAMPLE_SIMILARITY_TOGGLE_HEADER_WIDTH)
            .height(24.0),
        ui::compact_details_header_row(header_cells).fill_width(),
    ])
    .spacing(0.0)
    .fill_width()
    .height(24.0);
    let Some(feedback) = drag_feedback else {
        return header;
    };
    ui::stack([header, column_drop_marker(feedback.marker_x)])
        .fill_width()
        .height(24.0)
}

fn column_drop_marker(x: f32) -> ui::View<GuiMessage> {
    ui::local_drop_marker(x, ui::Rgba8::new(255, 160, 82, 230), 2.0, 20.0)
        .key("sample-column-drop-marker")
        .fill_width()
        .height(24.0)
        .padding_x(8.0)
        .padding_y(2.0)
}

fn sample_header_cell(column: &FileColumn, sort: &ui::DetailsSort) -> ui::View<GuiMessage> {
    let sort_id = column.id.clone();
    let drag_id = column.id.clone();
    let resize_id = column.id.clone();
    let label = ui::details_sort_label(column.label.as_str(), column.id.as_str(), Some(sort));
    ui::compact_resizable_details_header_cell_with_ids(
        format!("sample-header-{}", column.id),
        label,
        column.width,
        ui::CompactDetailsHeaderCellIds::new(
            Some(ui::stable_widget_id(
                SAMPLE_HEADER_SORT_DRAG_SCOPE,
                column.id.as_str(),
            )),
            Some(ui::stable_widget_id(
                SAMPLE_HEADER_RESIZE_SCOPE,
                column.id.as_str(),
            )),
        ),
        GuiMessage::FolderBrowser(FolderBrowserMessage::SortFileColumn(sort_id)),
        move |drag| {
            GuiMessage::FolderBrowser(FolderBrowserMessage::DragFileColumn(drag_id.clone(), drag))
        },
        move |message| {
            GuiMessage::FolderBrowser(FolderBrowserMessage::ResizeFileColumn(
                resize_id.clone(),
                message,
            ))
        },
    )
}

fn sample_header_cells(
    column: &FileColumn,
    sort: &ui::DetailsSort,
    similarity_mode_active: bool,
    similarity_controls: &SimilarityAspectSettings,
) -> Vec<ui::View<GuiMessage>> {
    let mut cells = vec![sample_header_cell(column, sort)];
    if column.kind() == FileColumnKind::Name && similarity_mode_active {
        cells.push(sample_similarity_header_cell(
            similarity_controls.aspect_enabled_flags(),
        ));
    }
    cells
}

fn sample_similarity_header_cell(
    aspect_enabled: [bool; wavecrate_analysis::aspects::ASPECT_COUNT],
) -> ui::View<GuiMessage> {
    let mut header_parts = Vec::with_capacity(wavecrate_analysis::aspects::ASPECT_COUNT + 1);
    for aspect in SimilarityAspect::ORDER {
        let label = similarity_aspect_short_label(aspect);
        let text = ui::text(label)
            .align_text(ui::TextAlign::Center)
            .key(format!("sample-header-similarity-aspect-{label}"))
            .height(20.0)
            .width(SAMPLE_SIMILARITY_ASPECT_HEADER_WIDTH);
        header_parts.push(if aspect_enabled[aspect.index()] {
            text
        } else {
            text.muted_text()
        });
    }
    header_parts.push(
        ui::text("Sim")
            .muted_text()
            .key("sample-header-similarity-label")
            .height(20.0)
            .fill_width(),
    );
    ui::compact_details_cell(
        ui::row(header_parts).spacing(3.0).height(20.0).fill_width(),
        Some(SAMPLE_SIMILARITY_SCORE_COLUMN_WIDTH),
    )
    .key("sample-header-similarity")
}

fn sample_similarity_controls_bar(controls: &SimilarityAspectSettings) -> ui::View<GuiMessage> {
    let mut controls_row = Vec::with_capacity(SimilarityAspect::ORDER.len() + 2);
    controls_row.push(
        ui::spacer()
            .width(SAMPLE_SIMILARITY_TOGGLE_HEADER_WIDTH)
            .height(22.0),
    );
    controls_row.push(
        ui::toggle("Weight", controls.weighting_enabled)
            .subtle()
            .message(GuiMessage::SetSimilarityAspectWeightingEnabled)
            .id(widget_ids::SAMPLE_SIMILARITY_WEIGHTING_TOGGLE_ID)
            .key("sample-similarity-weighting-toggle")
            .size(70.0, 20.0),
    );
    for aspect in SimilarityAspect::ORDER {
        controls_row.push(sample_similarity_aspect_control(aspect, controls));
    }
    ui::row(controls_row)
        .spacing(5.0)
        .padding_x(3.0)
        .fill_width()
        .height(28.0)
}

fn sample_similarity_aspect_control(
    aspect: SimilarityAspect,
    controls: &SimilarityAspectSettings,
) -> ui::View<GuiMessage> {
    let control = controls.control(aspect);
    let label = similarity_aspect_short_label(aspect);
    let aspect_key = similarity_aspect_key(aspect);
    ui::row([
        ui::color_marker(Some(similarity_aspect_color(aspect)))
            .side(7)
            .inset(0)
            .view()
            .width(8.0)
            .height(20.0),
        ui::toggle(label, control.enabled)
            .subtle()
            .message(move |enabled| GuiMessage::SetSimilarityAspectEnabled { aspect, enabled })
            .id(ui::stable_widget_id(
                SAMPLE_SIMILARITY_ASPECT_TOGGLE_SCOPE,
                aspect_key,
            ))
            .key(format!("sample-similarity-aspect-toggle-{aspect_key}"))
            .size(34.0, 20.0),
        ui::slider(control.weight)
            .compact()
            .subtle()
            .message(move |weight| GuiMessage::SetSimilarityAspectWeight { aspect, weight })
            .id(ui::stable_widget_id(
                SAMPLE_SIMILARITY_ASPECT_WEIGHT_SCOPE,
                aspect_key,
            ))
            .key(format!("sample-similarity-aspect-weight-{aspect_key}"))
            .size(62.0, 16.0),
    ])
    .spacing(3.0)
    .height(22.0)
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

fn similarity_aspect_short_label(aspect: SimilarityAspect) -> &'static str {
    match aspect {
        SimilarityAspect::Overall => "O",
        SimilarityAspect::Spectrum => "S",
        SimilarityAspect::Timbre => "T",
        SimilarityAspect::Pitch => "P",
        SimilarityAspect::Amplitude => "A",
    }
}

fn similarity_aspect_key(aspect: SimilarityAspect) -> &'static str {
    match aspect {
        SimilarityAspect::Overall => "overall",
        SimilarityAspect::Spectrum => "spectrum",
        SimilarityAspect::Timbre => "timbre",
        SimilarityAspect::Pitch => "pitch",
        SimilarityAspect::Amplitude => "amplitude",
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
