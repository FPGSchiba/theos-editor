//! The check across the planned stages of an interior (ruling R22): the walls and doors of
//! `space` must fit the lattice of `envelope`, so a recipe that `space` could not build is
//! refused before any stage runs.
//!
//! In raster cells, with the pitch `p`, the jitter `j`, the wall `w` and the door `d`:
//! - every shared wall segment is at least `p − 2j` long and must hold a door with `w` cells
//!   from each end: `p − 2j ≥ d + 2w` (else `space` fails with `NoSharedSegment`);
//! - the narrowest block, `p − 2j` wide, must keep at least 2 x 2 room cells inside a wall on
//!   each side: `p − 2j ≥ 2w + 2` (else `space` fails with `EmptyRoom`). This is
//!   `p − w ≥ w + 2` with the jitter's narrowing included, the bound `space_core` checks block
//!   by block.
//!
//! `space`'s own `check_frame` keeps the door at one or two cells, so `2w + 2 ≥ d + 2w` and the
//! second bound implies the first: one comparison covers both, refused at `wall_m`.

use crate::recipe::PlannedStage;
use crate::stage::{ParamError, TypedStage};
use crate::stages::interior::envelope::{Envelope, EnvelopeParams};
use crate::stages::interior::space::{Space, SpaceParams};
use crate::stages::interior::units::cells;
use crate::world::MapFrame;

/// The parameters of every planned stage with `id`, as `P`, in plan order.
fn params_of<'a, P: 'static>(stages: &'a [PlannedStage], id: &str) -> Vec<&'a P> {
    stages
        .iter()
        .filter(|s| s.id == id)
        .filter_map(|s| s.params.as_any().downcast_ref::<P>())
        .collect()
}

/// Checks that every `space` fits the lattice of the plan's first `envelope`; a recipe without
/// either has nothing to check.
///
/// # Errors
/// [`ParamError`] of `space` at `wall_m` when its walls leave the narrowest block without a
/// 2 x 2 room, and with it every shared wall without room for a door.
pub(crate) fn check_plan(stages: &[PlannedStage], frame: &MapFrame) -> Result<(), ParamError> {
    let Some(env) = params_of::<EnvelopeParams>(stages, Envelope::ID)
        .first()
        .copied()
    else {
        return Ok(());
    };
    let pitch = cells(Envelope::ID, "pitch_m", env.pitch_m, frame)?;
    let jitter = cells(Envelope::ID, "jitter_m", env.jitter_m, frame)?;
    let narrowest = pitch.saturating_sub(jitter.saturating_mul(2));
    for space in params_of::<SpaceParams>(stages, Space::ID) {
        let wall = cells(Space::ID, "wall_m", space.wall_m, frame)?;
        if narrowest < wall.saturating_mul(2).saturating_add(2) {
            return Err(ParamError::at(
                Space::ID,
                "wall_m",
                format!(
                    "walls of {wall} cells leave no 2 x 2 room and no door in the narrowest \
                     block of {narrowest} cells (pitch {pitch} minus twice the jitter {jitter})"
                ),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::recipe::{Planning, RecipeError, RecipeOrigin, parse, plan};
    use crate::stage::Registry;

    /// The stage and parameter `plan` refuses a 48 x 48 interior at 1 m with, `None` when the
    /// plan is made.
    fn refusal(envelope: &str, space: &str) -> Option<(Option<String>, Option<String>)> {
        let text = format!(
            "id = \"fit\"\ntitle = \"Fit\"\nkind = \"interior\"\n\
             grid = [48, 48]\nsize_km = [0.048, 0.048]\n\
             [[stage]]\nid = \"envelope\"\n{envelope}\n\
             [[stage]]\nid = \"mission\"\n\
             [[stage]]\nid = \"space\"\n{space}\n"
        );
        let loaded = parse(&text, RecipeOrigin::Builtin).unwrap();
        match plan(
            &loaded,
            &Registry::builtin(),
            &serde_json::Map::new(),
            Planning::Standalone,
        ) {
            Ok(_) => None,
            Err(RecipeError::Invalid {
                stage, parameter, ..
            }) => Some((stage, parameter)),
            Err(other) => panic!("{other}"),
        }
    }

    #[test]
    fn walls_must_leave_a_room_and_a_door_in_the_narrowest_block() {
        let space_wall = Some((Some("space".to_owned()), Some("wall_m".to_owned())));
        // The narrowest block of pitch 8 and jitter 2 is 4 cells: walls of 1 leave 2 x 2.
        assert_eq!(
            refusal(
                "pitch_m = 8.0\njitter_m = 2.0",
                "wall_m = 1.0\ndoor_m = 2.0"
            ),
            None
        );
        assert_eq!(
            refusal("pitch_m = 8.0\njitter_m = 2.0", "wall_m = 2.0"),
            space_wall
        );
        assert_eq!(
            refusal("pitch_m = 8.0\njitter_m = 2.0", "wall_m = 3.0"),
            space_wall
        );
        // Without jitter, walls of 3 leave 2 x 2 in 8 cells; walls of 4 do not.
        assert_eq!(refusal("pitch_m = 8.0", "wall_m = 3.0"), None);
        assert_eq!(refusal("pitch_m = 8.0", "wall_m = 4.0"), space_wall);
    }
}
