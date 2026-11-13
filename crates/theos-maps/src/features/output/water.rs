//! Rivers as `water`/`river` `LineString`s and lakes as `water`/`lake` `Polygon`s.

use geojson::{Feature as Gj, GeometryValue};
use serde_json::Value;

use super::{FeaturesError, Polygon, assemble, base, number, numbers, position, ring};
use crate::stages::terrain::hydrology::{LakeShape, RiverLine};

/// The feature of the river line `river` with id `id`: `[x, y, bed]` vertices and the per-vertex
/// `width_m`, `depth_m` and `discharge_m3s`, and the `order`.
///
/// # Errors
/// [`FeaturesError::NonFinite`] for a number that is not finite.
pub(super) fn river(river: &RiverLine, id: &str) -> Result<Gj, FeaturesError> {
    let mut props = base("water", "river");
    props.insert(
        "width_m".to_owned(),
        numbers(&river.width_m, id, "width_m")?,
    );
    props.insert(
        "depth_m".to_owned(),
        numbers(&river.depth_m, id, "depth_m")?,
    );
    props.insert(
        "discharge_m3s".to_owned(),
        numbers(&river.discharge_m3s, id, "discharge_m3s")?,
    );
    props.insert("order".to_owned(), Value::from(river.order));
    let line = river
        .points
        .iter()
        .map(|p| position(p, id))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(assemble(id, props, GeometryValue::new_line_string(line)))
}

/// The feature with id `id` of one `polygon` of `lake`: `surface_m`, `terminal`, `outlet` (when
/// the lake has one) and `depth_max_m`.
///
/// # Errors
/// [`FeaturesError::NonFinite`] for a number that is not finite.
pub(super) fn lake(lake: &LakeShape, id: &str, polygon: &Polygon) -> Result<Gj, FeaturesError> {
    let mut props = base("water", "lake");
    props.insert(
        "surface_m".to_owned(),
        number(lake.surface_m, id, "surface_m")?,
    );
    props.insert("terminal".to_owned(), Value::from(lake.terminal));
    if let Some(at) = lake.outlet {
        props.insert("outlet".to_owned(), numbers(&at, id, "outlet")?);
    }
    props.insert(
        "depth_max_m".to_owned(),
        number(lake.depth_max_m, id, "depth_max_m")?,
    );
    let rings = polygon
        .iter()
        .map(|r| ring(r, id))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(assemble(id, props, GeometryValue::new_polygon(rings)))
}
