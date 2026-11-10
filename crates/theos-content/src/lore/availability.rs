//! Availability of a clue (spec §5.7): its floor and its gate, and when one clue "can be found no
//! later than" another.

use crate::lore::places::{Floor, floor_of};
use crate::lore::{Context, FactError, RecordFacts};

/// An Understanding tier, in rising order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tier {
    /// Understanding T1.
    T1,
    /// Understanding T2.
    T2,
    /// Understanding T3.
    T3,
}

impl Tier {
    /// The text of the tier: `T1`, `T2` or `T3`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Tier::T1 => "T1",
            Tier::T2 => "T2",
            Tier::T3 => "T3",
        }
    }

    /// The tier a `gate` or `tier` value names: `T1`, `T2` or `T3`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Tier> {
        match text {
            "T1" => Some(Tier::T1),
            "T2" => Some(Tier::T2),
            "T3" => Some(Tier::T3),
            _ => None,
        }
    }
}

/// What a clue does to a truth (`moves_to`), in rising order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum State {
    /// The truth is hinted at.
    Hinted,
    /// The truth is suspected.
    Suspected,
    /// The truth is confirmed.
    Confirmed,
}

impl State {
    /// The `moves_to` value of the state.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            State::Hinted => "hinted",
            State::Suspected => "suspected",
            State::Confirmed => "confirmed",
        }
    }

    /// The state a `moves_to` value names: `hinted`, `suspected` or `confirmed`.
    #[must_use]
    pub fn parse(text: &str) -> Option<State> {
        match text {
            "hinted" => Some(State::Hinted),
            "suspected" => Some(State::Suspected),
            "confirmed" => Some(State::Confirmed),
            _ => None,
        }
    }
}

/// When a clue can be found: the lowest floor of its places and its gate. A part is `None` when
/// no valid value gives it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Availability {
    /// The lowest floor among the clue's valid `where` places that have one.
    pub floor: Option<Floor>,
    /// The tier of the clue's valid `gate`.
    pub gate: Option<Tier>,
}

impl Availability {
    /// The availability as a message reads it: "floor 21, gate T2", `?` for an unknown part.
    #[must_use]
    pub fn describe(&self) -> String {
        let floor = self
            .floor
            .as_ref()
            .map_or_else(|| "?".to_owned(), |floor| floor.number.to_string());
        let gate = self.gate.map_or("?", Tier::as_str);
        format!("floor {floor}, gate {gate}")
    }
}

/// The availability of `clue`. Among equal floors the first `where` item counts.
///
/// # Errors
///
/// Fails when the store fails.
pub fn of_clue(ctx: &mut Context<'_>, clue: &RecordFacts) -> Result<Availability, FactError> {
    let mut lowest: Option<Floor> = None;
    for item in clue.items("where") {
        let Some(place) = item.text.as_deref() else {
            continue;
        };
        if let Some(floor) = floor_of(ctx, place)?
            && lowest
                .as_ref()
                .is_none_or(|best| floor.number < best.number)
        {
            lowest = Some(floor);
        }
    }
    Ok(Availability {
        floor: lowest,
        gate: clue
            .text("gate")
            .and_then(|gate| gate.text.as_deref())
            .and_then(Tier::parse),
    })
}

/// Whether `a` can be found no later than `b`: its floor and its gate are each at most `b`'s.
/// `None` when a floor or a gate is unknown.
#[must_use]
pub fn no_later(a: &Availability, b: &Availability) -> Option<bool> {
    let (floor_a, floor_b) = (a.floor.as_ref()?, b.floor.as_ref()?);
    let (gate_a, gate_b) = (a.gate?, b.gate?);
    Some(floor_a.number <= floor_b.number && gate_a <= gate_b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lore::places::tests::FILES;
    use crate::test_support::with_context;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn at(floor: Option<u32>, gate: Option<Tier>) -> Availability {
        Availability {
            floor: floor.map(|number| Floor {
                number,
                place: "p".to_owned(),
            }),
            gate,
        }
    }

    #[test]
    fn availability_is_the_lowest_floor_and_the_gate() -> TestResult {
        let mut files = FILES.to_vec();
        files.push((
            "lore/clues/C-950.md",
            "---\nid: C-950\nkind: clue\ntitle: t\nreveals: T-950\nmoves_to: hinted\nwhere: [synthetic-high, synthetic-nook, synthetic-lost]\ngate: T2\nvector: v\n---\n",
        ));
        files.push((
            "lore/clues/C-951.md",
            "---\nid: C-951\nkind: clue\ntitle: t\nreveals: T-950\nmoves_to: hinted\nwhere: [synthetic-lost]\ngate: T9\nvector: v\n---\n",
        ));
        with_context(&files, |ctx| {
            let found = ctx.records("C-950")?;
            let clue = found.first().ok_or("C-950 missing")?;
            let got = of_clue(ctx, clue)?;
            assert_eq!(
                got.floor,
                Some(Floor {
                    number: 4,
                    place: "f04-synthetic".to_owned()
                })
            );
            assert_eq!(got.gate, Some(Tier::T2));
            let found = ctx.records("C-951")?;
            let clue = found.first().ok_or("C-951 missing")?;
            assert_eq!(of_clue(ctx, clue)?, at(None, None));
            Ok::<_, Box<dyn std::error::Error>>(())
        })??;
        Ok(())
    }

    #[test]
    fn no_later_needs_floor_and_gate() {
        let t1 = Some(Tier::T1);
        let t2 = Some(Tier::T2);
        let t3 = Some(Tier::T3);
        assert_eq!(no_later(&at(Some(4), t1), &at(Some(21), t2)), Some(true));
        assert_eq!(no_later(&at(Some(21), t1), &at(Some(4), t2)), Some(false));
        assert_eq!(no_later(&at(Some(4), t3), &at(Some(21), t2)), Some(false));
        assert_eq!(no_later(&at(Some(4), t2), &at(Some(4), t2)), Some(true));
        assert_eq!(no_later(&at(None, t1), &at(Some(21), t2)), None);
        assert_eq!(no_later(&at(Some(4), t1), &at(None, t2)), None);
        assert_eq!(no_later(&at(Some(4), None), &at(Some(21), t2)), None);
    }

    #[test]
    fn tiers_and_states_parse_their_values() {
        assert_eq!(Tier::parse("T2"), Some(Tier::T2));
        assert_eq!(Tier::parse("t2"), None);
        assert!(Tier::T1 < Tier::T3);
        assert_eq!(State::parse("suspected"), Some(State::Suspected));
        assert_eq!(State::parse("T1"), None);
        assert!(State::Hinted < State::Confirmed);
    }
}
