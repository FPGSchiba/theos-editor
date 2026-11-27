//! The frame of a map: where a child map lies in its parent, and the frame of a map that has no
//! parent (spec P29 §5.2 item 4, plan rulings L7 to L9).

use super::{ClassFrameValue, ParentContext};
use crate::kernels::num;
use crate::kind;
use crate::recipe::{Recipe, RecipeError, check_cell, grid_frame, invalid};
use crate::stage::MapKind;
use crate::stages::settlement::class::{self, ClassFrame};
use crate::world::MapFrame;

/// The most cells a map side may have: the manifest holds the grid as `u32`.
const MAX_SIDE_CELLS: f64 = 4_294_967_295.0;

/// What decides the frame of a map of a kind; the kind table names one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FrameRule {
    /// Terrain: the recipe `grid` and `size_km`, no origin, no parent.
    Recipe,
    /// Settlement: the window, cell and radius of the class table (L8, L9).
    SettlementClass,
    /// Interior (P34): the size of `grid` and `size_km`, centred on the parent feature.
    RecipeChild,
}

/// The frame of a child map: its grid and where it lies in the parent frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChildFrame {
    /// The position of the child corner in the parent frame, m; a whole number of child cells.
    pub origin_m: [f64; 2],
    /// Rows of the child.
    pub rows: usize,
    /// Columns of the child.
    pub cols: usize,
    /// The child cell edge, m.
    pub cell_m: f64,
}

/// The frame of a child map of `kind` made from `recipe` around the feature of `parent`.
///
/// # Errors
/// [`RecipeError::Invalid`] when the kind has no child frame (terrain), the feature has no
/// anchor, the class is unknown or its table is wrong, or the window fails the cell checks of a
/// plan.
pub fn child_frame(
    kind: MapKind,
    recipe: &Recipe,
    parent: &ParentContext,
) -> Result<ChildFrame, RecipeError> {
    let (rows, cols, cell_m) = match kind::table(kind).frame {
        FrameRule::Recipe => {
            return Err(invalid(
                recipe,
                None,
                format!("a {kind} map has no parent, so no child frame"),
            ));
        }
        FrameRule::SettlementClass => {
            refuse_grid_keys(recipe)?;
            class_grid(recipe, &parent.feature.ty)?
        }
        FrameRule::RecipeChild => {
            let frame = grid_frame(recipe)?;
            (frame.rows, frame.cols, frame.cell_m)
        }
    };
    let anchor = parent
        .anchor_m()
        .map_err(|e| invalid(recipe, Some(&parent.input), e.to_string()))?;
    let size = [num::to_f64(cols) * cell_m, num::to_f64(rows) * cell_m];
    let snap = |axis: usize| {
        num::canonical(num::round_half_even((anchor[axis] - size[axis] / 2.0) / cell_m) * cell_m)
    };
    Ok(ChildFrame {
        origin_m: [snap(0), snap(1)],
        rows,
        cols,
        cell_m,
    })
}

/// The frame of a map that has no parent: the recipe `grid` and `size_km`; for a settlement map
/// the window of the class that the `class` parameter of the `site_terrain` stage names (L8).
///
/// # Errors
/// [`RecipeError::Invalid`] naming the key or stage that is wrong or missing.
pub fn standalone_frame(kind: MapKind, recipe: &Recipe) -> Result<MapFrame, RecipeError> {
    match kind::table(kind).frame {
        FrameRule::Recipe | FrameRule::RecipeChild => grid_frame(recipe),
        FrameRule::SettlementClass => {
            refuse_grid_keys(recipe)?;
            let (rows, cols, cell_m) = class_grid(recipe, &site_class(recipe)?)?;
            Ok(MapFrame { rows, cols, cell_m })
        }
    }
}

/// The frame `describe` shows for a recipe it plans without a parent: like [`standalone_frame`],
/// but a settlement map takes the class of the first of the feature input's `parent_types`.
///
/// # Errors
/// [`RecipeError::Invalid`] naming the key that is wrong or missing.
pub(crate) fn describe_frame(kind: MapKind, recipe: &Recipe) -> Result<MapFrame, RecipeError> {
    match kind::table(kind).frame {
        FrameRule::SettlementClass => {
            refuse_grid_keys(recipe)?;
            let (rows, cols, cell_m) = class_grid(recipe, &first_parent_type(recipe)?)?;
            Ok(MapFrame { rows, cols, cell_m })
        }
        FrameRule::Recipe | FrameRule::RecipeChild => grid_frame(recipe),
    }
}

