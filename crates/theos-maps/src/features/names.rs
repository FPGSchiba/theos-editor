//! Generated place names: 0.3.0's syllable lists, or a recipe's own list.
//!
//! Generated names are not canon (spec §5.10).

use std::collections::BTreeSet;

use crate::rng::StageRng;

/// The first syllables of a generated name.
const PRE: [&str; 20] = [
    "Loom", "Spindle", "Weft", "Warp", "Shuttle", "Thread", "Distaff", "Bobbin", "Hearth", "Ash",
    "Fen", "Kell", "Mere", "Thorn", "Rush", "Oak", "Wen", "Cold", "Hal", "Brin",
];

/// The last syllables of a generated name.
const SUF: [&str; 16] = [
    "ford", "mere", "stead", "wick", "holm", "ley", "thorpe", "well", "dale", "brook", "cross",
    "haven", "moor", "fold", "by", "hithe",
];

/// The tries at a fresh syllable name before numbering (0.3.0's 50).
const TRIES: usize = 50;

/// One of `list` that `used` does not hold yet, drawn at random.
fn from_list(rng: &mut StageRng, used: &BTreeSet<String>, list: &[String]) -> Option<String> {
    let free: Vec<&String> = list.iter().filter(|n| !used.contains(*n)).collect();
    let pick = usize::try_from(rng.below(u64::try_from(free.len()).unwrap_or(0))).ok()?;
    free.get(pick).map(|n| (*n).clone())
}

/// A syllable name, `Place<n>` after [`TRIES`] collisions.
fn from_syllables(rng: &mut StageRng, used: &BTreeSet<String>) -> String {
    let mut syllable = |list: &'static [&'static str]| {
        let i = usize::try_from(rng.below(u64::try_from(list.len()).unwrap_or(0))).unwrap_or(0);
        list.get(i).copied().unwrap_or_default()
    };
    for _ in 0..TRIES {
        let name = format!("{}{}", syllable(&PRE), syllable(&SUF));
        if !used.contains(&name) {
            return name;
        }
    }
    format!("Place{}", used.len())
}

/// A name that `used` does not hold, which it now holds.
///
/// With `list`, the name is drawn from the entries not used yet; once they are spent, or without
/// a list, it is built from syllables. Draws: one per list pick, two per syllable try.
pub fn generated_name(
    rng: &mut StageRng,
    used: &mut BTreeSet<String>,
    list: Option<&[String]>,
) -> String {
    let name = list
        .and_then(|l| from_list(rng, used, l))
        .unwrap_or_else(|| from_syllables(rng, used));
    used.insert(name.clone());
    name
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rng() -> StageRng {
        StageRng::for_key(7, "names")
    }

    #[test]
    fn names_are_unique_and_come_from_the_recipe_list_when_given() {
        let list: Vec<String> = ["Alder", "Birch", "Cedar"].map(String::from).to_vec();
        let mut used = BTreeSet::new();
        let mut r = rng();
        let drawn: Vec<String> = (0..3)
            .map(|_| generated_name(&mut r, &mut used, Some(&list)))
            .collect();
        let mut sorted = drawn.clone();
        sorted.sort();
        assert_eq!(sorted, list, "each list name exactly once");
        assert_eq!(used.len(), 3);
        // The list is spent: the next name is a syllable name and still unique.
        let next = generated_name(&mut r, &mut used, Some(&list));
        assert!(!list.contains(&next) && !next.is_empty());
        assert_eq!(used.len(), 4);
    }

    #[test]
    fn names_skip_what_is_used_already() {
        let list: Vec<String> = ["Alder", "Birch"].map(String::from).to_vec();
        let mut used: BTreeSet<String> = ["Alder".to_owned()].into();
        assert_eq!(generated_name(&mut rng(), &mut used, Some(&list)), "Birch");
    }

    #[test]
    fn syllable_names_are_unique_and_deterministic() {
        let draw = |seed| {
            let mut used = BTreeSet::new();
            let mut r = StageRng::for_key(seed, "names");
            let names: Vec<String> = (0..60)
                .map(|_| generated_name(&mut r, &mut used, None))
                .collect();
            (names, used)
        };
        let (a, used) = draw(1);
        assert_eq!(used.len(), 60, "unique");
        assert!(
            a.iter()
                .all(|n| n.chars().next().is_some_and(char::is_uppercase))
        );
        assert_eq!(a, draw(1).0);
        assert_ne!(a, draw(2).0);
    }

    #[test]
    fn a_spent_syllable_space_falls_back_to_numbered_places() {
        let mut used = BTreeSet::new();
        for pre in PRE {
            for suf in SUF {
                used.insert(format!("{pre}{suf}"));
            }
        }
        let total = used.len();
        let name = generated_name(&mut rng(), &mut used, None);
        assert_eq!(name, format!("Place{total}"));
        // The fallback is recorded too (0.3.0 forgot to), so the next one differs.
        assert_eq!(used.len(), total + 1);
        let again = generated_name(&mut rng(), &mut used, None);
        assert_eq!(again, format!("Place{}", total + 1));
    }
}
