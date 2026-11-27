//! The room program of an interior (`[[stage.room]]`) and the queue rules draw rooms from.

use std::fmt;

use serde::de::{self, Deserializer, SeqAccess, Visitor};
use serde::{Deserialize, Serialize};

use crate::rng::StageRng;
use crate::rooms::{Role, Room, Shape};
use crate::stage::ParamError;

/// How many: `count = n` or `count = [min, max]`, drawn once.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum CountSpec {
    /// Exactly this many.
    Fixed(u32),
    /// From `min` to `max`, both included.
    Range(u32, u32),
}

impl Default for CountSpec {
    /// One.
    fn default() -> Self {
        Self::Fixed(1)
    }
}

impl CountSpec {
    /// The largest count the spec can give.
    #[must_use]
    pub fn max(self) -> u32 {
        match self {
            Self::Fixed(n) | Self::Range(_, n) => n,
        }
    }

    /// Draws the count: a fixed count draws nothing; a range draws one value from `rng`.
    pub fn draw(self, rng: &mut StageRng) -> u32 {
        match self {
            Self::Fixed(n) => n,
            Self::Range(min, max) => {
                let span = u64::from(max.saturating_sub(min)) + 1;
                min.saturating_add(u32::try_from(rng.below(span)).unwrap_or(0))
            }
        }
    }

    /// Refuses a range whose `min` exceeds its `max`.
    ///
    /// # Errors
    /// [`ParamError`] of `stage` at `path`.
    pub fn check(self, stage: &str, path: &str) -> Result<(), ParamError> {
        match self {
            Self::Range(min, max) if min > max => Err(ParamError::at(
                stage,
                path,
                format!("the range [{min}, {max}] must have min <= max"),
            )),
            _ => Ok(()),
        }
    }
}

/// What a count looks like, for error messages.
const COUNT_SHAPE: &str = "a count: a whole number n or a range [min, max]";

/// Reads `n` or `[min, max]` with a message that names the count (an untagged enum would only
/// say that no variant matched).
struct CountVisitor;

impl<'de> Visitor<'de> for CountVisitor {
    type Value = CountSpec;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(COUNT_SHAPE)
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<CountSpec, E> {
        u32::try_from(v)
            .map(CountSpec::Fixed)
            .map_err(|_| E::custom(format!("{v} is too large for {COUNT_SHAPE}")))
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> Result<CountSpec, E> {
        u64::try_from(v)
            .map_err(|_| E::custom(format!("{v} is negative; expected {COUNT_SHAPE}")))
            .and_then(|v| self.visit_u64(v))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<CountSpec, A::Error> {
        let wrong = || de::Error::custom(format!("expected {COUNT_SHAPE}"));
        let min: u32 = seq.next_element()?.ok_or_else(wrong)?;
        let max: u32 = seq.next_element()?.ok_or_else(wrong)?;
        if seq.next_element::<de::IgnoredAny>()?.is_some() {
            return Err(wrong());
        }
        Ok(CountSpec::Range(min, max))
    }
}

impl<'de> Deserialize<'de> for CountSpec {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_any(CountVisitor)
    }
}

/// The default footprint.
fn one_by_one() -> (u8, u8) {
    (1, 1)
}

/// The default shape.
fn rect() -> Shape {
    Shape::Rect
}

/// One room type of the program, as a `[[stage.room]]` entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgramRoom {
    /// The room type (`type` in the recipe), `^[a-z][a-z0-9-]*$`.
    #[serde(rename = "type")]
    pub ty: String,
    /// The role its rooms take.
    pub role: Role,
    /// How many rooms of the type; 1 when left out.
    #[serde(default)]
    pub count: CountSpec,
    /// The lattice footprint `[rows, cols]`; `[1, 1]` when left out.
    #[serde(default = "one_by_one")]
    pub footprint: (u8, u8),
    /// The outline; `rect` when left out.
    #[serde(default = "rect")]
    pub shape: Shape,
    /// Whether its rooms lie behind a lock.
    #[serde(default)]
    pub locked: bool,
}

