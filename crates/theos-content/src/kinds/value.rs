//! Checking a front-matter value against the type of its field. A scalar is judged by the
//! field's type, never by its spelling (P02's ruling D1).

use super::spec::{FieldSpec, Problem};
use super::types::{FieldType, LENS_ENTRIES};
use crate::text::Span;
use crate::yaml::{EntryValue, List, Map, Scalar, Value};

/// A front-matter value of any shape, borrowed: a field's value or a map entry's value.
#[derive(Clone, Copy, Debug)]
pub enum ValueRef<'a> {
    /// A scalar.
    Scalar(&'a Scalar),
    /// A list.
    List(&'a List),
    /// A map.
    Map(&'a Map),
}

impl<'a> ValueRef<'a> {
    /// Borrows a field's value.
    #[must_use]
    pub fn of(value: &'a Value) -> ValueRef<'a> {
        match value {
            Value::Scalar(scalar) => ValueRef::Scalar(scalar),
            Value::List(list) => ValueRef::List(list),
            Value::Map(map) => ValueRef::Map(map),
        }
    }

    /// Borrows a map entry's value.
    #[must_use]
    pub fn of_entry(value: &'a EntryValue) -> ValueRef<'a> {
        match value {
            EntryValue::Scalar(scalar) => ValueRef::Scalar(scalar),
            EntryValue::List(list) => ValueRef::List(list),
        }
    }

    /// The bytes of the value. A map runs from its first key to the end of its last entry's
    /// lines; an empty map has the empty span at 0.
    #[must_use]
    pub fn span(self) -> Span {
        match self {
            ValueRef::Scalar(scalar) => scalar.span,
            ValueRef::List(list) => list.span,
            ValueRef::Map(map) => match (map.entries.first(), map.entries.last()) {
                (Some(first), Some(last)) => Span::new(first.key_span.start, last.lines.end),
                _ => Span::default(),
            },
        }
    }

    /// How the value reads in a message: the scalar's text, or its shape.
    fn found(self) -> String {
        match self {
            ValueRef::Scalar(scalar) if scalar.as_text().is_empty() => "an empty text".to_owned(),
            ValueRef::Scalar(scalar) => format!("`{}`", scalar.as_text()),
            ValueRef::List(_) => "a list".to_owned(),
            ValueRef::Map(_) => "a map".to_owned(),
        }
    }
}

impl FieldSpec {
    /// Checks `value` against the field's type, and a required list for being empty.
    /// `Id` and `KindName` fields are judged by the id and kind rules, not here.
    #[must_use]
    pub fn check(&self, value: ValueRef<'_>) -> Vec<Problem> {
        let mut problems = self.ty.check(self.key, value);
        let is_list_type = matches!(
            self.ty,
            FieldType::TextList | FieldType::Links(_) | FieldType::NoteIds
        );
        if let (true, true, ValueRef::List(list)) = (self.required, is_list_type, value)
            && problems.is_empty()
            && list.items.is_empty()
        {
            problems.push(problem(
                list.span,
                format!("`{}` must not be empty", self.key),
            ));
        }
        problems
    }
}

impl FieldType {
    /// Checks `value` against this type; `label` names the field in the messages. One problem
    /// per bad list item or map entry, and one for a value of the wrong shape.
    #[must_use]
    pub fn check(self, label: &str, value: ValueRef<'_>) -> Vec<Problem> {
        match self {
            FieldType::Id | FieldType::KindName => Vec::new(),
            FieldType::Text => scalar(label, "a text that is not empty", value, not_empty),
            FieldType::TextList => list(label, "a list of non-empty texts", value, not_empty),
            FieldType::Enum(values) => {
                let what = format!("one of {}", values.join(", "));
                scalar(label, &what, value, |s| values.contains(&s.as_text()))
            }
            FieldType::Integer { min, max } => {
                let what = if min == i64::MIN && max == i64::MAX {
                    "an integer".to_owned()
                } else {
                    format!("an integer from {min} to {max}")
                };
                scalar(label, &what, value, |s| {
                    s.as_int().is_ok_and(|n| (min..=max).contains(&n))
                })
            }
            FieldType::FloorRange => floor_range(label, value),
            FieldType::Link(_) => scalar(label, "a link", value, not_empty),
            FieldType::Links(_) => list(label, "a list of links", value, not_empty),
            FieldType::Url => scalar(label, "a URL", value, |s| is_url(s.as_text())),
            FieldType::True => scalar(label, "true", value, |s| s.as_bool() == Ok(true)),
            FieldType::Generator => {
                scalar(label, "name@version", value, |s| is_generator(s.as_text()))
            }
            FieldType::NoteIds => list(label, "a list of note ids like n12", value, |s| {
                is_note_id(s.as_text())
            }),
            FieldType::BinaryRefs => binary_refs(label, value),
            FieldType::OpenMap => map(label, value).1,
            FieldType::Lens => lens(label, value),
        }
    }
}

fn problem(span: Span, message: String) -> Problem {
    Problem { span, message }
}

fn must_be(label: &str, what: &str, value: ValueRef<'_>) -> Problem {
    problem(
        value.span(),
        format!("`{label}` must be {what}, not {}", value.found()),
    )
}

fn not_empty(scalar: &Scalar) -> bool {
    !scalar.as_text().is_empty()
}

/// A scalar that satisfies `ok`.
fn scalar(
    label: &str,
    what: &str,
    value: ValueRef<'_>,
    ok: impl Fn(&Scalar) -> bool,
) -> Vec<Problem> {
    match value {
        ValueRef::Scalar(s) if ok(s) => Vec::new(),
        _ => vec![must_be(label, what, value)],
    }
}

/// A list whose every item satisfies `ok`; one problem per bad item.
fn list(
    label: &str,
    what: &str,
    value: ValueRef<'_>,
    ok: impl Fn(&Scalar) -> bool,
) -> Vec<Problem> {
    match value {
        ValueRef::List(list) => list
            .items
            .iter()
            .filter(|item| !ok(&item.scalar))
            .map(|item| must_be(label, what, ValueRef::Scalar(&item.scalar)))
            .collect(),
        _ => vec![must_be(label, what, value)],
    }
}

/// The map, or the one problem saying the value is not one.
fn map<'a>(label: &str, value: ValueRef<'a>) -> (Option<&'a Map>, Vec<Problem>) {
    match value {
        ValueRef::Map(map) => (Some(map), Vec::new()),
        _ => (None, vec![must_be(label, "a map", value)]),
    }
}

/// The two integers of a two-item list.
fn floor_pair(list: &List) -> Option<(i64, i64)> {
    match list.items.as_slice() {
        [a, b] => Some((a.scalar.as_int().ok()?, b.scalar.as_int().ok()?)),
        _ => None,
    }
}

fn floor_range(label: &str, value: ValueRef<'_>) -> Vec<Problem> {
    let fits = match value {
        ValueRef::List(items) => {
            floor_pair(items).is_some_and(|(a, b)| 1 <= a && a <= b && b <= 100)
        }
        _ => false,
    };
    if fits {
        Vec::new()
    } else {
        vec![problem(
            value.span(),
            format!("`{label}` must be two integers, first ≤ last, from 1 to 100"),
        )]
    }
}

fn binary_refs(label: &str, value: ValueRef<'_>) -> Vec<Problem> {
    let (found, mut problems) = map(label, value);
    let Some(found) = found else {
        return problems;
    };
    let what = "a path and `sha256:` with 64 lower-case hex digits, joined by @";
    if found.entries.is_empty() {
        problems.push(must_be(label, "a map of at least one entry", value));
    }
    for entry in &found.entries {
        let entry_value = ValueRef::of_entry(&entry.value);
        match entry_value {
            ValueRef::Scalar(s) if is_binary_ref(s.as_text()) => {}
            _ => problems.push(must_be(label, what, entry_value)),
        }
    }
    problems
}

fn lens(label: &str, value: ValueRef<'_>) -> Vec<Problem> {
    let (found, mut problems) = map(label, value);
    for entry in found.map_or(&[][..], |map| map.entries.as_slice()) {
        if let Some(known) = LENS_ENTRIES.iter().find(|known| known.key == entry.key) {
            let entry_label = format!("{label}.{}", entry.key);
            problems.extend(
                known
                    .ty
                    .check(&entry_label, ValueRef::of_entry(&entry.value)),
            );
        }
    }
    problems
}

fn no_space(text: &str) -> bool {
    !text.chars().any(char::is_whitespace)
}

pub(crate) fn is_url(text: &str) -> bool {
    ["http://", "https://"]
        .iter()
        .filter_map(|scheme| text.strip_prefix(scheme))
        .any(|rest| !rest.is_empty())
        && no_space(text)
}

fn is_generator(text: &str) -> bool {
    text.split_once('@').is_some_and(|(name, version)| {
        !name.is_empty() && !version.is_empty() && !version.contains('@')
    }) && no_space(text)
}

fn is_note_id(text: &str) -> bool {
    text.strip_prefix('n')
        .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
}

fn is_binary_ref(text: &str) -> bool {
    text.split_once('@').is_some_and(|(path, digest)| {
        !path.is_empty()
            && digest.strip_prefix("sha256:").is_some_and(|hex| {
                hex.len() == 64 && hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
            })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kinds::{FieldType, Target};
    use crate::test_support::record;

    const STATUS: &[&str] = &["draft", "canon"];
    const TARGETS: &[Target] = &[Target::kind("npc")];
    const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn field(key: &'static str, ty: FieldType, required: bool) -> FieldSpec {
        FieldSpec { key, ty, required }
    }

    /// The problems of `front`'s field `spec.key`, as (text under the span, message).
    fn run(front: &str, spec: FieldSpec) -> Vec<(String, String)> {
        let record = record(front, "");
        let front_matter = record.front_matter().expect("front matter parses");
        let found = front_matter.get(spec.key).expect("field exists");
        spec.check(ValueRef::of(&found.value))
            .into_iter()
            .map(|p| {
                let at = p.span.get(record.text()).expect("span in range");
                (at.to_owned(), p.message)
            })
            .collect()
    }

    /// Asserts that the field has no problem.
    fn ok(front: &str, spec: FieldSpec) {
        assert_eq!(run(front, spec), Vec::<(String, String)>::new());
    }

    fn at(problems: &[(String, String)]) -> Vec<&str> {
        problems.iter().map(|(at, _)| at.as_str()).collect()
    }

    // AC: CE-11.1
    #[test]
    fn texts_and_text_lists() {
        let title = field("title", FieldType::Text, true);
        assert_eq!(at(&run("title: \"\"\n", title)), ["\"\""]);
        assert_eq!(at(&run("title: [a]\n", title)), ["[a]"]);
        ok("title: a\n", title);
        let tags = field("tags", FieldType::TextList, false);
        assert_eq!(at(&run("tags: [a, \"\"]\n", tags)), ["\"\""]);
        let shape = run("tags: a\n", tags);
        assert_eq!(at(&shape), ["a"]);
        assert!(shape[0].1.contains("must be a list"));
        ok("tags: []\n", tags);
        ok("tags:\n  - a\n  - b\n", tags);
    }

    // AC: CE-11.1
    #[test]
    fn enums_read_the_text() {
        let status = field("status", FieldType::Enum(STATUS), true);
        let bad = run("status: Draft\n", status);
        assert_eq!(at(&bad), ["Draft"]);
        assert!(bad[0].1.contains("draft, canon"), "{}", bad[0].1);
        ok("status: \"draft\"\n", status);
        let accuracy = field("accuracy", FieldType::Enum(&["true", "false"]), false);
        ok("accuracy: false\n", accuracy);
        let tier = field("tier", FieldType::Enum(&["T1", "T2", "T3"]), false);
        assert_eq!(at(&run("tier: T4\n", tier)), ["T4"]);
        assert_eq!(at(&run("tier: [T1]\n", tier)), ["[T1]"]);
    }

    // AC: CE-11.1
    #[test]
    fn integers_and_floor_ranges() {
        let number = field("number", FieldType::Integer { min: 1, max: 100 }, true);
        assert_eq!(at(&run("number: \"4\"\n", number)), ["\"4\""]);
        ok("number: 007\n", number);
        assert_eq!(at(&run("number: 0\n", number)), ["0"]);
        let big = run("number: 104\n", number);
        assert_eq!(at(&big), ["104"]);
        assert_eq!(
            big[0].1,
            "`number` must be an integer from 1 to 100, not `104`"
        );
        assert_eq!(at(&run("number: x\n", number)), ["x"]);
        assert_eq!(at(&run("number: [1]\n", number)), ["[1]"]);
        let seed = field(
            "seed",
            FieldType::Integer {
                min: i64::MIN,
                max: i64::MAX,
            },
            false,
        );
        ok("seed: -1\n", seed);
        assert_eq!(
            run("seed: x\n", seed)[0].1,
            "`seed` must be an integer, not `x`"
        );

        let floors = field("floors", FieldType::FloorRange, true);
        let reversed = run("floors: [20, 1]\n", floors);
        assert_eq!(at(&reversed), ["[20, 1]"]);
        assert_eq!(
            reversed[0].1,
            "`floors` must be two integers, first ≤ last, from 1 to 100"
        );
        assert_eq!(at(&run("floors: [1]\n", floors)), ["[1]"]);
        assert_eq!(at(&run("floors: 5\n", floors)), ["5"]);
        assert_eq!(at(&run("floors: [1, \"2\"]\n", floors)), ["[1, \"2\"]"]);
        assert_eq!(at(&run("floors: [0, 2]\n", floors)), ["[0, 2]"]);
        assert_eq!(at(&run("floors: [1, 101]\n", floors)), ["[1, 101]"]);
        ok("floors: [1, 20]\n", floors);
        ok("floors: [7, 7]\n", floors);
    }

    // AC: CE-11.1
    #[test]
    fn links_are_only_shapes() {
        let parent = field("parent", FieldType::Link(TARGETS), false);
        assert_eq!(at(&run("parent: [a]\n", parent)), ["[a]"]);
        assert_eq!(at(&run("parent: \"\"\n", parent)), ["\"\""]);
        ok("parent: a\n", parent);
        let wh = field("where", FieldType::Links(TARGETS), false);
        assert_eq!(at(&run("where: a\n", wh)), ["a"]);
        assert_eq!(at(&run("where: [a, \"\"]\n", wh)), ["\"\""]);
        ok("where: [a, b]\n", wh);
        ok("where: []\n", wh);
        let held = field("held_by", FieldType::Links(TARGETS), true);
        let empty = run("held_by: []\n", held);
        assert_eq!(at(&empty), ["[]"]);
        assert!(empty[0].1.contains("must not be empty"));
        ok("held_by: [a]\n", held);
    }

    // AC: CE-11.1
    #[test]
    fn formats() {
        let source = field("source", FieldType::Url, false);
        assert_eq!(at(&run("source: todo\n", source)), ["todo"]);
        assert_eq!(
            at(&run("source: \"https://a b\"\n", source)),
            ["\"https://a b\""]
        );
        assert_eq!(at(&run("source: \"http://\"\n", source)), ["\"http://\""]);
        ok("source: https://x.y/z\n", source);

        let generated = field("generated", FieldType::True, false);
        assert_eq!(at(&run("generated: \"true\"\n", generated)), ["\"true\""]);
        assert_eq!(at(&run("generated: false\n", generated)), ["false"]);
        ok("generated: true\n", generated);

        let generator = field("generator", FieldType::Generator, false);
        assert_eq!(at(&run("generator: comfyui@\n", generator)), ["comfyui@"]);
        assert_eq!(at(&run("generator: \"@1\"\n", generator)), ["\"@1\""]);
        assert_eq!(at(&run("generator: a@b@c\n", generator)), ["a@b@c"]);
        assert_eq!(at(&run("generator: \"a b@c\"\n", generator)), ["\"a b@c\""]);
        assert_eq!(at(&run("generator: comfyui\n", generator)), ["comfyui"]);
        ok("generator: comfyui@1.2\n", generator);

        let addresses = field("addresses", FieldType::NoteIds, false);
        assert_eq!(at(&run("addresses: [n1, x]\n", addresses)), ["x"]);
        assert_eq!(at(&run("addresses: [n]\n", addresses)), ["n"]);
        assert_eq!(at(&run("addresses: n1\n", addresses)), ["n1"]);
        ok("addresses: [n1, n20]\n", addresses);

        let outputs = field("outputs", FieldType::BinaryRefs, false);
        let good = format!("outputs:\n  image: a/b.png@sha256:{HASH}\n");
        ok(&good, outputs);
        let upper = format!("outputs:\n  image: a.png@sha256:{}\n", HASH.to_uppercase());
        assert_eq!(run(&upper, outputs).len(), 1);
        let short = format!("outputs:\n  image: a.png@sha256:{}\n", &HASH[1..]);
        assert_eq!(run(&short, outputs).len(), 1);
        let at_path = format!("outputs:\n  image: a@b@sha256:{HASH}\n");
        assert_eq!(run(&at_path, outputs).len(), 1);
        let no_path = format!("outputs:\n  image: \"@sha256:{HASH}\"\n");
        assert_eq!(run(&no_path, outputs).len(), 1);
        let one_of_two = format!("outputs:\n  a: x.png@sha256:{HASH}\n  b: nope\n");
        assert_eq!(at(&run(&one_of_two, outputs)), ["nope"]);
        let listed = "outputs:\n  a: [x, y]\n";
        assert_eq!(at(&run(listed, outputs)), ["[x, y]"]);
        assert_eq!(at(&run("outputs: x\n", outputs)), ["x"]);

        let inputs = field("inputs", FieldType::OpenMap, false);
        assert_eq!(at(&run("inputs: x\n", inputs)), ["x"]);
        ok("inputs:\n  a: 1\n  b: [x, y]\n", inputs);
    }

    // AC: CE-11.1
    #[test]
    fn lens_entries_follow_the_lens_table() {
        let lens = field("lens", FieldType::Lens, false);
        let tier = run("lens:\n  tier: T4\n", lens);
        assert_eq!(at(&tier), ["T4"]);
        assert!(tier[0].1.starts_with("`lens.tier`"), "{}", tier[0].1);
        let unaware = run("lens:\n  unaware_of: T-930\n", lens);
        assert_eq!(unaware.len(), 1);
        assert!(unaware[0].1.contains("must be a list"));
        let shape = run("lens: x\n", lens);
        assert_eq!(at(&shape), ["x"]);
        assert!(shape[0].1.starts_with("`lens` must be a map"));
        ok("lens:\n  mood: x\n", lens);
        ok("lens:\n  tier: T1\n  unaware_of: [T-930]\n", lens);
    }

    #[test]
    fn a_wrongly_shaped_required_value_gives_one_problem() {
        let title = run("title: []\n", field("title", FieldType::Text, true));
        assert_eq!(title.len(), 1, "{title:?}");
        let floors = run("floors: []\n", field("floors", FieldType::FloorRange, true));
        assert_eq!(floors.len(), 1, "{floors:?}");
        let held = run(
            "held_by: []\n",
            field("held_by", FieldType::Links(TARGETS), true),
        );
        assert_eq!(held.len(), 1, "{held:?}");
        assert!(held[0].1.contains("must not be empty"));
    }

    #[test]
    fn id_and_kind_are_not_checked_here() {
        ok("id: \"\"\n", field("id", FieldType::Id, true));
        ok("kind: [a]\n", field("kind", FieldType::KindName, true));
    }

    #[test]
    fn a_map_span_runs_from_the_first_key_to_the_last_line() {
        let record = record("inputs:\n  a: 1\n  b: 2\n", "");
        let front = record.front_matter().expect("parses");
        let field = front.get("inputs").expect("field");
        let span = ValueRef::of(&field.value).span();
        assert_eq!(span.get(record.text()), Some("a: 1\n  b: 2\n"));
    }
}
