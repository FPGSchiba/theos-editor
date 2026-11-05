//! Checking one record against its kind: the record rules, the YAML rules, `schema/kind`, the
//! schema rules every kind shares and then the checks of the kind itself.
//!
//! The order of the rules is the order of the phase spec's table. Each schema rule is one
//! module with one `RULE` constant, registered by one line in `SCHEMA_RULES`.

mod decode;
mod file_name;
mod id;
mod kind;
mod required;
mod unknown_entry;
mod unknown_field;
mod value;

use crate::finding::{Finding, Severity};
use crate::kinds::{CheckInput, KindRegistry, KindSpec, Problem};
use crate::record::{Record, decode as decode_bytes};
use crate::text::{Position, Span};
use crate::yaml::{FrontMatter, YamlRule};

/// The result of checking one record.
#[derive(Clone, Debug)]
pub struct Checked {
    /// The file stem of the path.
    pub id: String,
    /// The parsed record; `None` after `record/utf8` or a split error.
    pub record: Option<Record>,
    /// The kind name, when `schema/kind` passed.
    pub kind: Option<&'static str>,
    /// What the check found, step `record`, in the order the rules ran.
    pub findings: Vec<Finding>,
}

/// What a schema rule reads: one record whose kind is known.
pub(crate) struct Subject<'a> {
    /// The file stem.
    pub id: &'a str,
    /// The front matter.
    pub front: &'a FrontMatter,
    /// The kind of the record.
    pub spec: &'a KindSpec,
    /// The span of the `kind` value.
    pub kind_span: Span,
    /// All kinds.
    pub registry: &'a KindRegistry,
}

/// A rule every kind shares, run by [`check_record`] after `schema/kind`.
pub(crate) struct SchemaRule {
    /// `<group>/<name>`.
    pub id: &'static str,
    /// The default severity of its findings.
    pub severity: Severity,
    /// The rule.
    pub run: fn(&Subject<'_>) -> Vec<Problem>,
}

/// The schema rules in the order they run; a new rule is one module plus one line here.
pub(crate) const SCHEMA_RULES: &[SchemaRule] = &[
    id::RULE,
    file_name::RULE,
    required::RULE,
    value::RULE,
    unknown_field::RULE,
    unknown_entry::RULE,
];

const RECORD_RULES: &[(&str, Severity)] = &[
    ("record/path", Severity::Error),
    ("record/read", Severity::Error),
    (decode::RULE, Severity::Error),
    ("record/bom", Severity::Error),
    ("record/opening-line", Severity::Error),
    ("record/closing-line", Severity::Error),
];

/// Every rule id with its default severity, in the order the rules run: the record rules, the
/// YAML rules, `schema/kind` and the shared schema rules. `record/path` and `record/read` are
/// reported by the project sync, which sees file names and read errors. The checks of single
/// kinds are part of the registry fingerprint instead.
#[must_use]
pub fn rules() -> Vec<(&'static str, Severity)> {
    let yaml = YamlRule::ALL
        .iter()
        .map(|rule| (rule.id(), Severity::Error));
    RECORD_RULES
        .iter()
        .copied()
        .chain(yaml)
        .chain([(kind::RULE, Severity::Error)])
        .chain(SCHEMA_RULES.iter().map(|rule| (rule.id, rule.severity)))
        .collect()
}

/// Checks the file `bytes` at the project-relative `path` against its kind.
#[must_use]
pub fn check_record(registry: &KindRegistry, path: &str, bytes: &[u8]) -> Checked {
    let id = stem(path).to_owned();
    let mut checked = Checked {
        id,
        record: None,
        kind: None,
        findings: Vec::new(),
    };
    let text = match decode_bytes(bytes) {
        Ok(text) => text,
        Err(error) => {
            checked
                .findings
                .extend(decode::finding(path, &checked.id, bytes, &error));
            return checked;
        }
    };
    let record = match Record::parse(text) {
        Ok(record) => record,
        Err(error) => {
            let message = error.to_string();
            checked.findings.push(Finding {
                rule: format!("record/{}", error.rule()),
                severity: Severity::Error,
                path: path.to_owned(),
                id: Some(checked.id.clone()),
                span: Some(Span::default()),
                position: Some(error.position()),
                message: message.strip_prefix("1:1: ").unwrap_or(&message).to_owned(),
            });
            return checked;
        }
    };
    let found = Found {
        path,
        id: &checked.id,
        record: &record,
    };
    let mut findings = Vec::new();
    match record.front_matter() {
        Err(error) => findings.push(found.finding(
            error.rule.id(),
            Severity::Error,
            error.span,
            error.message.clone(),
        )),
        Ok(front) => {
            checked.kind = judge(registry, front, &found, &mut findings);
        }
    }
    checked.findings = findings;
    checked.record = Some(record);
    checked
}

/// Where findings of one record are made.
struct Found<'a> {
    path: &'a str,
    id: &'a str,
    record: &'a Record,
}