/// Checks the program: room types match `^[a-z][a-z0-9-]*$`, ranges have `min <= max`, and
/// both sides of a footprint are at least 1. Paths read `room[i].<key>`.
///
/// # Errors
/// [`ParamError`] of `stage` for the first entry that breaks a rule.
pub fn check_program(stage: &str, program: &[ProgramRoom]) -> Result<(), ParamError> {
    for (i, room) in program.iter().enumerate() {
        let mut chars = room.ty.chars();
        let ty_ok = chars.next().is_some_and(|c| c.is_ascii_lowercase())
            && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
        if !ty_ok {
            return Err(ParamError::at(
                stage,
                format!("room[{i}].type"),
                "must match ^[a-z][a-z0-9-]*$",
            ));
        }
        room.count.check(stage, &format!("room[{i}].count"))?;
        if room.footprint.0 == 0 || room.footprint.1 == 0 {
            return Err(ParamError::at(
                stage,
                format!("room[{i}].footprint"),
                "both sides must be at least 1",
            ));
        }
    }
    Ok(())
}

/// The rooms of the program still to be placed, in program order.
///
/// [`ProgramQueue::take`] hands out the first entry of a role with rooms left. When the program
/// has used a role up, `path` and `leaf` get a generic room (type `path` or `leaf`, spec §5.6);
/// `start`, `goal` and `exit` have no generic room, so the queue records the role in
/// [`ProgramQueue::exhausted`] and the engine fails the run with
/// `MissionError::ProgramExhausted`: a recipe names the type of its entrance, goal and exits.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProgramQueue {
    /// The program entries and how many rooms each has left.
    entries: Vec<(ProgramRoom, u32)>,
    /// The first role asked for after the program had used it up, without a generic room.
    exhausted: Option<Role>,
}

impl ProgramQueue {
    /// The queue of `program`. Each range count is drawn from `rng`, in program order; the
    /// mission stage builds the queue before its first rule entry draws its count.
    #[must_use]
    pub fn new(program: &[ProgramRoom], rng: &mut StageRng) -> Self {
        let entries = program
            .iter()
            .map(|room| (room.clone(), room.count.draw(rng)))
            .collect();
        Self {
            entries,
            exhausted: None,
        }
    }

    /// The program entry the next [`ProgramQueue::take`] of `role` uses, if any is left; a rule
    /// reads its footprint before placing the room.
    #[must_use]
    pub fn peek(&self, role: Role) -> Option<&ProgramRoom> {
        self.entries
            .iter()
            .find(|(room, left)| room.role == role && *left > 0)
            .map(|(room, _)| room)
    }

    /// The footprint of the room type `ty`, from its first program entry, whether or not rooms
    /// of it are left; `None` for a type the program does not name (a generic room). The
    /// `merge_hall` rule grows a placed room to it.
    #[must_use]
    pub fn footprint(&self, ty: &str) -> Option<(u8, u8)> {
        self.entries
            .iter()
            .find(|(room, _)| room.ty == ty)
            .map(|(room, _)| room.footprint)
    }

    /// Takes the next room of `role`, without cells: from the program in order, else a generic
    /// room (see the type's docs).
    pub fn take(&mut self, role: Role) -> Room {
        match self.take_first(role, |_| true) {
            Some(room) => room,
            None => self.generic(role),
        }
    }

    /// Takes the next room of `role` whose `locked` flag equals `locked`: from the program in
    /// order, else a generic room with that flag. The `leaf` rule takes open rooms only, so it
    /// never uses up the `locked` rooms the `locked_leaf` rule places.
    pub fn take_with_lock(&mut self, role: Role, locked: bool) -> Room {
        match self.take_first(role, |room| room.locked == locked) {
            Some(room) => room,
            None => Room {
                locked,
                ..self.generic(role)
            },
        }
    }

