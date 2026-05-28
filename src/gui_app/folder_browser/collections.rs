use std::{collections::BTreeMap, path::PathBuf};

use radiant::{
    gui::types::{Point, Rect, Rgba8},
    layout::{LayoutOutput, Vector2},
    runtime::{PaintFillRect, PaintPrimitive, PaintStrokeRect, PaintText, PaintTextRun},
    theme::ThemeTokens,
    widgets::{
        DragHandleMessage, FocusBehavior, PaintBounds, PointerButton, TextInputMessage, TextWrap,
        Widget, WidgetCommon, WidgetInput, WidgetOutput, WidgetSizing,
    },
};
use wavecrate::sample_sources::SampleCollection;

use super::{FolderBrowserDrag, FolderBrowserState};

pub(in crate::gui_app) const COLLECTION_ROW_HEIGHT: f32 = 22.0;
pub(in crate::gui_app) const MIN_COLLECTIONS_PANEL_HEIGHT: f32 = 72.0;
pub(in crate::gui_app) const MAX_COLLECTIONS_PANEL_HEIGHT: f32 = 260.0;
pub(in crate::gui_app) const DEFAULT_COLLECTIONS_PANEL_HEIGHT: f32 = 168.0;

#[derive(Clone, Debug, PartialEq)]
pub(in crate::gui_app) struct SampleCollectionView {
    pub(in crate::gui_app) collection: SampleCollection,
    pub(in crate::gui_app) hotkey: char,
    pub(in crate::gui_app) name: String,
    pub(in crate::gui_app) color: Rgba8,
    pub(in crate::gui_app) selected: bool,
    pub(in crate::gui_app) drop_target: bool,
    pub(in crate::gui_app) drag_active: bool,
    pub(in crate::gui_app) assigned_count: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct SampleCollectionConfig {
    pub(super) collection: SampleCollection,
    pub(super) hotkey: char,
    pub(super) name: String,
    pub(super) color: Rgba8,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct CollectionPanelResize {
    pub(super) start_y: f32,
    pub(super) start_height: f32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct CollectionRenameEdit {
    pub(super) collection: SampleCollection,
    pub(super) draft: String,
    pub(super) input_id: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::gui_app) struct CollectionRenameView {
    pub(in crate::gui_app) draft: String,
    pub(in crate::gui_app) input_id: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::gui_app) enum CollectionHitMessage {
    Activate,
    Drop,
    HoverDropTarget(Point),
}

#[derive(Clone, Debug)]
pub(in crate::gui_app) struct CollectionHitTarget {
    common: WidgetCommon,
    label: String,
    hotkey: char,
    color: Rgba8,
    selected: bool,
    drop_target: bool,
    drag_active: bool,
    assigned_count: usize,
}

impl FolderBrowserState {
    pub(super) fn default_collections() -> Vec<SampleCollectionConfig> {
        (0..SampleCollection::COUNT)
            .filter_map(|index| {
                let collection = SampleCollection::new(index as u8)?;
                Some(SampleCollectionConfig {
                    collection,
                    hotkey: collection_hotkey(collection),
                    name: format!("Collection {}", collection_hotkey(collection)),
                    color: collection_color(collection),
                })
            })
            .collect()
    }

    pub(in crate::gui_app) fn collections_panel_height(&self) -> f32 {
        self.collections_panel_height
    }

    pub(in crate::gui_app) fn visible_collections(&self) -> Vec<SampleCollectionView> {
        let counts = self.collection_counts();
        self.collections
            .iter()
            .map(|collection| SampleCollectionView {
                collection: collection.collection,
                hotkey: collection.hotkey,
                name: collection.name.clone(),
                color: collection.color,
                selected: self.selected_collection == Some(collection.collection),
                drop_target: self.drop_target_collection == Some(collection.collection),
                drag_active: self.file_drag_active(),
                assigned_count: counts
                    .get(&collection.collection.index())
                    .copied()
                    .unwrap_or_default(),
            })
            .collect()
    }

    pub(in crate::gui_app) fn collection_color(
        &self,
        collection: SampleCollection,
    ) -> Option<Rgba8> {
        self.collections
            .iter()
            .find(|entry| entry.collection == collection)
            .map(|entry| entry.color)
    }

    pub(in crate::gui_app) fn set_file_collection_state(
        &mut self,
        path: &std::path::Path,
        collection: SampleCollection,
    ) -> bool {
        let path_id = path.to_string_lossy();
        let mut updated = false;
        for folder in &mut self.folders {
            if let Some(file) = folder
                .files
                .iter_mut()
                .find(|file| file.id == path_id.as_ref())
            {
                file.collection = Some(collection);
                updated = true;
            }
        }
        updated
    }

    pub(in crate::gui_app) fn selected_file_collection_candidates(
        &self,
    ) -> Vec<SelectedFileCollectionCandidate> {
        self.selected_audio_files()
            .into_iter()
            .filter(|file| self.is_file_selected(&file.id))
            .map(|file| SelectedFileCollectionCandidate {
                path: PathBuf::from(&file.id),
            })
            .collect()
    }

    pub(in crate::gui_app) fn drag_file_collection_candidates(
        &self,
    ) -> Vec<SelectedFileCollectionCandidate> {
        match &self.drag {
            Some(FolderBrowserDrag::Files { file_ids }) => file_ids
                .iter()
                .filter(|file_id| {
                    self.selected_files()
                        .iter()
                        .any(|file| file.id == **file_id)
                })
                .map(PathBuf::from)
                .map(|path| SelectedFileCollectionCandidate { path })
                .collect(),
            _ => Vec::new(),
        }
    }

    pub(super) fn resize_collections_panel(&mut self, message: DragHandleMessage) {
        match message {
            DragHandleMessage::Started { position } => {
                self.collection_panel_resize = Some(CollectionPanelResize {
                    start_y: position.y,
                    start_height: self.collections_panel_height,
                });
            }
            DragHandleMessage::Moved { position } | DragHandleMessage::Ended { position } => {
                let Some(resize) = self.collection_panel_resize.clone() else {
                    return;
                };
                self.collections_panel_height = (resize.start_height + position.y - resize.start_y)
                    .clamp(MIN_COLLECTIONS_PANEL_HEIGHT, MAX_COLLECTIONS_PANEL_HEIGHT);
                if matches!(message, DragHandleMessage::Ended { .. }) {
                    self.collection_panel_resize = None;
                }
            }
        }
    }

    pub(super) fn activate_collection(&mut self, collection: SampleCollection) {
        if self.selected_collection == Some(collection) && self.collection_rename_edit.is_none() {
            self.begin_rename_collection(collection);
            return;
        }
        self.selected_collection = Some(collection);
    }

    pub(in crate::gui_app) fn collection_rename_view(
        &self,
        collection: SampleCollection,
    ) -> Option<CollectionRenameView> {
        let edit = self.collection_rename_edit.as_ref()?;
        (edit.collection == collection).then(|| CollectionRenameView {
            draft: edit.draft.clone(),
            input_id: edit.input_id,
        })
    }

    pub(in crate::gui_app) fn begin_rename_collection(
        &mut self,
        collection: SampleCollection,
    ) -> Option<u64> {
        let entry = self
            .collections
            .iter()
            .find(|entry| entry.collection == collection)?;
        let input_id = collection_rename_input_id(collection);
        self.selected_collection = Some(collection);
        self.collection_rename_edit = Some(CollectionRenameEdit {
            collection,
            draft: entry.name.clone(),
            input_id,
        });
        Some(input_id)
    }

    pub(in crate::gui_app) fn apply_collection_rename_input(
        &mut self,
        message: TextInputMessage,
    ) -> Option<String> {
        let edit = self.collection_rename_edit.as_mut()?;
        match message {
            TextInputMessage::Changed { value } => {
                edit.draft = value;
                None
            }
            TextInputMessage::Submitted { value } => {
                let label = value.trim();
                if label.is_empty() {
                    self.collection_rename_edit = None;
                    return Some(String::from("Collection rename cancelled"));
                }
                if let Some(entry) = self
                    .collections
                    .iter_mut()
                    .find(|entry| entry.collection == edit.collection)
                {
                    entry.name = label.to_string();
                }
                self.collection_rename_edit = None;
                Some(String::from("Collection renamed"))
            }
            TextInputMessage::CompletionRequested { .. } => None,
        }
    }

    pub(super) fn hover_drop_target_collection(
        &mut self,
        collection: SampleCollection,
        position: Point,
    ) {
        self.update_drag_pointer(position);
        self.drop_target_collection = self.file_drag_active().then_some(collection);
    }

    fn collection_counts(&self) -> BTreeMap<u8, usize> {
        let mut counts = BTreeMap::new();
        for file in self.selected_audio_files() {
            if let Some(collection) = file.collection {
                *counts.entry(collection.index()).or_insert(0) += 1;
            }
        }
        counts
    }
}

impl CollectionHitTarget {
    pub(in crate::gui_app) fn new(collection: &SampleCollectionView) -> Self {
        let mut common = WidgetCommon::new(
            0,
            WidgetSizing::fixed(Vector2::new(1.0, COLLECTION_ROW_HEIGHT)),
        );
        common.focus = FocusBehavior::None;
        common.paint.bounds = PaintBounds::ClipToRect;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self {
            common,
            label: collection.name.clone(),
            hotkey: collection.hotkey,
            color: collection.color,
            selected: collection.selected,
            drop_target: collection.drop_target,
            drag_active: collection.drag_active,
            assigned_count: collection.assigned_count,
        }
    }
}

impl Widget for CollectionHitTarget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        match input {
            WidgetInput::PointerMove { position } => {
                self.common.state.hovered = bounds.contains(position);
                if self.common.state.hovered && self.drag_active && !self.drop_target {
                    return Some(WidgetOutput::typed(CollectionHitMessage::HoverDropTarget(
                        position,
                    )));
                }
                None
            }
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                ..
            } if bounds.contains(position) => {
                self.common.state.hovered = true;
                self.common.state.pressed = true;
                None
            }
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Primary,
                ..
            } => {
                let activated = self.common.state.pressed && bounds.contains(position);
                self.common.state.pressed = false;
                self.common.state.hovered = bounds.contains(position);
                activated.then(|| WidgetOutput::typed(CollectionHitMessage::Activate))
            }
            WidgetInput::PointerDrop {
                position,
                button: PointerButton::Primary,
                ..
            } if bounds.contains(position) => Some(WidgetOutput::typed(CollectionHitMessage::Drop)),
            _ => None,
        }
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        let Some(previous) = previous.as_any().downcast_ref::<Self>() else {
            return;
        };
        self.common.state = previous.common.state;
    }

    fn accepts_pointer_move(&self) -> bool {
        self.drag_active || self.drop_target || self.common.state.pressed
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        if self.selected || self.common.state.hovered || self.drop_target {
            let fill = if self.drop_target {
                self.color.blend_toward(theme.bg_primary, 0.72)
            } else if self.selected {
                theme.accent_mint.blend_toward(theme.bg_primary, 0.82)
            } else {
                theme.bg_secondary.blend_toward(theme.text_primary, 0.10)
            };
            primitives.push(PaintPrimitive::FillRect(PaintFillRect {
                widget_id: self.common.id,
                rect: bounds,
                color: fill,
            }));
        }

        if self.drop_target {
            primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
                widget_id: self.common.id,
                rect: Rect::from_min_max(
                    Point::new(bounds.min.x + 1.0, bounds.min.y + 1.0),
                    Point::new(bounds.max.x - 1.0, bounds.max.y - 1.0),
                ),
                color: self.color,
                width: 1.0,
            }));
        }

        let swatch = Rect::from_min_size(
            Point::new(bounds.min.x + 6.0, bounds.min.y + 6.0),
            Vector2::new(10.0, 10.0),
        );
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: self.common.id,
            rect: swatch,
            color: self.color,
        }));

        primitives.push(PaintPrimitive::Text(PaintTextRun {
            widget_id: self.common.id,
            text: PaintText::from(format!("{}  {}", self.hotkey, self.label)),
            rect: Rect::from_min_max(
                Point::new(bounds.min.x + 22.0, bounds.min.y),
                Point::new(bounds.max.x - 38.0, bounds.max.y),
            ),
            font_size: 12.0,
            baseline: Some((bounds.height() * 0.5 + 12.0 * 0.35).max(0.0)),
            color: theme.text_primary,
            align: radiant::runtime::PaintTextAlign::Left,
            wrap: TextWrap::None,
        }));
        if self.assigned_count > 0 {
            primitives.push(PaintPrimitive::Text(PaintTextRun {
                widget_id: self.common.id,
                text: PaintText::from(self.assigned_count.to_string()),
                rect: Rect::from_min_max(
                    Point::new(bounds.max.x - 34.0, bounds.min.y),
                    Point::new(bounds.max.x - 6.0, bounds.max.y),
                ),
                font_size: 11.0,
                baseline: Some((bounds.height() * 0.5 + 11.0 * 0.35).max(0.0)),
                color: theme.text_muted,
                align: radiant::runtime::PaintTextAlign::Right,
                wrap: TextWrap::None,
            }));
        }
    }
}