/// Where the class of a settlement map comes from.
#[derive(Debug, Clone, Copy)]
pub(crate) enum ClassSource<'a> {
    /// The `class` of the recipe's `site_terrain` stage (a map without a parent).
    Site,
    /// The type of the parent feature.
    Parent(&'a str),
    /// The first of the feature input's `parent_types` (`describe`).
    First,
}

/// The class table entry of a settlement map, `None` for the kinds that have none.
///
/// # Errors
/// [`RecipeError::Invalid`] when the class is missing, unknown or its table is wrong.
pub(crate) fn class_frame_of(
    kind: MapKind,
    recipe: &Recipe,
    source: ClassSource<'_>,
) -> Result<Option<ClassFrameValue>, RecipeError> {
    if kind::table(kind).frame != FrameRule::SettlementClass {
        return Ok(None);
    }
    let class = match source {
        ClassSource::Site => site_class(recipe)?,
        ClassSource::Parent(ty) => ty.to_owned(),
        ClassSource::First => first_parent_type(recipe)?,
    };
    let frame = class::resolve(&class, recipe.child.as_ref())
        .map_err(|message| invalid(recipe, Some("child"), message))?;
    Ok(Some(ClassFrameValue { class, frame }))
}

/// The `class` of the `site_terrain` stage of a recipe without a parent.
fn site_class(recipe: &Recipe) -> Result<String, RecipeError> {
    recipe
        .stages
        .iter()
        .find(|t| t.get("id").and_then(toml::Value::as_str) == Some("site_terrain"))
        .and_then(|t| t.get("class"))
        .and_then(toml::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| {
            invalid(
                recipe,
                None,
                "a settlement recipe without a parent needs a `site_terrain` stage                  with a `class`",
            )
        })
}

/// The first parent type of the recipe's feature input.
fn first_parent_type(recipe: &Recipe) -> Result<String, RecipeError> {
    recipe
        .inputs()
        .feature()
        .and_then(|(_, types)| types.first().cloned())
        .ok_or_else(|| invalid(recipe, None, "a settlement recipe needs a `feature` input"))
}

/// A settlement recipe has neither `grid` nor `size_km` (L7): its frame comes from the class.
fn refuse_grid_keys(recipe: &Recipe) -> Result<(), RecipeError> {
    for (key, present) in [
        ("grid", recipe.grid.is_some()),
        ("size_km", recipe.size_km.is_some()),
    ] {
        if present {
            return Err(invalid(
                recipe,
                Some(key),
                format!("a settlement recipe has no `{key}`; its frame comes from the class"),
            ));
        }
    }
    Ok(())
}

/// Rows, columns and cell edge of the window of `class`: `round_half_even(window_m / cell_m)`
/// cells per side (L9).
fn class_grid(recipe: &Recipe, class: &str) -> Result<(usize, usize, f64), RecipeError> {
    let ClassFrame {
        window_m, cell_m, ..
    } = class::resolve(class, recipe.child.as_ref())
        .map_err(|message| invalid(recipe, Some("child"), message))?;
    check_cell(recipe, "child", cell_m, &format!("the class `{class}`"))?;
    let cells = num::round_half_even(window_m / cell_m);
    if !(2.0..=MAX_SIDE_CELLS).contains(&cells) {
        return Err(invalid(
            recipe,
            Some("child"),
            format!(
                "`window_m` {window_m} over `cell_m` {cell_m} of the class `{class}` gives \
                 {cells} cells a side; it must be from 2 to {MAX_SIDE_CELLS}"
            ),
        ));
    }
    let side = num::to_index(cells, usize::MAX);
    if side.checked_mul(side).is_none() {
        return Err(invalid(
            recipe,
            Some("child"),
            format!("a window of {side} x {side} cells overflows the cell count"),
        ));
    }
    Ok((side, side, cell_m))
}
