//! Reading the feature list for the renders: points, texts and numbers by property name.

use std::collections::BTreeSet;

use crate::features::{Feature, FeatureSet, Geometry, Prop};

/// The point of a point feature, in metres.
pub(crate) fn point(f: &Feature) -> Option<[f64; 2]> {
    match f.geometry {
        Geometry::Point(p) => Some(p),
        _ => None,
    }
}

/// The text property `key`.
pub(crate) fn text<'a>(f: &'a Feature, key: &str) -> Option<&'a str> {
    match f.props.get(key) {
        Some(Prop::Text(t)) => Some(t),
        _ => None,
    }
}

/// The numeric property `key`, whether it is stored as a float or an integer.
pub(crate) fn number(f: &Feature, key: &str) -> Option<f64> {
    match f.props.get(key) {
        Some(Prop::Num(n)) => Some(*n),
        Some(Prop::Int(i)) => Some(crate::kernels::num::to_f64_i64(*i)),
        _ => None,
    }
}

/// The point features of `kind`, in insertion order.
pub(crate) fn points<'a>(
    features: &'a FeatureSet,
    kind: &'a str,
) -> impl Iterator<Item = (&'a Feature, [f64; 2])> {
    features
        .of_kind(kind)
        .filter_map(|(_, f)| point(f).map(|p| (f, p)))
}

/// The types of `kind` that occur, sorted.
pub(crate) fn types_present(features: &FeatureSet, kind: &str) -> BTreeSet<String> {
    features.of_kind(kind).map(|(_, f)| f.ty.clone()).collect()
}
