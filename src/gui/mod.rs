//! Backend-agnostic GUI primitives re-exported from the standalone `radiant` crate.
//!
//! Architectural boundary:
//!
//! * `src/gui` exposes `radiant` API types only (inputs, layout-independent types,
//!   repaint signals).
//! * it performs no widget construction, state transitions, layout decisions, or hit
//!   testing.
//! * it performs no input normalization or propagation policy.
//! * it performs no rendering orchestration.
//!
//! Keeping these modules as pure re-exports prevents accidental duplication of GUI
//! primitives in `sempal` and makes ownership boundaries enforceable in code review.

pub mod input {
    //! Shared key, pointer, and modifier tokens from `radiant`.
    //!
    //! The types are re-exported to avoid duplication of input vocabulary in
    //! application code.
    pub use radiant::gui::input::*;
}

pub mod layout_core {
    //! Generic slot-based layout primitives from `radiant`.
    pub use radiant::gui::layout_core::*;
}

pub mod text_layout {
    //! Generic text placement helpers from `radiant`.
    pub use radiant::gui::text_layout::*;
}

pub mod automation {
    //! Generic automation snapshot primitives from `radiant`.
    pub use radiant::gui::automation::*;
}

pub mod badge {
    //! Generic badge and pill primitives from `radiant`.
    pub use radiant::gui::badge::*;
}

pub mod chrome {
    //! Generic chrome and status-surface primitives from `radiant`.
    pub use radiant::gui::chrome::*;
}

pub mod feedback {
    //! Generic user-feedback surface primitives from `radiant`.
    pub use radiant::gui::feedback::*;
}

pub mod focus {
    //! Generic focus routing primitives from `radiant`.
    pub use radiant::gui::focus::*;
}

pub mod fingerprint {
    //! Generic stable fingerprint helpers from `radiant`.
    pub use radiant::gui::fingerprint::*;
}

pub mod form {
    //! Generic form and picker primitives from `radiant`.
    pub use radiant::gui::form::*;
}

pub mod frame {
    //! Frame feedback primitives from `radiant`.
    pub use radiant::gui::frame::*;
}

pub mod invalidation {
    //! Generic retained invalidation primitives from `radiant`.
    pub use radiant::gui::invalidation::*;
}

pub mod list {
    //! Generic list and virtualization primitives from `radiant`.
    //!
    //! Re-exports keep Sempal's large-list behavior on the framework-owned
    //! virtualization contract instead of duplicating list-window math locally.
    pub use radiant::gui::list::*;
}

pub mod paint {
    //! Backend-neutral paint primitives from `radiant`.
    pub use radiant::gui::paint::*;
}

pub mod panel {
    //! Generic panel and split-pane primitives from `radiant`.
    pub use radiant::gui::panel::*;
}

pub mod range {
    //! Normalized range and viewport projection primitives from `radiant`.
    //!
    //! Re-exported so Sempal-owned waveform and timeline surfaces use generic
    //! normalized coordinate math instead of duplicating projection helpers.
    pub use radiant::gui::range::*;
}

pub mod retained {
    //! Retained snapshot storage primitives from `radiant`.
    pub use radiant::gui::retained::*;
}

pub mod selection {
    //! Generic selection state primitives from `radiant`.
    pub use radiant::gui::selection::*;
}

pub mod shortcuts {
    //! Generic shortcut resolution primitives from `radiant`.
    pub use radiant::gui::shortcuts::*;
}

pub mod snapshot {
    //! Serializable visual snapshot primitives from `radiant`.
    pub use radiant::gui::snapshot::*;
}

pub mod repaint {
    //! Signals used to request UI updates from background work.
    //!
    //! Re-exports allow application subsystems to request deterministic paint
    //! invalidation without depending on runtime internals.
    pub use radiant::gui::repaint::*;
}

pub mod svg {
    //! SVG helpers used by Sempal-owned native-shell icon rasterization.
    //!
    //! Radiant owns retained SVG painting now. Sempal keeps this small subset
    //! parser only for native-shell paths that still rasterize SVG assets into
    //! image primitives before handing them to the runtime.
    pub use radiant::gui::svg::*;

    use vello::kurbo::{
        Affine, BezPath, Circle as KurboCircle, Point as KurboPoint, Rect as KurboRect, Shape, Vec2,
    };

    /// Parsed SVG document ready for legacy icon rasterization.
    #[derive(Clone, Debug)]
    pub struct SvgDocument {
        /// The minimum x coordinate in the declared view box.
        pub view_box_min_x: f32,
        /// The minimum y coordinate in the declared view box.
        pub view_box_min_y: f32,
        /// The width of the declared view box.
        pub view_box_width: f32,
        /// The height of the declared view box.
        pub view_box_height: f32,
        /// The transformed filled shapes emitted by the document.
        pub shapes: Vec<SvgShape>,
    }