pub(in crate::gui_app) fn collection_hotkey(collection: SampleCollection) -> char {
    match collection.index() {
        0..=8 => char::from(b'1' + collection.index()),
        _ => '0',
    }
}

pub(in crate::gui_app) fn collection_color(collection: SampleCollection) -> Rgba8 {
    const COLORS: [Rgba8; 10] = [
        Rgba8 {
            r: 255,
            g: 86,
            b: 98,
            a: 240,
        },
        Rgba8 {
            r: 255,
            g: 166,
            b: 62,
            a: 240,
        },
        Rgba8 {
            r: 249,
            g: 220,
            b: 82,
            a: 240,
        },
        Rgba8 {
            r: 118,
            g: 226,
            b: 97,
            a: 240,
        },
        Rgba8 {
            r: 65,
            g: 221,
            b: 167,
            a: 240,
        },
        Rgba8 {
            r: 70,
            g: 202,
            b: 244,
            a: 240,
        },
        Rgba8 {
            r: 112,
            g: 146,
            b: 255,
            a: 240,
        },
        Rgba8 {
            r: 178,
            g: 117,
            b: 255,
            a: 240,
        },
        Rgba8 {
            r: 246,
            g: 105,
            b: 218,
            a: 240,
        },
        Rgba8 {
            r: 255,
            g: 132,
            b: 165,
            a: 240,
        },
    ];
    COLORS[collection.index() as usize]
}

fn collection_rename_input_id(collection: SampleCollection) -> u64 {
    0x5743_0000_0000_4300 + collection.index() as u64
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::gui_app) struct SelectedFileCollectionCandidate {
    pub(in crate::gui_app) path: PathBuf,
}
