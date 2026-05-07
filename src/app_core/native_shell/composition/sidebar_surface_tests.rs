use super::*;
use crate::app_core::native_shell::composition::style::StyleTokens;
use crate::gui::types::Point;
use crate::widgets::{ButtonWidget, TextWidget, Widget};

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
fn sidebar_surfaces_use_public_text_and_button_widgets() {
    let style = StyleTokens::for_viewport_width(1280.0);
    let header =
        build_sidebar_header_surface(&SidebarHeaderSurfaceContent::default(), style.sizing);
    let footer = build_sidebar_footer_surface(
        &SidebarFooterSurfaceContent {
            actions: sidebar_footer_action_specs(),
            ..SidebarFooterSurfaceContent::default()
        },
        style.sizing,
        208.0,
    );
    assert!(is_widget::<TextWidget>(&header, HEADER_TITLE_ID));
    assert!(is_widget::<ButtonWidget>(&header, HEADER_ADD_BUTTON_ID));
    assert!(is_widget::<ButtonWidget>(&footer, FOOTER_ACTION_BASE_ID));
}

#[test]
fn sidebar_surface_layout_keeps_header_and_footer_widgets_inside_bands() {
    for viewport_width in [820.0, 1280.0, 2300.0] {
        let style = StyleTokens::for_viewport_width(viewport_width);
        let header_rect = Rect::from_min_max(
            Point::new(0.0, 0.0),
            Point::new(208.0, style.sizing.source_header_block_height),
        );
        let footer_rect = Rect::from_min_max(
            Point::new(0.0, 40.0),
            Point::new(208.0, 40.0 + style.sizing.source_header_block_height),
        );
        let header = resolve_sidebar_header_surface_layout(
            header_rect,
            style.sizing,
            &SidebarHeaderSurfaceContent {
                title: String::from("48 sources"),
                query: String::from("search: drums"),
            },
        );
        let footer = resolve_sidebar_footer_surface_layout(
            footer_rect,
            style.sizing,
            &SidebarFooterSurfaceContent {
                primary_summary: String::from("+4 more…"),
                secondary_summary: String::from("recovery entries: 2"),
                actions: sidebar_footer_action_specs(),
            },
        );
        assert_inside(header_rect, header.title_text_rect);
        assert_inside(header_rect, header.query_text_rect);
        assert!(header.query_text_rect.min.y >= header.title_text_rect.max.y);
        let add = header.add_button_rect.expect("add button");
        assert_inside(header_rect, add);
        assert!(header.title_text_rect.max.x <= add.min.x);
        assert_inside(footer_rect, footer.primary_text_rect);
        assert_inside(footer_rect, footer.secondary_text_rect);
        for button in &footer.action_buttons {
            assert_inside(footer_rect, button.rect);
        }
    }
}