    /// Takes the first program room of `role` with rooms left that `fits`, if any.
    fn take_first(&mut self, role: Role, fits: impl Fn(&ProgramRoom) -> bool) -> Option<Room> {
        let (room, left) = self
            .entries
            .iter_mut()
            .find(|(room, left)| room.role == role && *left > 0 && fits(room))?;
        *left -= 1;
        Some(Room {
            ty: room.ty.clone(),
            role,
            cells: Vec::new(),
            shape: room.shape,
            keys: Vec::new(),
            locked: room.locked,
        })
    }

    /// The generic room of `role`; records a used-up `start`, `goal` or `exit`.
    fn generic(&mut self, role: Role) -> Room {
        let ty = match role {
            Role::Path => "path",
            Role::Leaf => "leaf",
            Role::Start => "start",
            Role::Goal => "goal",
            Role::Exit => "exit",
        };
        if !matches!(role, Role::Path | Role::Leaf) {
            self.exhausted.get_or_insert(role);
        }
        Room {
            ty: ty.to_owned(),
            role,
            cells: Vec::new(),
            shape: Shape::Rect,
            keys: Vec::new(),
            locked: false,
        }
    }

    /// The first role a rule asked for after the program had used it up and that has no
    /// generic room.
    #[must_use]
    pub fn exhausted(&self) -> Option<Role> {
        self.exhausted
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::StageRng;
    use crate::rooms::{Role, Shape};

    fn room(ty: &str, role: Role, count: CountSpec) -> ProgramRoom {
        ProgramRoom {
            ty: ty.to_owned(),
            role,
            count,
            footprint: (1, 1),
            shape: Shape::Rect,
            locked: false,
        }
    }

    // AC: CE-80.3
    #[test]
    fn program_rooms_read_from_toml_with_defaults() {
        let full: ProgramRoom = toml::from_str(
            "type = \"crypt\"\nrole = \"leaf\"\ncount = [1, 3]\nfootprint = [2, 2]\n\
             shape = \"round\"\nlocked = true",
        )
        .unwrap();
        assert_eq!(
            full,
            ProgramRoom {
                ty: "crypt".to_owned(),
                role: Role::Leaf,
                count: CountSpec::Range(1, 3),
                footprint: (2, 2),
                shape: Shape::Round,
                locked: true,
            }
        );
        let short: ProgramRoom = toml::from_str("type = \"nave\"\nrole = \"path\"").unwrap();
        assert_eq!(short, room("nave", Role::Path, CountSpec::Fixed(1)));
        let fixed: ProgramRoom =
            toml::from_str("type = \"nave\"\nrole = \"path\"\ncount = 4").unwrap();
        assert_eq!(fixed.count, CountSpec::Fixed(4));
    }

    // AC: CE-80.3
    #[test]
    fn counts_refuse_other_shapes() {
        for text in [
            "count = -1",
            "count = [1]",
            "count = [1, 2, 3]",
            "count = \"two\"",
        ] {
            let err = toml::from_str::<toml::Table>(text)
                .unwrap()
                .remove("count")
                .unwrap()
                .try_into::<CountSpec>()
                .unwrap_err();
            assert!(err.to_string().contains("count"), "{text}: {err}");
        }
    }

    // AC: CE-80.3
    #[test]
    fn counts_draw_in_their_range() {
        let mut rng = StageRng::for_key(3, "count");
        assert_eq!(CountSpec::Fixed(4).draw(&mut rng), 4);
        assert_eq!(CountSpec::Fixed(4).max(), 4);
        assert_eq!(CountSpec::Range(2, 5).max(), 5);
        for _ in 0..50 {
            assert!((2..=5).contains(&CountSpec::Range(2, 5).draw(&mut rng)));
        }
        assert!(CountSpec::Range(3, 2).check("mission", "count").is_err());
        assert!(CountSpec::Range(2, 2).check("mission", "count").is_ok());
    }

    // AC: CE-80.3
    #[test]
    fn the_queue_takes_in_program_order_then_makes_generic_rooms() {
        let program = [
            room("crypt", Role::Leaf, CountSpec::Fixed(1)),
            room("nave", Role::Path, CountSpec::Fixed(1)),
            room("vault", Role::Leaf, CountSpec::Fixed(1)),
        ];
        let mut rng = StageRng::for_key(1, "program");
        let mut q = ProgramQueue::new(&program, &mut rng);
        assert_eq!(q.peek(Role::Leaf).map(|r| r.ty.as_str()), Some("crypt"));
        assert_eq!(q.take(Role::Leaf).ty, "crypt");
        assert_eq!(q.take(Role::Leaf).ty, "vault");
        let generic = q.take(Role::Leaf);
        assert_eq!((generic.ty.as_str(), generic.role), ("leaf", Role::Leaf));
        assert!(generic.cells.is_empty() && generic.keys.is_empty());
        assert_eq!(q.take(Role::Path).ty, "nave");
        assert_eq!(q.take(Role::Path).ty, "path");
        assert_eq!(q.exhausted(), None);
    }

    // AC: CE-80.3
    #[test]
    fn a_used_up_start_goal_or_exit_is_recorded() {
        let program = [room("porch", Role::Start, CountSpec::Fixed(1))];
        let mut rng = StageRng::for_key(1, "program");
        let mut q = ProgramQueue::new(&program, &mut rng);
        let start = q.take(Role::Start);
        assert_eq!((start.ty.as_str(), start.locked), ("porch", false));
        assert_eq!(q.exhausted(), None);
        let _ = q.take(Role::Start);
        assert_eq!(q.exhausted(), Some(Role::Start));
    }

    // AC: CE-80.3
    #[test]
    fn the_queue_copies_shape_and_lock() {
        let mut vault = room("vault", Role::Leaf, CountSpec::Range(2, 2));
        vault.shape = Shape::Ragged;
        vault.locked = true;
        let mut rng = StageRng::for_key(1, "program");
        let mut q = ProgramQueue::new(&[vault], &mut rng);
        for _ in 0..2 {
            let r = q.take(Role::Leaf);
            assert_eq!((r.shape, r.locked), (Shape::Ragged, true));
        }
        assert_eq!(q.take(Role::Leaf).ty, "leaf");
    }

    // AC: CE-80.2
    #[test]
    fn take_with_lock_keeps_open_and_locked_rooms_apart() {
        let mut vault = room("vault", Role::Leaf, CountSpec::Fixed(1));
        vault.locked = true;
        let program = [vault, room("crypt", Role::Leaf, CountSpec::Fixed(1))];
        let mut rng = StageRng::for_key(1, "program");
        let mut q = ProgramQueue::new(&program, &mut rng);
        assert_eq!(q.take_with_lock(Role::Leaf, false).ty, "crypt");
        let open = q.take_with_lock(Role::Leaf, false);
        assert_eq!((open.ty.as_str(), open.locked), ("leaf", false));
        let locked = q.take_with_lock(Role::Leaf, true);
        assert_eq!((locked.ty.as_str(), locked.locked), ("vault", true));
        let generic = q.take_with_lock(Role::Leaf, true);
        assert_eq!((generic.ty.as_str(), generic.locked), ("leaf", true));
        assert_eq!(q.exhausted(), None);
    }

    // AC: CE-80.3
    #[test]
    fn check_program_refuses_bad_rooms() {
        let ok = room("crypt", Role::Leaf, CountSpec::Fixed(1));
        assert!(check_program("mission", std::slice::from_ref(&ok)).is_ok());
        let mut bad_type = ok.clone();
        bad_type.ty = "Crypt".to_owned();
        let mut bad_count = ok.clone();
        bad_count.count = CountSpec::Range(3, 1);
        let mut bad_footprint = ok.clone();
        bad_footprint.footprint = (0, 1);
        for (r, path) in [
            (bad_type, "room[1].type"),
            (bad_count, "room[1].count"),
            (bad_footprint, "room[1].footprint"),
        ] {
            let err = check_program("mission", &[ok.clone(), r]).unwrap_err();
            assert_eq!(err.parameter.as_deref(), Some(path), "{err}");
        }
    }
}
