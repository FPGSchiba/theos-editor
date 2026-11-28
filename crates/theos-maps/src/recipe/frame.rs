//! The frame a recipe's own `grid` and `size_km` give (spec §5.5, check 1 of the plan).

use super::plan::{MAX_CELL_M, MIN_CELL_M};
use super::{Recipe, RecipeError};
use crate::kernels::num;
use crate::world::MapFrame;

/// How far two cell edges may differ, relative, and still count as square.
const SQUARE_TOLERANCE: f64 = 1e-9;

/// A refusal of `recipe` at the recipe key `parameter`, when it names one.
pub(crate) fn invalid(
    recipe: &Recipe,
    parameter: Option<&str>,
    message: impl Into<String>,
) -> RecipeError {
    RecipeError::Invalid {
        recipe: recipe.id.clone(),
        stage: None,
        layer: None,
        parameter: parameter.map(str::to_owned),
        message: message.into(),
    }
}

/// Checks a cell edge against the range every map keeps (`MIN_CELL_M` to `MAX_CELL_M`).
pub(crate) fn check_cell(
    recipe: &Recipe,
    parameter: &str,
    cell_m: f64,
    what: &str,
) -> Result<(), RecipeError> {
    if (MIN_CELL_M..=MAX_CELL_M).contains(&cell_m) {
        return Ok(());
    }
    Err(invalid(
        recipe,
        Some(parameter),
        format!(
            "{what} gives cells of {cell_m} m; the edge must be from {MIN_CELL_M} m to \
             {MAX_CELL_M} m"
        ),
    ))
}

/// The keys `grid` and `size_km` of a terrain or interior recipe: both are needed; then the
/// grid (at least 2 x 2, a cell count that fits a `usize`), the size, square cells and a cell
/// edge from [`MIN_CELL_M`] to [`MAX_CELL_M`].
pub(crate) fn grid_frame(recipe: &Recipe) -> Result<MapFrame, RecipeError> {
    let need = |key: &str| {
        invalid(
            recipe,
            Some(key),
            format!("a {} recipe needs `{key}`", recipe.kind),
        )
    };
    let [rows, cols] = recipe.grid.ok_or_else(|| need("grid"))?;
    let [width, height] = recipe.size_km.ok_or_else(|| need("size_km"))?;
    if rows < 2 || cols < 2 {
        return Err(invalid(
            recipe,
            Some("grid"),
            format!("`grid` must be at least 2 x 2, got {rows} x {cols}"),
        ));
    }
    // Every raster and `MapFrame::cells` multiply the two; the memory budget sees only the product.
    if rows.checked_mul(cols).is_none() {
        return Err(invalid(
            recipe,
            Some("grid"),
            format!("`grid` of {rows} x {cols} overflows the cell count"),
        ));
    }
    // In metres too: a size beyond f64 there would make the cell edge an infinity.
    let usable = |km: f64| km > 0.0 && (km * 1000.0).is_finite();
    if !(usable(width) && usable(height)) {
        return Err(invalid(
            recipe,
            Some("size_km"),
            "`size_km` must be positive and finite, also in metres",
        ));
    }
    let across = width / num::to_f64(cols);
    let down = height / num::to_f64(rows);
    if (across - down).abs() > SQUARE_TOLERANCE * across {
        return Err(invalid(
            recipe,
            Some("size_km"),
            format!(
                "cells must be square: {width} km over {cols} columns is {} m, \
                 {height} km over {rows} rows is {} m",
                across * 1000.0,
                down * 1000.0
            ),
        ));
    }
    // Final review round 3, class B: every metre of a stage becomes cells, areas and slopes
    // through the cell edge, so its range keeps them finite and its conversions bounded. It also
    // refuses a size whose cells underflow to 0, which the square check above cannot see.
    let cell_m = width * 1000.0 / num::to_f64(cols);
    check_cell(recipe, "size_km", cell_m, "`size_km` over `grid`")?;
    Ok(MapFrame { rows, cols, cell_m })
}