    /// One rasterizable filled SVG shape.
    #[derive(Clone, Debug)]
    pub struct SvgShape {
        path: BezPath,
        fill_rule: SvgFillRule,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum SvgFillRule {
        NonZero,
        EvenOdd,
    }

    /// Parse one SVG document from an asset file.
    pub fn parse_svg_document(svg: &str) -> Option<SvgDocument> {
        let document = roxmltree::Document::parse(svg).ok()?;
        let root = document.root_element();
        if root.tag_name().name() != "svg" {
            return None;
        }

        let view_box_values = parse_number_list(root.attribute("viewBox")?)?;
        if view_box_values.len() != 4 {
            return None;
        }

        let mut shapes = Vec::new();
        collect_shapes(
            root,
            Affine::IDENTITY,
            resolve_fill_rule(root, SvgFillRule::NonZero),
            &mut shapes,
        )?;
        if shapes.is_empty() {
            return None;
        }

        Some(SvgDocument {
            view_box_min_x: view_box_values[0] as f32,
            view_box_min_y: view_box_values[1] as f32,
            view_box_width: view_box_values[2] as f32,
            view_box_height: view_box_values[3] as f32,
            shapes,
        })
    }

    fn collect_shapes(
        node: roxmltree::Node<'_, '_>,
        inherited_transform: Affine,
        inherited_fill_rule: SvgFillRule,
        shapes: &mut Vec<SvgShape>,
    ) -> Option<()> {
        let local_transform = parse_transform_list(node.attribute("transform"))?;
        let transform = inherited_transform * local_transform;
        let fill_rule = resolve_fill_rule(node, inherited_fill_rule);

        match node.tag_name().name() {
            "svg" | "g" => {
                for child in node.children().filter(roxmltree::Node::is_element) {
                    collect_shapes(child, transform, fill_rule, shapes)?;
                }
            }
            "path" => {
                if !shape_is_filled(node) {
                    return Some(());
                }
                let path = BezPath::from_svg(node.attribute("d")?).ok()?;
                shapes.push(SvgShape {
                    path: transform * path,
                    fill_rule,
                });
            }
            "rect" => {
                if !shape_is_filled(node) {
                    return Some(());
                }
                let x = parse_attr_f64(node, "x").unwrap_or(0.0);
                let y = parse_attr_f64(node, "y").unwrap_or(0.0);
                let width = parse_attr_f64(node, "width")?;
                let height = parse_attr_f64(node, "height")?;
                let path = KurboRect::new(x, y, x + width, y + height).to_path(0.1);
                shapes.push(SvgShape {
                    path: transform * path,
                    fill_rule,
                });
            }
            "circle" => {
                if !shape_is_filled(node) {
                    return Some(());
                }
                let circle = KurboCircle::new(
                    KurboPoint::new(parse_attr_f64(node, "cx")?, parse_attr_f64(node, "cy")?),
                    parse_attr_f64(node, "r")?,
                );
                shapes.push(SvgShape {
                    path: transform * circle.to_path(0.1),
                    fill_rule,
                });
            }
            "polygon" => {
                if !shape_is_filled(node) {
                    return Some(());
                }
                let points = parse_points(node.attribute("points")?)?;
                let mut path = BezPath::new();
                let first = points.first()?;
                path.move_to(*first);
                for point in points.iter().skip(1) {
                    path.line_to(*point);
                }
                path.close_path();
                shapes.push(SvgShape {
                    path: transform * path,
                    fill_rule,
                });
            }
            _ => {}
        }

        Some(())
    }

    fn resolve_fill_rule(node: roxmltree::Node<'_, '_>, inherited: SvgFillRule) -> SvgFillRule {
        node.attribute("fill-rule")
            .and_then(parse_fill_rule)
            .or_else(|| extract_style_property(node, "fill-rule").and_then(parse_fill_rule))
            .unwrap_or(inherited)
    }

    fn parse_fill_rule(raw: &str) -> Option<SvgFillRule> {
        match raw.trim() {
            "evenodd" => Some(SvgFillRule::EvenOdd),
            "nonzero" => Some(SvgFillRule::NonZero),
            _ => None,
        }
    }

    fn shape_is_filled(node: roxmltree::Node<'_, '_>) -> bool {
        let fill = node
            .attribute("fill")
            .or_else(|| extract_style_property(node, "fill"));
        !matches!(fill.map(str::trim), Some("none"))
    }

