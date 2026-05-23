use crate::{
    app_core::actions::{
        NativeAppModel, NativeBrowserRowModel, NativeBrowserRowProcessingState,
        NativeNormalizedRangeModel, NativeSourceRowModel, NativeUpdateStatusModel,
        native_folder_row_model,
    },
    gui::types::ImageRgba,
};
use std::sync::Arc;

pub(super) fn startup_scene_model() -> NativeAppModel {
    let mut model = NativeAppModel::default();
    model.title = String::from("Wavecrate Native");
    model.backend_label = String::from("radiant/native_vello");
    model.transport_running = true;
    model.status.left = String::from("Ready");
    model.status.center = String::from("rows: 18 | selected: 1 | anchor: - | search: -");
    model.status.right = String::from("col: 2/3");
    model.status_text = String::from("Startup scene");
    model.sources.header = String::from("Sources");
    model.sources.search_query = String::from("kick");
    model.sources.tree_search_query = String::from("drum");
    model.sources.selected_row = Some(0);
    model.sources.focused_tree_row = Some(0);
    model.sources.tree_actions.can_create_child = true;
    model.sources.tree_actions.can_create_root = true;
    model.sources.tree_actions.can_rename = true;
    model.sources.tree_actions.can_delete = true;
    model.sources.tree_actions.can_clear_history = true;
    model.sources.recovery.in_progress = false;
    model.sources.recovery.entry_count = 3;
    for index in 0..8 {
        model.sources.rows.push(NativeSourceRowModel::new(
            format!("source_{index:02}"),
            format!("/samples/source_{index:02}"),
            index == 1,
            false,
        ));
    }
    for index in 0..10 {
        model.sources.tree_rows.push(native_folder_row_model(
            format!("folder_{index:02}"),
            String::new(),
            index % 2,
            index == 0,
            index == 1,
            index == 0,
            false,
            false,
        ));
    }
    model.columns[0].item_count = 5;
    model.columns[1].item_count = 12;
    model.columns[2].item_count = 1;
    model.browser.visible_count = 12;
    model.browser.selected_visible_row = Some(1);
    model.browser.anchor_visible_row = Some(1);
    model.browser.search_placeholder = Some(String::from("Search samples"));
    model.browser.search_query = String::from("kick");
    model.browser.sort_label = Some(String::from("List order"));
    model.browser.active_tab_label = Some(String::from("Samples"));
    model.browser_chrome.item_count_label = String::from("12 items");
    model.browser_actions.can_delete = true;
    model.browser_actions.can_tag = true;
    model.browser_actions.can_rename = true;
    model
}

pub(super) fn browser_dense_model() -> NativeAppModel {
    let mut model = NativeAppModel::default();
    model.title = String::from("Wavecrate - Dense Browser");
    model.status.left = String::from("Focus list");
    model.status.center = String::from("rows: 500 | selected: 7 | anchor: 72 | search: dense");
    model.status.right = String::from("col: 2/3");
    model.status_text = String::from("Dense browser scene");
    model.transport_running = true;

    for index in 0..14 {
        model.sources.rows.push(NativeSourceRowModel::new(
            format!("source_{index:02}"),
            format!("/source_{index:02}.wav"),
            index == 3,
            false,
        ));
    }
    for index in 0..16 {
        model.sources.tree_rows.push(native_folder_row_model(
            format!("folder_{index:02}"),
            String::new(),
            index % 3,
            index == 8,
            index == 1,
            index == 0,
            true,
            true,
        ));
    }
    for index in 0..500 {
        let mut row = NativeBrowserRowModel::new(
            index,
            format!("row_{index:03}.wav"),
            index % 3,
            index % 11 == 0,
            index == 72,
        );
        if index % 5 == 0 {
            row = row.with_bucket_label(format!("BPM {index}"));
        }
        row.processing_state = match index {
            65 => NativeBrowserRowProcessingState::Queued,
            72 => NativeBrowserRowProcessingState::Active,
            73 => NativeBrowserRowProcessingState::Completed,
            74 => NativeBrowserRowProcessingState::Failed,
            _ => NativeBrowserRowProcessingState::None,
        };
        model.browser.rows.push(row);
    }
    model.browser.visible_count = 500;
    model.browser.autoscroll = true;
    model.browser.view_start_row = 57;
    model.browser.selected_path_count = 7;
    model.browser.selected_visible_row = Some(72);
    model.browser.anchor_visible_row = Some(68);
    model.browser.search_query = String::from("kick");
    model.browser.search_placeholder = Some(String::from("Search samples"));
    model.browser.sort_label = Some(String::from("Name"));
    model.browser.active_tab_label = Some(String::from("Samples"));
    model.browser_chrome.item_count_label = String::from("500 items");
    model.browser_actions.can_delete = true;
    model.browser_actions.can_tag = true;
    model.browser_actions.can_rename = false;
    model.columns[0].item_count = 40;
    model.columns[1].item_count = 460;
    model
}

