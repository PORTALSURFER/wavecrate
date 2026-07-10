use super::*;
pub(super) fn starmap_empty_message(
    curation_mode_enabled: bool,
    status: StarmapStatus,
) -> &'static str {
    if status.listed_count > 0 {
        return "No Starmap positions yet";
    }
    if curation_mode_enabled {
        "No files left to curate"
    } else {
        "No audio files in selected folder"
    }
}

pub(super) fn starmap_search_overlay(name_filter: String) -> ui::View<GuiMessage> {
    ui::column([
        ui::row([
            ui::spacer().fill_width().height(26.0),
            ui::text_input(name_filter)
                .placeholder("Search")
                .clear_button(GuiMessage::FolderBrowser(
                    FolderBrowserMessage::NameFilterInput(TextInputMessage::Changed {
                        value: String::new(),
                    }),
                ))
                .id(widget_ids::SAMPLE_BROWSER_MAP_SEARCH_INPUT_ID)
                .message_event(|message| {
                    GuiMessage::FolderBrowser(FolderBrowserMessage::NameFilterInput(message))
                })
                .size(320.0, 24.0),
            ui::spacer().fill_width().height(26.0),
        ])
        .height(30.0)
        .padding_y(4.0)
        .fill_width(),
        ui::spacer().fill_height(),
    ])
    .fill()
}

pub(super) fn starmap_controls_overlay() -> ui::View<GuiMessage> {
    ui::column([
        ui::spacer().fill_width().height(36.0),
        ui::row([
            ui::spacer().fill_width().height(26.0),
            starmap_control_button(
                starmap_zoom_out_icon(),
                GuiMessage::ChangeStarmapViewport(StarmapViewportChange::Zoom {
                    anchor: MAP_CONTROL_ANCHOR,
                    factor: 1.0 / MAP_CONTROL_ZOOM_FACTOR,
                }),
            )
            .tooltip("Zoom out"),
            starmap_control_button(
                starmap_zoom_in_icon(),
                GuiMessage::ChangeStarmapViewport(StarmapViewportChange::Zoom {
                    anchor: MAP_CONTROL_ANCHOR,
                    factor: MAP_CONTROL_ZOOM_FACTOR,
                }),
            )
            .tooltip("Zoom in"),
            starmap_control_button(starmap_focus_icon(), GuiMessage::FocusSelectedStarmapNode)
                .tooltip("Focus selected sample"),
            starmap_control_button(
                starmap_reset_icon(),
                GuiMessage::ChangeStarmapViewport(StarmapViewportChange::Reset),
            )
            .tooltip("Reset map view"),
        ])
        .spacing(4.0)
        .padding(8.0)
        .height(40.0)
        .fill_width(),
        ui::spacer().fill_height(),
    ])
    .fill()
}

pub(super) fn starmap_legend_overlay(
    controls: &SimilarityAspectSettings,
    status: StarmapStatus,
) -> ui::View<GuiMessage> {
    let entries = if status.clustered_count > 0 {
        starmap_cluster_legend_entries(status.cluster_color_count)
    } else {
        SimilarityAspect::ORDER
            .into_iter()
            .filter(|aspect| controls.aspect_enabled(*aspect))
            .map(starmap_aspect_legend_entry)
            .collect::<Vec<_>>()
    };
    if entries.is_empty() {
        return ui::spacer().fill();
    }
    ui::column([
        ui::spacer().fill_height(),
        ui::row([
            ui::spacer().fill_width().height(24.0),
            ui::row(entries)
                .spacing(7.0)
                .padding(6.0)
                .height(24.0)
                .style(ui::WidgetStyle::subtle(ui::WidgetTone::Neutral)),
        ])
        .padding(8.0)
        .height(40.0)
        .fill_width(),
    ])
    .fill()
}

fn starmap_cluster_legend_entries(cluster_color_count: usize) -> Vec<ui::View<GuiMessage>> {
    let swatch_count = cluster_color_count.clamp(1, 6);
    std::iter::once(starmap_text_legend_entry("Similarity clusters", 120.0))
        .chain((0..swatch_count).map(starmap_cluster_legend_swatch))
        .collect()
}

fn starmap_cluster_legend_swatch(index: usize) -> ui::View<GuiMessage> {
    ui::color_marker(Some(starmap_cluster_palette_color(index)))
        .side(MAP_LEGEND_SWATCH_SIZE)
        .inset(0)
        .view()
        .width(f32::from(MAP_LEGEND_SWATCH_SIZE) + 1.0)
        .height(16.0)
}

