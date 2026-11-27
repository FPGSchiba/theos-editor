//! The feature list the stages build: handles, geometry, properties (spec §5.10).
//!
//! While the pipeline runs, a feature is named by its [`FeatureHandle`], its index in the list.
//! The runner assigns the `<type>-<n>` ids afterwards, so a promotion ([`FeatureSet::set_type`])
//! renames no reference.

use std::any::Any;
use std::collections::BTreeMap;

use serde::Serialize;

use crate::kernels::num::to_u32;
use crate::world::{ValueType, WorldValue};

/// A reference to a feature of a [`FeatureSet`]: its position in insertion order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct FeatureHandle(pub u32);

/// The geometry of a feature, in metres.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum Geometry {
    /// A point `[x, y]`.
    Point([f64; 2]),
    /// A line of vertices `[x, y, z]`.
    Line(Vec<[f64; 3]>),
    /// A polygon: rings of `[x, y]`, the first the outline.
    Polygon(Vec<Vec<[f64; 2]>>),
}

/// The value of a feature property.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum Prop {
    /// A number.
    Num(f64),
    /// An integer.
    Int(i64),
    /// Text.
    Text(String),
    /// A flag.
    Bool(bool),
    /// A list of numbers (a per-vertex attribute).
    Nums(Vec<f64>),
    /// A point in metres.
    At([f64; 2]),
    /// A reference to another feature.
    Ref(FeatureHandle),
    /// References to other features.
    Refs(Vec<FeatureHandle>),
    /// A reference to a road: its index in the world value `road_network`. Roads are not in the
    /// feature list; the features output resolves this to the road's id, like a [`Prop::Ref`].
    Road(u32),
}

/// A feature: a kind, a type, a geometry and properties.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Feature {
    /// The kind, such as `settlement` (`docs/map-package.md` §5).
    pub kind: &'static str,
    /// The type, such as `hamlet`; a promotion changes it.
    pub ty: String,
    /// The geometry in metres.
    pub geometry: Geometry,
    /// The properties by name.
    pub props: BTreeMap<String, Prop>,
}

impl Feature {
    /// A point feature of `kind` and type `ty` at `at` (metres) with the properties `props`.
    #[must_use]
    pub fn point(
        kind: &'static str,
        ty: &str,
        at: [f64; 2],
        props: BTreeMap<String, Prop>,
    ) -> Self {
        Self {
            kind,
            ty: ty.to_owned(),
            geometry: Geometry::Point(at),
            props,
        }
    }

    /// A `site` of type `ty` at the point `at` (metres) with the properties `props`: fords and
    /// bridges of `roads`, and the sites of `sites`.
    #[must_use]
    pub fn site<const K: usize>(ty: &str, at: [f64; 2], props: [(&str, Prop); K]) -> Self {
        let props = props.into_iter().map(|(k, v)| (k.to_owned(), v)).collect();
        Self::point("site", ty, at, props)
    }

    /// The point of a point feature, in metres; `None` for a line or a polygon.
    #[must_use]
    pub fn point_at(&self) -> Option<[f64; 2]> {
        match self.geometry {
            Geometry::Point(p) => Some(p),
            _ => None,
        }
    }

    /// How many of `features` have the type `ty`.
    #[must_use]
    pub fn count_of(features: &[Self], ty: &str) -> usize {
        features.iter().filter(|f| f.ty == ty).count()
    }
}

/// Adds every feature in order, as [`FeatureSet::add`] does.
impl Extend<Feature> for FeatureSet {
    fn extend<I: IntoIterator<Item = Feature>>(&mut self, features: I) {
        for f in features {
            self.add(f);
        }
    }
}

/// The world value `features`: every feature stages have added, in insertion order.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FeatureSet {
    features: Vec<Feature>,
}

impl FeatureSet {
    /// Adds `f` and returns its handle.
    pub fn add(&mut self, f: Feature) -> FeatureHandle {
        // A list never reaches 2^32 features; saturating keeps the call infallible.
        let handle = FeatureHandle(to_u32(self.features.len()));
        self.features.push(f);
        handle
    }

    /// The feature `h`, `None` when no such handle was issued.
    #[must_use]
    pub fn get(&self, h: FeatureHandle) -> Option<&Feature> {
        self.features.get(usize::try_from(h.0).ok()?)
    }