pub(super) fn waveform_selection_model() -> NativeAppModel {
    let mut model = NativeAppModel::default();
    model.title = String::from("Wavecrate Native Waveform");
    model.status.left = String::from("Waveform focus");
    model.status.center = String::from("rows: 48 | selected: 2 | anchor: 1 | search: wav");
    model.status.right = String::from("col: 2/3");
    model.status_text = String::from("Waveform scene");
    model.transport_running = true;
    model.sources.search_query = String::from("sample");
    for index in 0..20 {
        model.sources.rows.push(NativeSourceRowModel::new(
            format!("source_{index:02}"),
            String::new(),
            index == 9,
            false,
        ));
        model.browser.rows.push(
            NativeBrowserRowModel::new(
                index,
                format!("track_{index:03}.wav"),
                index % 3,
                index % 6 == 0,
                index == 1,
            )
            .with_bucket_label(format!("{} bpm", 90 + index)),
        );
    }
    model.browser.visible_count = 20;
    model.browser.selected_visible_row = Some(1);
    model.browser.anchor_visible_row = Some(1);
    model.browser.search_query = String::from("track");
    model.browser.selected_path_count = 2;
    model.browser_actions.can_delete = true;
    model.browser_actions.can_tag = true;
    model.browser_actions.can_rename = true;

    model.waveform.loaded_label = Some(String::from("track_001.wav"));
    model.waveform.cursor_milli = Some(315);
    model.waveform.playhead_milli = Some(620);
    model.waveform.selection_milli = Some(NativeNormalizedRangeModel::new(200, 760));
    model.waveform.view_start_milli = 50;
    model.waveform.view_end_milli = 950;
    model.waveform.loop_enabled = true;
    model.waveform.tempo_label = Some(String::from("128.0 BPM"));
    model.waveform.zoom_label = Some(String::from("125%"));
    model.waveform.waveform_image = Some(Arc::new(waveform_fixture_image(160, 32)));
    model.waveform.waveform_image_signature = Some(7_123_456);
    model.waveform_chrome.transport_hint = String::from("Loop enabled");
    model.map.active = false;
    model.map.summary = String::from("Waveform scene");
    model.browser_chrome.item_count_label = String::from("20 items");
    model.update.status = NativeUpdateStatusModel::Idle;
    model.update.status_label = String::from("Updates: idle");
    model.update.action_hint_label = String::from("Action: check");
    model
}

fn waveform_fixture_image(width: usize, height: usize) -> ImageRgba {
    let mut pixels = Vec::with_capacity(width.saturating_mul(height).saturating_mul(4));
    for y in 0..height {
        for x in 0..width {
            let x_u8 = u8::try_from(x * 5).unwrap_or(0);
            let y_u8 = u8::try_from((y * 13) % 256).unwrap_or(0);
            let alpha = if (x + y) % 9 == 0 {
                255
            } else if (x + y) % 13 == 0 {
                128
            } else {
                200
            };
            let color = x_u8.saturating_add(y_u8 / 2);
            pixels.extend_from_slice(&[color, 255_u8.wrapping_sub(color), color / 2, alpha]);
        }
    }
    ImageRgba::new(width, height, pixels).unwrap_or_else(|| {
        panic!(
            "failed to construct waveform fixture image ({width}x{height}) with {} px",
            width.saturating_mul(height).saturating_mul(4)
        )
    })
}