fn starmap_aspect_legend_entry(aspect: SimilarityAspect) -> ui::View<GuiMessage> {
    ui::row([
        ui::color_marker(Some(similarity_aspect_color(aspect)))
            .side(MAP_LEGEND_SWATCH_SIZE)
            .inset(0)
            .view()
            .width(f32::from(MAP_LEGEND_SWATCH_SIZE) + 1.0)
            .height(16.0),
        ui::text(starmap_aspect_label(aspect))
            .muted_text()
            .height(16.0)
            .width(starmap_aspect_label_width(aspect)),
    ])
    .spacing(3.0)
    .height(16.0)
}

fn starmap_text_legend_entry(label: &'static str, width: f32) -> ui::View<GuiMessage> {
    ui::text(label).muted_text().height(16.0).width(width)
}

fn starmap_aspect_label(aspect: SimilarityAspect) -> &'static str {
    match aspect {
        SimilarityAspect::Overall => "Overall",
        SimilarityAspect::Spectrum => "Spectrum",
        SimilarityAspect::Timbre => "Timbre",
        SimilarityAspect::Pitch => "Pitch",
        SimilarityAspect::Amplitude => "Amp",
    }
}

fn starmap_aspect_label_width(aspect: SimilarityAspect) -> f32 {
    match aspect {
        SimilarityAspect::Overall => 54.0,
        SimilarityAspect::Spectrum => 62.0,
        SimilarityAspect::Timbre => 48.0,
        SimilarityAspect::Pitch => 34.0,
        SimilarityAspect::Amplitude => 28.0,
    }
}

fn starmap_control_button(icon: ui::SvgIcon, message: GuiMessage) -> ui::View<GuiMessage> {
    ui::icon_button(icon).message(message).size(24.0, 22.0)
}

fn starmap_zoom_in_icon() -> ui::SvgIcon {
    MAP_ZOOM_IN_ICON.icon_for_state(MAP_CONTROL_ICON_TINTS, true, false)
}

fn starmap_zoom_out_icon() -> ui::SvgIcon {
    MAP_ZOOM_OUT_ICON.icon_for_state(MAP_CONTROL_ICON_TINTS, true, false)
}

fn starmap_focus_icon() -> ui::SvgIcon {
    MAP_FOCUS_ICON.icon_for_state(MAP_CONTROL_ICON_TINTS, true, false)
}

fn starmap_reset_icon() -> ui::SvgIcon {
    MAP_RESET_ICON.icon_for_state(MAP_CONTROL_ICON_TINTS, true, false)
}

pub(super) fn starmap_status_overlay(
    status: StarmapStatus,
    prep_running: bool,
) -> ui::View<GuiMessage> {
    let Some(label) = status.label(prep_running) else {
        return ui::spacer().fill();
    };
    ui::column([
        ui::spacer().fill_height(),
        ui::row([
            ui::passive_badge(label)
                .style(ui::WidgetStyle::subtle(ui::WidgetTone::Warning))
                .height(20.0),
            ui::spacer().fill_width().height(20.0),
        ])
        .padding(8.0)
        .height(36.0)
        .fill_width(),
    ])
    .fill()
}
static MAP_ZOOM_IN_ICON: ui::SvgIconTintCache = ui::SvgIconTintCache::new(
    r#"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <circle cx="7" cy="7" r="4.4" fill="none" stroke="currentColor" stroke-width="1.5"/>
  <path d="M7 4.8v4.4M4.8 7h4.4M10.4 10.4l3.1 3.1" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
</svg>"#,
);

static MAP_ZOOM_OUT_ICON: ui::SvgIconTintCache = ui::SvgIconTintCache::new(
    r#"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <circle cx="7" cy="7" r="4.4" fill="none" stroke="currentColor" stroke-width="1.5"/>
  <path d="M4.8 7h4.4M10.4 10.4l3.1 3.1" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
</svg>"#,
);

static MAP_FOCUS_ICON: ui::SvgIconTintCache = ui::SvgIconTintCache::new(
    r#"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <path d="M8 2.4v2M8 11.6v2M2.4 8h2M11.6 8h2" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
  <circle cx="8" cy="8" r="2.7" fill="none" stroke="currentColor" stroke-width="1.5"/>
  <circle cx="8" cy="8" r="0.9" fill="currentColor"/>
</svg>"#,
);

static MAP_RESET_ICON: ui::SvgIconTintCache = ui::SvgIconTintCache::new(
    r#"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <path d="M4.3 5.2A4.7 4.7 0 1 1 3.8 10" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
  <path d="M4.3 2.6v2.6H1.7" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
</svg>"#,
);