impl Found<'_> {
    fn finding(&self, rule: &str, severity: Severity, span: Span, message: String) -> Finding {
        let position: Option<Position> = self.record.index().position(span.start).ok();
        Finding {
            rule: rule.to_owned(),
            severity,
            path: self.path.to_owned(),
            id: Some(self.id.to_owned()),
            span: Some(span),
            position,
            message,
        }
    }

    fn all(&self, rule: &str, severity: Severity, problems: Vec<Problem>, into: &mut Vec<Finding>) {
        into.extend(
            problems
                .into_iter()
                .map(|p| self.finding(rule, severity, p.span, p.message)),
        );
    }
}

/// Runs `schema/kind`, the schema rules and the kind's checks; returns the kind when it passed.
fn judge(
    registry: &KindRegistry,
    front: &FrontMatter,
    found: &Found<'_>,
    findings: &mut Vec<Finding>,
) -> Option<&'static str> {
    let (spec, kind_span) = match kind::run(registry, front) {
        Ok(known) => known,
        Err(problem) => {
            found.all(kind::RULE, Severity::Error, vec![problem], findings);
            return None;
        }
    };
    let subject = Subject {
        id: found.id,
        front,
        spec,
        kind_span,
        registry,
    };
    for rule in SCHEMA_RULES {
        found.all(rule.id, rule.severity, (rule.run)(&subject), findings);
    }
    let input = CheckInput {
        id: found.id,
        front,
        kind_span,
    };
    for check in &spec.checks {
        found.all(check.rule, Severity::Error, (check.run)(&input), findings);
    }
    Some(spec.name)
}

