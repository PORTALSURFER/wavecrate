use radiant::prelude as ui;

const TOOLBAR_ICON_ACTIVE_COLOR: ui::Rgba8 = ui::Rgba8::new(255, 160, 82, 255);
const TOOLBAR_ICON_ENABLED_COLOR: ui::Rgba8 = ui::Rgba8::new(238, 238, 238, 255);
const TOOLBAR_ICON_DISABLED_COLOR: ui::Rgba8 = ui::Rgba8::new(145, 145, 145, 255);
const TOOLBAR_ICON_TINTS: ui::SvgIconTintPalette = ui::SvgIconTintPalette::new(
    TOOLBAR_ICON_ENABLED_COLOR,
    TOOLBAR_ICON_ACTIVE_COLOR,
    TOOLBAR_ICON_DISABLED_COLOR,
);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::native_app) enum ToolbarIcon {
    FocusLoaded,
    Loop,
    Random,
    SimilarSections,
    BeatGuides,
    BeatGuideMinus,
    BeatGuidePlus,
    Play,
    Stop,
}

impl ToolbarIcon {
    fn cache(self) -> &'static ui::SvgIconTintCache {
        match self {
            Self::FocusLoaded => &FOCUS_LOADED_ICON,
            Self::Loop => &LOOP_ICON,
            Self::Random => &RANDOM_ICON,
            Self::SimilarSections => &SIMILAR_SECTIONS_ICON,
            Self::BeatGuides => &BEAT_GUIDES_ICON,
            Self::BeatGuideMinus => &BEAT_GUIDE_MINUS_ICON,
            Self::BeatGuidePlus => &BEAT_GUIDE_PLUS_ICON,
            Self::Play => &PLAY_ICON,
            Self::Stop => &STOP_ICON,
        }
    }
}

pub(in crate::native_app) fn toolbar_icon_color(enabled: bool, active: bool) -> ui::Rgba8 {
    TOOLBAR_ICON_TINTS.color(enabled, active)
}

pub(in crate::native_app) fn toolbar_icon_glyph(
    icon: ToolbarIcon,
    enabled: bool,
    active: bool,
) -> ui::SvgIcon {
    icon.cache()
        .icon_for_state(TOOLBAR_ICON_TINTS, enabled, active)
}

static FOCUS_LOADED_ICON: ui::SvgIconTintCache = ui::SvgIconTintCache::new(
    r#"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <rect x="3" y="3" width="2" height="2"/>
  <rect x="6" y="3.25" width="7" height="1.5"/>
  <rect x="3" y="7" width="2" height="2"/>
  <rect x="6" y="7.25" width="7" height="1.5"/>
  <rect x="3" y="11" width="2" height="2"/>
  <rect x="6" y="11.25" width="7" height="1.5"/>
</svg>"#,
);

static LOOP_ICON: ui::SvgIconTintCache = ui::SvgIconTintCache::new(
    r#"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <path d="M4 3h5.4V1.5L14 5l-4.6 3.5V7H4.2C3 7 2 8 2 9.2V10H.5v-.8C.5 5.8 2 3 4 3z"/>
  <path d="M12 13H6.6v1.5L2 11l4.6-3.5V9H12c1.2 0 2-1 2-2.2V6h1.5v.8C15.5 10.2 14 13 12 13z"/>
</svg>"#,
);

static RANDOM_ICON: ui::SvgIconTintCache = ui::SvgIconTintCache::new(
    r#"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <path d="M2 4h2.1c1.8 0 2.9.8 4.1 2.5l.8 1.1c.8 1.1 1.4 1.4 2.6 1.4H12V7l3 3-3 3v-2h-.4c-1.9 0-3.1-.7-4.2-2.4l-.8-1.1C5.8 6.3 5.2 6 4.1 6H2z"/>
  <path d="M11.6 4H12V2l3 3-3 3V6h-.4c-1.2 0-1.8.3-2.6 1.4l-.2.3-.9-1.4.5-.7C8.5 4.7 9.7 4 11.6 4z"/>
  <path d="M2 10h2.1c1.1 0 1.7-.3 2.5-1.5l.9 1.4c-1 1.4-2 2.1-3.4 2.1H2z"/>
</svg>"#,
);

static SIMILAR_SECTIONS_ICON: ui::SvgIconTintCache = ui::SvgIconTintCache::new(
    r#"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <rect x="2" y="3" width="3.5" height="3"/>
  <rect x="10.5" y="3" width="3.5" height="3"/>
  <rect x="2" y="10" width="3.5" height="3"/>
  <rect x="10.5" y="10" width="3.5" height="3"/>
  <rect x="6.5" y="4.1" width="3" height="1.2"/>
  <rect x="6.5" y="11.1" width="3" height="1.2"/>
  <rect x="3.15" y="7" width="1.2" height="2"/>
  <rect x="11.65" y="7" width="1.2" height="2"/>
</svg>"#,
);

static BEAT_GUIDES_ICON: ui::SvgIconTintCache = ui::SvgIconTintCache::new(
    r#"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <rect x="2" y="3" width="1.5" height="10"/>
  <rect x="12.5" y="3" width="1.5" height="10"/>
  <rect x="5.5" y="5" width="1" height="6"/>
  <rect x="9.5" y="5" width="1" height="6"/>
  <rect x="2" y="7.25" width="12" height="1.5"/>
</svg>"#,
);

static BEAT_GUIDE_MINUS_ICON: ui::SvgIconTintCache = ui::SvgIconTintCache::new(
    r#"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <rect x="4" y="7.25" width="8" height="1.5"/>
</svg>"#,
);

static BEAT_GUIDE_PLUS_ICON: ui::SvgIconTintCache = ui::SvgIconTintCache::new(
    r#"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <rect x="4" y="7.25" width="8" height="1.5"/>
  <rect x="7.25" y="4" width="1.5" height="8"/>
</svg>"#,
);

static PLAY_ICON: ui::SvgIconTintCache = ui::SvgIconTintCache::new(
    r#"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <polygon points="4,3 13,8 4,13"/>
</svg>"#,
);

static STOP_ICON: ui::SvgIconTintCache = ui::SvgIconTintCache::new(
    r#"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <rect x="4" y="4" width="8" height="8"/>
</svg>"#,
);
