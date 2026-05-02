use super::super::*;

#[test]
fn top_bar_surface_layout_stays_ordered_across_density_tiers() {
    let model = AppModel {
        update: crate::compat_app_contract::UpdatePanelModel {
            status: crate::compat_app_contract::UpdateStatusModel::Available,
            status_label: String::from("Update available: v20.1.0"),
            action_hint_label: String::from("Actions: open | install(manual) | dismiss"),
            release_notes_label: String::from("Release: v20.1.0"),
            available_version_label: Some(String::from("v20.1.0")),
            available_url: Some(String::from("https://example.invalid/release")),
            last_error: None,
        },
        ..AppModel::default()
    };
    for viewport in [
        Vector2::new(820.0, 520.0),
        Vector2::new(1280.0, 720.0),
        Vector2::new(2300.0, 1080.0),
    ] {
        let layout = ShellLayout::build(viewport);
        let style = style_for_layout(&layout);
        let surface = resolve_top_bar_surface_layout(
            layout.top_bar,
            style.sizing,
            &top_bar_surface_content(&model),
        );
        assert_rect_inside(layout.top_bar, surface.title_cluster);
        assert_rect_inside(layout.top_bar, surface.action_cluster);
        assert!(surface.title_cluster.max.x <= surface.action_cluster.min.x);
        assert_rect_inside(surface.title_cluster, surface.volume_meter_rect);
        assert_rect_inside(surface.title_cluster, surface.volume_value_rect);
        assert_rect_inside(surface.title_cluster, surface.volume_label_rect);
        let options = surface.options_button_rect.expect("options button");
        assert_rect_inside(surface.action_cluster, options);
        for button in &surface.update_buttons {
            assert_rect_inside(surface.action_cluster, button.rect);
            assert!(button.rect.max.x <= options.min.x);
        }
    }
}