    /// Every feature with its handle, in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = (FeatureHandle, &Feature)> {
        self.features
            .iter()
            .enumerate()
            .map(|(i, f)| (FeatureHandle(to_u32(i)), f))
    }

    /// The features of `kind` with their handles, in insertion order.
    pub fn of_kind<'a>(
        &'a self,
        kind: &'a str,
    ) -> impl Iterator<Item = (FeatureHandle, &'a Feature)> {
        self.iter().filter(move |(_, f)| f.kind == kind)
    }

    /// The points of the features of `kind` and type `ty` (`None`: any type), in metres, in
    /// insertion order; features that are not points are left out.
    #[must_use]
    pub fn points(&self, kind: &str, ty: Option<&str>) -> Vec<[f64; 2]> {
        self.of_kind(kind)
            .filter(|(_, f)| ty.is_none_or(|t| f.ty == t))
            .filter_map(|(_, f)| f.point_at())
            .collect()
    }

    /// How many features have the type `ty`.
    #[must_use]
    pub fn count_type(&self, ty: &str) -> usize {
        Feature::count_of(&self.features, ty)
    }

    /// Changes the type of `h`; `false` when no such handle was issued.
    pub fn set_type(&mut self, h: FeatureHandle, ty: &str) -> bool {
        let Some(feature) = usize::try_from(h.0)
            .ok()
            .and_then(|i| self.features.get_mut(i))
        else {
            return false;
        };
        ty.clone_into(&mut feature.ty);
        true
    }

    /// The number of features.
    #[must_use]
    pub fn len(&self) -> usize {
        self.features.len()
    }

    /// Whether there is no feature.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.features.is_empty()
    }
}

impl WorldValue for FeatureSet {
    fn type_name(&self) -> &'static str {
        Self::NAME
    }
    fn canonical_json(&self) -> Result<serde_json::Value, theos_map_format::FormatError> {
        theos_map_format::json::to_checked_value(&self.features)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl ValueType for FeatureSet {
    const NAME: &'static str = "features";
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feature(kind: &'static str, ty: &str, at: [f64; 2]) -> Feature {
        Feature {
            kind,
            ty: ty.to_owned(),
            geometry: Geometry::Point(at),
            props: BTreeMap::new(),
        }
    }

    #[test]
    fn points_filter_by_kind_and_type() {
        let mut set = FeatureSet::default();
        set.add(feature("settlement", "hamlet", [1.0, 2.0]));
        set.add(feature("poi", "ruin", [3.0, 4.0]));
        set.add(feature("poi", "chime", [5.0, 6.0]));
        let mut line = feature("poi", "ruin", [0.0, 0.0]);
        line.geometry = Geometry::Line(vec![[0.0, 0.0, 0.0]]);
        set.add(line);
        assert_eq!(set.points("poi", Some("ruin")), [[3.0, 4.0]]);
        assert_eq!(set.points("poi", None), [[3.0, 4.0], [5.0, 6.0]]);
        assert_eq!(set.points("resource", None).len(), 0);
    }

    #[test]
    fn handles_follow_insertion_order_and_types_can_change() {
        let mut set = FeatureSet::default();
        let a = set.add(feature("settlement", "hamlet", [1.0, 2.0]));
        let b = set.add(feature("poi", "ruin", [3.0, 4.0]));
        let c = set.add(feature("settlement", "hamlet", [5.0, 6.0]));
        assert_eq!(
            (a, b, c),
            (FeatureHandle(0), FeatureHandle(1), FeatureHandle(2))
        );
        assert_eq!(set.get(b).map(|f| f.ty.as_str()), Some("ruin"));
        assert!(set.get(FeatureHandle(3)).is_none());
        let order: Vec<FeatureHandle> = set.iter().map(|(h, _)| h).collect();
        assert_eq!(order, [a, b, c]);
        let settlements: Vec<FeatureHandle> = set.of_kind("settlement").map(|(h, _)| h).collect();
        assert_eq!(settlements, [a, c]);
        assert_eq!(set.count_type("hamlet"), 2);

        // A promotion keeps the handle and the position in the list.
        assert!(set.set_type(a, "village"));
        assert_eq!(set.count_type("hamlet"), 1);
        assert_eq!(set.count_type("village"), 1);
        assert_eq!(set.get(a).map(|f| f.ty.as_str()), Some("village"));
        assert!(!set.set_type(FeatureHandle(9), "town"));
        assert_eq!(set.len(), 3);
        assert!(!set.is_empty() && FeatureSet::default().is_empty());
    }

    #[test]
    fn a_feature_set_is_a_world_value_with_canonical_json() {
        let mut set = FeatureSet::default();
        let mut f = feature("settlement", "town", [10.0, 20.0]);
        f.props.insert("name".into(), Prop::Text("Fenford".into()));
        f.props.insert("population".into(), Prop::Int(2500));
        f.props.insert("link".into(), Prop::Ref(FeatureHandle(0)));
        set.add(f);
        assert_eq!(set.type_name(), "features");
        assert_eq!(FeatureSet::NAME, "features");
        let json = set.canonical_json().unwrap();
        let first = &json[0];
        assert_eq!(first["kind"], "settlement");
        assert_eq!(first["ty"], "town");
        assert_eq!(first["geometry"]["Point"][1], 20.0);
        assert_eq!(first["props"]["population"]["Int"], 2500);
        assert_eq!(first["props"]["name"]["Text"], "Fenford");
        assert_eq!(set.clone(), set);
        assert!(set.as_any().downcast_ref::<FeatureSet>().is_some());
        assert!(
            WorldValue::as_any_mut(&mut set)
                .downcast_mut::<FeatureSet>()
                .is_some()
        );
    }
}
