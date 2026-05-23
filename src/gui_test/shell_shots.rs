use crate::{app_core::actions::NativeAppModel, gui_runtime::capture_native_shell_shot_snapshot};
use std::{fs, path::PathBuf};

mod rasterize;
mod scenes;

use rasterize::{ShotSnapshot, rasterize_shot};
use scenes::{browser_dense_model, startup_scene_model, waveform_selection_model};

fn shot_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("gui_shots")
}

fn fixture_paths(name: &str) -> (PathBuf, PathBuf) {
    let root = shot_root();
    (
        root.join(format!("{name}.json")),
        root.join(format!("{name}.png")),
    )
}

fn write_or_compare_shot(name: &str, viewport: [f32; 2], model: NativeAppModel, write_mode: bool) {
    let snapshot = capture_native_shell_shot_snapshot(name, viewport, &model);
    let (json_path, png_path) = fixture_paths(name);
    let snapshot_json = serde_json::to_value(&snapshot)
        .unwrap_or_else(|err| panic!("serialize actual shot snapshot for {name}: {err}"));

    if write_mode {
        fs::create_dir_all(shot_root()).unwrap_or_else(|err| {
            panic!("create fixture directory {}: {err}", shot_root().display())
        });
        fs::write(
            &json_path,
            serde_json::to_string_pretty(&snapshot_json)
                .unwrap_or_else(|err| panic!("serialize shot snapshot for {name}: {err}")),
        )
        .unwrap_or_else(|err| {
            panic!(
                "write shot JSON fixture for {name} to {}: {err}",
                json_path.display()
            )
        });
        let snapshot: ShotSnapshot = serde_json::from_value(snapshot_json)
            .unwrap_or_else(|err| panic!("parse actual shot snapshot for {name}: {err}"));
        rasterize_shot(&snapshot)
            .save(&png_path)
            .unwrap_or_else(|err| {
                panic!(
                    "write shot PNG fixture for {name} to {}: {err}",
                    png_path.display()
                )
            });
        return;
    }

    let expected_json = fs::read_to_string(&json_path).unwrap_or_else(|err| {
        panic!(
            "read expected JSON shot {name} from {}: {err}",
            json_path.display()
        )
    });
    let expected_json: serde_json::Value =
        serde_json::from_str(&expected_json).unwrap_or_else(|err| {
            panic!(
                "parse expected JSON shot {name} from {}: {err}",
                json_path.display()
            )
        });
    assert_eq!(
        canonicalize_json(expected_json),
        canonicalize_json(snapshot_json.clone()),
        "shot fixture mismatch for {name}: {}",
        json_path.display()
    );

    let expected_png = image::open(&png_path).unwrap_or_else(|err| {
        panic!(
            "read expected PNG shot {name} from {}: {err}",
            png_path.display()
        )
    });
    let expected = expected_png.to_rgba8();
    let snapshot: ShotSnapshot = serde_json::from_value(snapshot_json)
        .unwrap_or_else(|err| panic!("parse actual shot snapshot for {name}: {err}"));
    let actual = rasterize_shot(&snapshot);
    assert_eq!(expected.width(), actual.width(), "PNG width mismatch");
    assert_eq!(expected.height(), actual.height(), "PNG height mismatch");
    assert_eq!(expected.into_raw(), actual.into_raw(), "PNG bytes mismatch");
}

#[test]
fn startup_shot_matches_fixture() {
    write_or_compare_shot("startup", [1280.0, 720.0], startup_scene_model(), false);
}

#[test]
fn browser_dense_shot_matches_fixture() {
    write_or_compare_shot(
        "browser_dense",
        [1600.0, 900.0],
        browser_dense_model(),
        false,
    );
}

#[test]
fn waveform_selection_shot_matches_fixture() {
    write_or_compare_shot(
        "waveform_selection",
        [1440.0, 810.0],
        waveform_selection_model(),
        false,
    );
}

#[ignore = "Generate snapshot fixtures with `cargo test -p wavecrate --lib update_shot_fixtures -- --ignored`"]
#[test]
fn update_shot_fixtures() {
    write_or_compare_shot("startup", [1280.0, 720.0], startup_scene_model(), true);
    write_or_compare_shot(
        "browser_dense",
        [1600.0, 900.0],
        browser_dense_model(),
        true,
    );
    write_or_compare_shot(
        "waveform_selection",
        [1440.0, 810.0],
        waveform_selection_model(),
        true,
    );
}

fn canonicalize_json(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut entries: Vec<(String, serde_json::Value)> = map.into_iter().collect();
            entries.sort_by(|left, right| left.0.cmp(&right.0));
            let canonical = entries
                .into_iter()
                .map(|(key, nested)| (key, canonicalize_json(nested)))
                .collect::<serde_json::Map<String, serde_json::Value>>();
            serde_json::Value::Object(canonical)
        }
        serde_json::Value::Array(values) => {
            serde_json::Value::Array(values.into_iter().map(canonicalize_json).collect())
        }
        serde_json::Value::Number(number) => number
            .as_f64()
            .filter(|value| value.is_finite() && value.fract() != 0.0)
            .and_then(|value| serde_json::Number::from_f64((value * 1000.0).round() / 1000.0))
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Number(number)),
        primitive => primitive,
    }
}
