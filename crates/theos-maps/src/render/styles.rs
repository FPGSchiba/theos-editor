//! How the people's renders draw each kind and type of feature: one table per kind. Colours and
//! shapes follow 0.3.0's `render_terrain.py`; a type no table knows draws as a plain circle.

use super::canvas::LineStyle;
use super::marks::{Mark, Shape};
use super::palette::Rgb;
use crate::stages::terrain::roads::RoadClass;

/// The ink of outlines and labels.
pub(crate) const INK: Rgb = [0x1d, 0x14, 0x0a];
/// The face of a settlement square.
const SETTLEMENT_FACE: Rgb = [0xf4, 0xea, 0xd2];
/// The outline of a settlement square.
pub(crate) const SETTLEMENT_EDGE: Rgb = [0x3a, 0x2a, 0x14];

/// A mark of unknown type.
const UNKNOWN: Mark = Mark {
    shape: Shape::Circle,
    size: 6,
    face: [0xff, 0xff, 0xff],
    edge: [0, 0, 0],
};

/// A mark in the table's form.
const fn mark(shape: Shape, size: u8, face: Rgb, edge: Rgb) -> Mark {
    Mark {
        shape,
        size,
        face,
        edge,
    }
}

/// The side of a settlement's square in pixels: bigger for bigger classes.
pub(crate) fn settlement_px(ty: &str) -> u8 {
    settlement_px_of(ty).unwrap_or(6)
}

/// The square side of a class above hamlet; `None` for a hamlet and any other type.
fn settlement_px_of(ty: &str) -> Option<u8> {
    match ty {
        "city" => Some(14),
        "town" => Some(11),
        "village" => Some(8),
        _ => None,
    }
}

/// The mark of a settlement of class `ty`.
pub(crate) fn settlement_mark(ty: &str) -> Mark {
    mark(
        Shape::Square,
        settlement_px(ty),
        SETTLEMENT_FACE,
        SETTLEMENT_EDGE,
    )
}

/// The mark of a site of type `ty`.
pub(crate) fn site_mark(ty: &str) -> Mark {
    let black = [0, 0, 0];
    match ty {
        "mill" => mark(Shape::Circle, 6, [0x9f, 0xd0, 0xff], black),
        "mine" => mark(Shape::Plus, 8, [0x8a, 0x5a, 0x2b], black),
        "quarry" => mark(Shape::Square, 6, [0xbd, 0xbd, 0xbd], black),
        "lumber-camp" => mark(Shape::Triangle, 7, [0x2f, 0x7a, 0x2f], black),
        "shrine" => mark(Shape::Star, 10, [0xff, 0xd3, 0x4d], black),
        "watchtower" => mark(Shape::Diamond, 7, [0xb0, 0x3a, 0x2e], black),
        "inn" => mark(Shape::Hexagon, 7, [0xe0, 0xb8, 0x78], black),
        "cave" => mark(Shape::TriangleDown, 7, [0x22, 0x22, 0x22], [255, 255, 255]),
        "camp" => mark(Shape::Cross, 8, [0xff, 0x3b, 0x3b], black),
        "bridge" => mark(Shape::Bar, 9, [0x11, 0x11, 0x11], [0x11, 0x11, 0x11]),
        "ford" => mark(Shape::Dash, 9, [0x1f, 0x4f, 0x7a], [0x1f, 0x4f, 0x7a]),
        _ => UNKNOWN,
    }
}

/// The mark of a point of interest of type `ty`.
pub(crate) fn poi_mark(ty: &str) -> Mark {
    let black = [0, 0, 0];
    match ty {
        "ruin" => mark(Shape::Triangle, 11, [0xc9, 0xa2, 0xe6], black),
        "chime" => mark(Shape::Circle, 11, [0xff, 0x5a, 0x36], black),
        "gate-up" => mark(Shape::Star, 14, [0x38, 0xe1, 0xff], black),
        "gate-down" => mark(Shape::Star, 14, [0xff, 0x4f, 0xd8], black),
        _ => UNKNOWN,
    }
}

/// The mark of a deposit of type `ty`: a circle of `size` pixels in its colour, edged black.
pub(crate) fn resource_mark(ty: &str, size: u8) -> Mark {
    mark(Shape::Circle, size, resource_colour(ty), [0, 0, 0])
}

/// The colour of a deposit of type `ty`.
pub(crate) fn resource_colour(ty: &str) -> Rgb {
    match ty {
        "iron-ore" => [0x8b, 0x45, 0x13],
        "copper-ore" => [0xd2, 0x69, 0x1e],
        "silver-ore" => [0xc0, 0xc0, 0xc0],
        "stone" => [0x9e, 0x9e, 0x9e],
        "clay" => [0xc2, 0x70, 0x3d],
        "timber" => [0x2e, 0x7d, 0x32],
        "peat-reeds" => [0x6d, 0x7f, 0x3a],
        "salt" => [0xf5, 0xf5, 0xf5],
        "resonance-crystal" => [0xb3, 0x88, 0xff],
        "herbs" => [0x8b, 0xc3, 0x4a],
        _ => [0xff, 0xff, 0xff],
    }
}

/// How a road of a class is drawn: an optional dark edge under a line, which may be dashed.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RoadStyle {
    /// The line itself.
    pub line: LineStyle,
    /// The edge under it, wider, when the class has one.
    pub edge: Option<Rgb>,
}

/// The style of `class` (0.3.0's widths, in pixels at two pixels per cell).
pub(crate) fn road_style(class: RoadClass) -> RoadStyle {
    let edge = [0x4a, 0x3b, 0x28];
    let (width_px, colour, edge, dash) = match class {
        RoadClass::Main => (4.0, [0xf7, 0xef, 0xd9], Some(edge), None),
        RoadClass::Road => (3.0, [0xef, 0xe2, 0xc0], Some(edge), None),
        RoadClass::Track => (2.0, [0x6b, 0x5a, 0x3e], None, Some([8.0, 4.0])),
        RoadClass::Trail => (1.5, [0x5b, 0x4a, 0x32], None, Some([2.0, 4.0])),
    };
    RoadStyle {
        line: LineStyle {
            width_px,
            colour,
            dash,
        },
        edge,
    }
}

/// The edge's extra width over the line, pixels.
pub(crate) const ROAD_EDGE_EXTRA_PX: f64 = 2.0;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::TYPES;

    // Final review round 6: a misspelt class arm would fall to the hamlet default unseen.
    #[test]
    fn every_settlement_class_above_hamlet_has_its_own_square() {
        for ty in crate::features::types_of("settlement").filter(|t| *t != "hamlet") {
            assert!(settlement_px_of(ty).is_some(), "{ty}");
        }
    }

    #[test]
    fn every_type_has_its_own_style() {
        for &(kind, ty) in TYPES {
            match kind {
                "site" => assert_ne!(site_mark(ty), UNKNOWN, "{ty}"),
                "poi" => assert_ne!(poi_mark(ty), UNKNOWN, "{ty}"),
                "resource" => assert_ne!(resource_colour(ty), [255; 3], "{ty}"),
                // Final review round 4, class K: a new kind needs a style rule here.
                "settlement" => assert!(settlement_px(ty) > 0, "{ty}"),
                "road" | "water" => {}
                other => panic!("the kind `{other}` has no style rule"),
            }
        }
        assert_eq!(site_mark("nothing"), UNKNOWN);
        assert!(settlement_px("city") > settlement_px("town"));
        assert!(settlement_px("hamlet") < settlement_px("village"));
        assert!(
            road_style(RoadClass::Main).line.width_px > road_style(RoadClass::Trail).line.width_px
        );
    }
}
