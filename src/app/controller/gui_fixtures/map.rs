//! Map-specific controller state seeded for GUI fixture scenarios.

use super::{GuiFixtureControllerBundle, build_browser_fixture_with_source_id};
use crate::{
    app::controller::library::analysis_jobs,
    app::state::{MapBounds, MapPoint, MapQueryBounds},
    sample_sources::SourceId,
    waveform::WaveformRenderer,
};
use std::{path::Path, sync::Arc};

const GUI_TEST_MAP_SOURCE_ID: &str = "gui-map-source";
const GUI_TEST_MAP_SAMPLE_ID: &str = "gui-map-source::kick_one.wav";
const MAP_FIXTURE_UMAP_VERSION: &str = "v1";

pub(super) fn build_map_fixture(
    renderer: WaveformRenderer,
) -> Result<GuiFixtureControllerBundle, String> {
    let mut bundle = build_browser_fixture_with_source_id(
        renderer,
        Some(SourceId::from_string(GUI_TEST_MAP_SOURCE_ID)),
    )?;
    seed_map_state(&mut bundle)?;
    Ok(bundle)
}

fn seed_map_state(bundle: &mut GuiFixtureControllerBundle) -> Result<(), String> {
    let source_id = current_source_id(bundle)?;
    let sample_id = analysis_jobs::build_sample_id(source_id.as_str(), Path::new("kick_one.wav"));
    debug_assert_eq!(sample_id, GUI_TEST_MAP_SAMPLE_ID);

    bundle.controller.set_browser_tab(true);
    bundle.controller.ui.map.bounds = Some(full_map_bounds());
    bundle.controller.ui.map.cached_bounds_source_id = Some(source_id.clone());
    bundle.controller.ui.map.cached_bounds_umap_version = Some(map_version());
    bundle.controller.ui.map.last_query = Some(full_map_query());
    bundle.controller.ui.map.cached_points = vec![fixture_map_point(sample_id)];
    bundle.controller.ui.map.cached_points_source_id = Some(source_id);
    bundle.controller.ui.map.cached_points_umap_version = Some(map_version());
    bundle.controller.ui.map.cached_points_revision = 1;
    bundle.controller.ui.map.selected_sample_id = None;
    bundle.controller.ui.map.hovered_sample_id = None;
    bundle.controller.ui.map.paint_hover_active_id = None;
    bundle.controller.focus_browser_list();
    Ok(())
}

fn current_source_id(bundle: &GuiFixtureControllerBundle) -> Result<String, String> {
    bundle
        .controller
        .current_source()
        .map(|source| source.id.as_str().to_string())
        .ok_or_else(|| String::from("map fixture missing current source"))
}

fn full_map_bounds() -> MapBounds {
    MapBounds {
        min_x: -1.0,
        max_x: 1.0,
        min_y: -1.0,
        max_y: 1.0,
    }
}

fn full_map_query() -> MapQueryBounds {
    MapQueryBounds {
        min_x: -1.0,
        max_x: 1.0,
        min_y: -1.0,
        max_y: 1.0,
    }
}

fn fixture_map_point(sample_id: String) -> MapPoint {
    MapPoint {
        sample_id: Arc::<str>::from(sample_id),
        x: 0.0,
        y: 0.0,
        cluster_id: Some(1),
    }
}

fn map_version() -> String {
    String::from(MAP_FIXTURE_UMAP_VERSION)
}
