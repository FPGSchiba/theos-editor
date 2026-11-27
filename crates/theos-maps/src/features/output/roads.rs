//! Roads as `road`/`<class>` `LineString`s of cell centres.

use geojson::{Feature as Gj, GeometryValue};
use serde_json::Value;

use super::{FeaturesError, Named, assemble, base, number, position};
use crate::grid::Grid;
use crate::kernels::num::to_usize;
use crate::stages::terrain::roads::{Road, RoadNetwork};
use crate::world::MapFrame;

/// The feature of `road` with id `id`: `class`, `from`, `to`, `length_m`, `width_m` (from the
/// parameters of the `roads` stage) and `grade_max`. A bridge is the straight segment between
/// its banks.
///
/// # Errors
/// [`FeaturesError`] for an end that is no feature or a number that is not finite.
pub(super) fn feature(
    road: &Road,
    id: &str,
    network: &RoadNetwork,
    height: &Grid<f64>,
    frame: &MapFrame,
    named: &Named,
) -> Result<Gj, FeaturesError> {
    let mut props = base("road", road.class.name());
    props.insert("class".to_owned(), Value::from(road.class.name()));
    props.insert(
        "from".to_owned(),
        Value::from(named.feature(road.from, id)?),
    );
    props.insert("to".to_owned(), Value::from(named.feature(road.to, id)?));
    props.insert(
        "length_m".to_owned(),
        number(road.length_m(frame), id, "length_m")?,
    );
    props.insert(
        "width_m".to_owned(),
        number(network.params.width_m.of(road.class), id, "width_m")?,
    );
    props.insert(
        "grade_max".to_owned(),
        number(road.grade_max(height, frame), id, "grade_max")?,
    );
    let line = road
        .cells
        .iter()
        .map(|&cell| position(&frame.index_centre_m(to_usize(cell)), id))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(assemble(id, props, GeometryValue::new_line_string(line)))
}
