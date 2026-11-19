//! The places roads join, settlements and gates, and the class of a link between two of them
//! (0.3.0's rule, `features.py::roads`).

use super::RoadClass;
use crate::features::{Feature, FeatureHandle, FeatureSet, Geometry, Prop};
use crate::world::MapFrame;

/// A settlement or gate the network joins.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Place {
    /// The feature.
    pub handle: FeatureHandle,
    /// Its point, m.
    pub at: [f64; 2],
    /// The cell holding the point.
    pub cell: usize,
    /// 0.3.0's rank: hamlet 1, village 2, town 3, city 4; a gate ranks as a town.
    pub rank: u8,
    /// Whether it is a settlement (not a gate).
    pub settlement: bool,
    /// How warnings name it.
    pub label: String,
}

/// The rank of a feature roads join; `None` for any other feature. A settlement of a type 0.3.0
/// did not know ranks as a town, as 0.3.0 had it.
fn rank(f: &Feature) -> Option<u8> {
    match (f.kind, f.ty.as_str()) {
        ("settlement", "hamlet") => Some(1),
        ("settlement", "village") => Some(2),
        ("settlement", "city") => Some(4),
        ("settlement", _) | ("poi", "gate-up" | "gate-down") => Some(3),
        _ => None,
    }
}

/// How a warning names the feature `f` with handle `h`: a settlement by its type and name, any
/// other feature by its type and handle.
pub(super) fn label(f: &Feature, h: FeatureHandle) -> String {
    match f.props.get("name") {
        Some(Prop::Text(name)) if f.kind == "settlement" => format!("{} {name}", f.ty),
        _ => format!("{} #{}", f.ty, h.0),
    }
}

/// The point features of `set` that pass `keep`, with their handles, in handle order.
fn points<'a>(
    set: &'a FeatureSet,
    keep: impl Fn(&Feature) -> bool + 'a,
) -> impl Iterator<Item = (FeatureHandle, &'a Feature, [f64; 2])> + 'a {
    set.iter().filter_map(move |(h, f)| match f.geometry {
        Geometry::Point(at) if keep(f) => Some((h, f, at)),
        _ => None,
    })
}

/// The settlements and gates of `set`, in handle order.
pub(super) fn places(set: &FeatureSet, frame: &MapFrame) -> Vec<Place> {
    points(set, |f| rank(f).is_some())
        .map(|(handle, f, at)| Place {
            handle,
            at,
            cell: frame.index_at_m(at),
            rank: rank(f).unwrap_or(0),
            settlement: f.kind == "settlement",
            label: label(f, handle),
        })
        .collect()
}

/// The ruins of `set` with their cells and labels, in handle order.
pub(super) fn ruins(set: &FeatureSet, frame: &MapFrame) -> Vec<(FeatureHandle, usize, String)> {
    points(set, |f| f.kind == "poi" && f.ty == "ruin")
        .map(|(h, f, at)| (h, frame.index_at_m(at), label(f, h)))
        .collect()
}

/// The class of a link between `a` and `b`, from the lesser rank: 3 or more a main road, 2 a
/// road, else a track.
pub(super) fn class_of(a: &Place, b: &Place) -> RoadClass {
    match a.rank.min(b.rank) {
        3.. => RoadClass::Main,
        2 => RoadClass::Road,
        _ => RoadClass::Track,
    }
}

/// Whether a link between `a` and `b` may join as a loop: both are settlements of class village
/// or higher.
pub(super) fn may_loop(a: &Place, b: &Place) -> bool {
    a.settlement && b.settlement && a.rank.min(b.rank) >= 2
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    fn feature(kind: &'static str, ty: &str, name: Option<&str>) -> Feature {
        let mut props = BTreeMap::new();
        if let Some(n) = name {
            props.insert("name".to_owned(), Prop::Text(n.to_owned()));
        }
        Feature {
            kind,
            ty: ty.to_owned(),
            geometry: Geometry::Point([150.0, 250.0]),
            props,
        }
    }

    #[test]
    fn the_class_follows_the_lesser_end_and_gates_rank_as_towns() {
        let frame = MapFrame {
            rows: 4,
            cols: 4,
            cell_m: 100.0,
        };
        let mut set = FeatureSet::default();
        for (kind, ty) in [
            ("settlement", "city"),
            ("settlement", "town"),
            ("settlement", "village"),
            ("settlement", "hamlet"),
            ("poi", "gate-down"),
            ("poi", "ruin"),
            ("poi", "chime"),
            ("settlement", "market"),
        ] {
            set.add(feature(kind, ty, Some("Fen")));
        }
        let mut line = feature("settlement", "town", None);
        line.geometry = Geometry::Line(vec![[0.0, 0.0, 0.0]]);
        set.add(line);
        let p = places(&set, &frame);
        let ranks: Vec<u8> = p.iter().map(|p| p.rank).collect();
        assert_eq!(ranks, [4, 3, 2, 1, 3, 3]);
        assert_eq!(p[0].cell, 2 * 4 + 1);
        assert_eq!(p[2].label, "village Fen");
        assert_eq!(p[4].label, "gate-down #4");
        assert!(!p[4].settlement && p[3].settlement);
        let class = |a: usize, b: usize| class_of(&p[a], &p[b]);
        assert_eq!(class(0, 1), RoadClass::Main);
        assert_eq!(class(0, 4), RoadClass::Main);
        assert_eq!(class(1, 2), RoadClass::Road);
        assert_eq!(class(2, 2), RoadClass::Road);
        assert_eq!(class(0, 3), RoadClass::Track);
        assert!(may_loop(&p[1], &p[2]) && !may_loop(&p[1], &p[3]) && !may_loop(&p[1], &p[4]));
        let r = ruins(&set, &frame);
        assert_eq!(r.len(), 1);
        assert_eq!((r[0].0, r[0].2.as_str()), (FeatureHandle(5), "ruin #5"));
    }
}