    fn extract_style_property<'a>(
        node: roxmltree::Node<'a, 'a>,
        property: &str,
    ) -> Option<&'a str> {
        let style = node.attribute("style")?;
        style.split(';').find_map(|entry| {
            let (name, value) = entry.split_once(':')?;
            (name.trim() == property).then_some(value.trim())
        })
    }

    /// Determine whether one point lands inside any parsed SVG shape.
    pub fn point_in_svg_shapes(x: f32, y: f32, shapes: &[SvgShape]) -> bool {
        let point = KurboPoint::new(x as f64, y as f64);
        shapes.iter().any(|shape| point_in_svg_shape(point, shape))
    }

    fn point_in_svg_shape(point: KurboPoint, shape: &SvgShape) -> bool {
        match shape.fill_rule {
            SvgFillRule::NonZero => shape.path.contains(point),
            SvgFillRule::EvenOdd => shape.path.winding(point).abs() % 2 == 1,
        }
    }

    fn parse_attr_f64(node: roxmltree::Node<'_, '_>, attr: &str) -> Option<f64> {
        parse_number(node.attribute(attr)?)
    }

    fn parse_transform_list(raw: Option<&str>) -> Option<Affine> {
        let Some(mut remaining) = raw.map(str::trim) else {
            return Some(Affine::IDENTITY);
        };
        if remaining.is_empty() {
            return Some(Affine::IDENTITY);
        }

        let mut transform = Affine::IDENTITY;
        while !remaining.is_empty() {
            remaining =
                remaining.trim_start_matches(|ch: char| ch.is_ascii_whitespace() || ch == ',');
            if remaining.is_empty() {
                break;
            }
            let open = remaining.find('(')?;
            let name = remaining[..open].trim();
            let body = &remaining[open + 1..];
            let close = body.find(')')?;
            let args = &body[..close];
            remaining = &body[close + 1..];
            transform *= parse_single_transform(name, args)?;
        }
        Some(transform)
    }

    fn parse_single_transform(name: &str, args: &str) -> Option<Affine> {
        let values = parse_number_list(args)?;
        match name {
            "matrix" if values.len() == 6 => Some(Affine::new([
                values[0], values[1], values[2], values[3], values[4], values[5],
            ])),
            "translate" if values.len() == 1 => Some(Affine::translate(Vec2::new(values[0], 0.0))),
            "translate" if values.len() == 2 => {
                Some(Affine::translate(Vec2::new(values[0], values[1])))
            }
            "scale" if values.len() == 1 => Some(Affine::scale(values[0])),
            "scale" if values.len() == 2 => Some(Affine::scale_non_uniform(values[0], values[1])),
            "rotate" if values.len() == 1 => Some(Affine::rotate(values[0].to_radians())),
            "rotate" if values.len() == 3 => {
                let center = KurboPoint::new(values[1], values[2]);
                Some(
                    Affine::translate(center.to_vec2())
                        * Affine::rotate(values[0].to_radians())
                        * Affine::translate(-center.to_vec2()),
                )
            }
            "skewX" if values.len() == 1 => Some(Affine::new([
                1.0,
                0.0,
                values[0].to_radians().tan(),
                1.0,
                0.0,
                0.0,
            ])),
            "skewY" if values.len() == 1 => Some(Affine::new([
                1.0,
                values[0].to_radians().tan(),
                0.0,
                1.0,
                0.0,
                0.0,
            ])),
            _ => None,
        }
    }

    fn parse_points(points: &str) -> Option<Vec<KurboPoint>> {
        let coords = parse_number_list(points)?;
        if coords.len() < 6 || coords.len() % 2 != 0 {
            return None;
        }
        Some(
            coords
                .chunks_exact(2)
                .map(|pair| KurboPoint::new(pair[0], pair[1]))
                .collect(),
        )
    }

    fn parse_number_list(raw: &str) -> Option<Vec<f64>> {
        let normalized = raw.replace(',', " ");
        normalized
            .split_whitespace()
            .map(parse_number)
            .collect::<Option<Vec<_>>>()
    }

    fn parse_number(raw: &str) -> Option<f64> {
        raw.trim().parse::<f64>().ok()
    }
}

pub mod types {
    //! Light-weight value types used by UI declarations and render payloads.
    //!
    //! These types are intentionally constrained to data contracts (geometry,
    //! style primitives, IDs) and intentionally exclude behavior.
    pub use radiant::gui::types::*;
}

pub mod visualization {
    //! Generic visualization primitives from `radiant`.
    //!
    //! Re-exported so Sempal-owned waveform, timeline, and map surfaces consume
    //! framework-owned data contracts instead of compatibility aliases.
    pub use radiant::gui::visualization::*;
}