/// The file stem of a project-relative path.
fn stem(path: &str) -> &str {
    let name = path.rsplit('/').next().unwrap_or(path);
    name.rsplit_once('.').map_or(name, |(stem, _)| stem)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::finding::Severity::{Error, Warning};
    use crate::parts;

    type Row = (&'static str, Severity, usize, usize);

    fn run(path: &str, bytes: &[u8]) -> Result<Checked, Box<dyn std::error::Error>> {
        Ok(check_record(&parts::builtin()?, path, bytes))
    }

    fn rows(checked: &Checked) -> Vec<(String, Severity, usize, usize)> {
        checked
            .findings
            .iter()
            .map(|f| {
                let (line, column) = f.position.map_or((0, 0), |p| (p.line, p.column));
                (f.rule.clone(), f.severity, line, column)
            })
            .collect()
    }

    fn expect(rows_: &[Row]) -> Vec<(String, Severity, usize, usize)> {
        rows_
            .iter()
            .map(|(rule, severity, line, column)| ((*rule).to_owned(), *severity, *line, *column))
            .collect()
    }

    const BAND: &str = "id: a\nkind: band\ntitle: A\nparent: tower\nfloors: [1, 9]\n";

    // AC: CE-11.1
    #[test]
    fn each_rule_reports_at_its_position() -> Result<(), Box<dyn std::error::Error>> {
        let topic = |track: &str, counterpart: &str| {
            format!(
                "---\nid: runes-in-world\nkind: topic\ntitle: A\nstatus: draft\ntrack: {track}\n{counterpart}---\n"
            )
        };
        let cases: Vec<(&str, Vec<u8>, Vec<Row>)> = vec![
            (
                "world/a.md",
                b"---\nid: \xff\n".to_vec(),
                vec![("record/utf8", Error, 2, 5)],
            ),
            (
                "world/a.md",
                b"--- \nid: a\n---\n".to_vec(),
                vec![("record/opening-line", Error, 1, 1)],
            ),
            (
                "world/a.md",
                b"---\nid: a\nkind: &x band\n---\n".to_vec(),
                vec![("yaml/anchor", Error, 3, 7)],
            ),
            (
                "world/a.md",
                b"---\nid: a\ntitle: A\n---\n".to_vec(),
                vec![("schema/kind", Error, 1, 1)],
            ),
            (
                "world/a.md",
                b"---\nid: a\nkind: kindd\n---\n".to_vec(),
                vec![("schema/kind", Error, 3, 7)],
            ),
            (
                "world/a.md",
                b"---\nkind: band\ntitle: A\nparent: tower\nfloors: [1, 9]\n---\n".to_vec(),
                vec![("schema/id", Error, 2, 7)],
            ),
            (
                "world/a.md",
                b"---\nid: b\nkind: band\ntitle: A\nparent: tower\nfloors: [1, 9]\n---\n".to_vec(),
                vec![("index/file-name", Error, 2, 5)],
            ),
            (
                "world/a.md",
                b"---\nid: a\nkind: band\ntitle: A\nparent: tower\n---\n".to_vec(),
                vec![("schema/required", Error, 3, 7)],
            ),
            (
                "world/f04-x.md",
                b"---\nid: f04-x\nkind: floor\ntitle: A\nstatus: draft\nparent: band\nnumber: 104\n---\n"
                    .to_vec(),
                vec![("schema/value", Error, 7, 9)],
            ),
            (
                "world/a.md",
                format!("---\n{BAND}colour: red\n---\n").into_bytes(),
                vec![("schema/unknown-field", Warning, 7, 1)],
            ),
            (
                "lore/topics/runes-in-world.md",
                topic("explaining", "counterpart: runes-explaining\n").into_bytes(),
                vec![("schema/topic-track", Error, 6, 8)],
            ),
            (
                "lore/topics/runes-in-world.md",
                topic("in-world", "").into_bytes(),
                vec![("schema/topic-counterpart", Error, 3, 7)],
            ),
        ];
        for (path, bytes, want) in cases {
            let checked = run(path, &bytes)?;
            assert_eq!(
                rows(&checked),
                expect(&want),
                "{}",
                String::from_utf8_lossy(&bytes)
            );
        }
        let band = run(
            "world/a.md",
            b"---\nid: a\nkind: band\ntitle: A\nparent: tower\n---\n",
        )?;
        assert_eq!(band.findings[0].message, "a band needs `floors`");
        assert_eq!(band.kind, Some("band"));
        assert_eq!(band.id, "a");
        Ok(())
    }

    // AC: CE-11.4
    #[test]
    fn unknown_fields_and_entries_warn() -> Result<(), Box<dyn std::error::Error>> {
        let npc = |role: &str, colour: &str, lens: &str| {
            format!("---\nid: a\nkind: npc\ntitle: A\nparent: f\n{role}{colour}{lens}---\n")
        };
        let both = npc("", "colour: red\n", "lens:\n  mood: x\n");
        let checked = run("people/a.md", both.as_bytes())?;
        assert_eq!(
            rows(&checked),
            expect(&[
                ("schema/unknown-field", Warning, 6, 1),
                ("schema/unknown-entry", Warning, 8, 3)
            ])
        );

        let with_error = npc("role: \"\"\n", "colour: red\n", "lens:\n  mood: x\n");
        let checked = run("people/a.md", with_error.as_bytes())?;
        assert_eq!(
            rows(&checked),
            expect(&[
                ("schema/value", Error, 6, 7),
                ("schema/unknown-field", Warning, 7, 1),
                ("schema/unknown-entry", Warning, 9, 3)
            ])
        );

        let only_error = npc("role: \"\"\n", "", "");
        let checked = run("people/a.md", only_error.as_bytes())?;
        assert_eq!(rows(&checked), expect(&[("schema/value", Error, 6, 7)]));
        Ok(())
    }

    // AC: CE-11.1
    #[test]
    fn a_broken_front_matter_stops_the_schema() -> Result<(), Box<dyn std::error::Error>> {
        let checked = run("world/a.md", b"---\nid: a\nkind: &x kindd\n---\n")?;
        assert_eq!(rows(&checked), expect(&[("yaml/anchor", Error, 3, 7)]));
        assert!(checked.record.is_some());
        assert_eq!(checked.kind, None);
        Ok(())
    }

    #[test]
    fn an_unknown_kind_stops_the_other_schema_rules() -> Result<(), Box<dyn std::error::Error>> {
        let checked = run("world/a.md", b"---\nid: a\nkind: kindd\ncolour: red\n---\n")?;
        assert_eq!(rows(&checked), expect(&[("schema/kind", Error, 3, 7)]));
        assert!(checked.findings[0].message.contains("band"));
        assert_eq!(checked.kind, None);
        let list = run("world/a.md", b"---\nid: a\nkind: [x]\n---\n")?;
        assert_eq!(rows(&list), expect(&[("schema/kind", Error, 3, 7)]));
        Ok(())
    }

    #[test]
    fn file_names_break_id_rules() -> Result<(), Box<dyn std::error::Error>> {
        let truth = b"---\nid: T-12\nkind: truth\ntitle: A\nstatus: draft\ntarget: hinted\n---\n";
        let checked = run("lore/truths/T-12.md", truth)?;
        assert_eq!(rows(&checked), expect(&[("schema/id", Error, 2, 5)]));
        assert!(
            checked.findings[0]
                .message
                .contains("`T-` and at least three digits")
        );
        let list = run(
            "world/a.md",
            b"---\nid: [a]\nkind: band\ntitle: A\nparent: t\nfloors: [1, 9]\n---\n",
        )?;
        assert_eq!(rows(&list), expect(&[("schema/id", Error, 2, 5)]));
        Ok(())
    }

    #[test]
    fn rules_are_unique_and_grouped() -> Result<(), Box<dyn std::error::Error>> {
        let rules = rules();
        let mut seen = std::collections::BTreeSet::new();
        for (id, _) in &rules {
            assert_eq!(id.matches('/').count(), 1, "{id}");
            assert!(seen.insert(*id), "{id} twice");
        }
        for reported in [
            "record/path",
            "record/read",
            "record/utf8",
            "record/bom",
            "record/opening-line",
            "record/closing-line",
            "yaml/anchor",
            "yaml/syntax",
            "schema/kind",
            "schema/id",
            "index/file-name",
            "schema/required",
            "schema/value",
            "schema/unknown-field",
            "schema/unknown-entry",
        ] {
            assert!(seen.contains(reported), "{reported} missing");
        }
        assert!(rules.contains(&("schema/unknown-field", Warning)));
        let bom = run("world/a.md", "\u{feff}---\n---\n".as_bytes())?;
        assert_eq!(rows(&bom), expect(&[("record/bom", Error, 1, 1)]));
        let closing = run("world/a.md", b"---\nid: a\n")?;
        assert_eq!(
            rows(&closing),
            expect(&[("record/closing-line", Error, 1, 1)])
        );
        Ok(())
    }
}
