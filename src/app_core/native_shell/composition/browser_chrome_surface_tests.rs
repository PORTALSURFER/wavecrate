use super::*;
use crate::{
    app_core::native_shell::composition::style::StyleTokens,
    gui::types::Point,
    widgets::{ButtonWidget, TextInputWidget, ToggleWidget, Widget},
};

fn is_widget<T: Widget + 'static>(surface: &UiSurface<()>, id: u64) -> bool {
    surface
        .find_widget(id)
        .is_some_and(|widget| widget.widget().as_any().downcast_ref::<T>().is_some())
}

fn assert_inside(outer: Rect, inner: Rect) {
    assert!(inner.min.x >= outer.min.x);
    assert!(inner.min.y >= outer.min.y);
    assert!(inner.max.x <= outer.max.x);
    assert!(inner.max.y <= outer.max.y);
}

#[test]
fn browser_tabs_surface_uses_public_button_widgets() {
    let style = StyleTokens::for_viewport_width(1280.0);
    let surface = build_browser_tabs_surface(
        &browser_tabs_surface_content(&AppModel::default()),
        style.sizing,
        800.0,
    );
    assert!(is_widget::<ButtonWidget>(&surface, TABS_ITEMS_ID));
    assert!(is_widget::<ButtonWidget>(&surface, TABS_MAP_ID));
}

#[test]
fn browser_tabs_surface_layout_stays_inside_tabs_band() {
    let style = StyleTokens::for_viewport_width(1280.0);
    let tabs_rect = Rect::from_min_max(Point::new(220.0, 244.0), Point::new(1220.0, 276.0));
    let layout = resolve_browser_tabs_surface_layout(
        tabs_rect,
        style.sizing,
        &browser_tabs_surface_content(&AppModel::default()),
    );
    assert_inside(tabs_rect, layout.items);
    assert_inside(tabs_rect, layout.map);
    assert!(layout.items.max.x <= layout.map.min.x);
}

#[test]
fn browser_toolbar_surface_uses_public_toggle_button_and_text_input_widgets() {
    let style = StyleTokens::for_viewport_width(1280.0);
    let content = browser_toolbar_surface_content(&AppModel::default());
    let surface = build_browser_toolbar_surface(
        &content,
        18.0,
        helpers::browser_toolbar_surface_widths(
            Rect::from_min_max(Point::new(220.0, 326.0), Point::new(1220.0, 344.0)),
            style.sizing,
        ),
    );
    assert!(is_widget::<ToggleWidget>(&surface, TOOLBAR_RATING_BASE_ID));
    assert!(is_widget::<ButtonWidget>(&surface, TOOLBAR_RANDOM_ID));
    assert!(is_widget::<TextInputWidget>(&surface, TOOLBAR_SEARCH_ID));
}

#[test]
fn browser_toolbar_surface_layout_preserves_search_and_button_order() {
    let style = StyleTokens::for_viewport_width(1280.0);
    let toolbar_rect = Rect::from_min_max(Point::new(220.0, 326.0), Point::new(1220.0, 344.0));
    let layout = resolve_browser_toolbar_surface_layout(
        toolbar_rect,
        style.sizing,
        &browser_toolbar_surface_content(&AppModel::default()),
    );
    assert!(
        layout
            .rating_filter_chips
            .iter()
            .all(|chip| chip.width() <= 1.0)
    );
    assert_inside(toolbar_rect, layout.search_field);
    assert!(layout.action_slots[0].max.x <= layout.action_slots[1].min.x);
    assert!(layout.action_slots[1].max.x <= layout.search_field.min.x);
    assert!(layout.search_field.max.x <= layout.action_slots[2].min.x);
    assert!(layout.activity_chip.width() <= 0.0);
    assert!(layout.sort_chip.width() <= 0.0);
}
