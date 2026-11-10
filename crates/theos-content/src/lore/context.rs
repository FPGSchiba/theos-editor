//! The context of the lore step: the only way a lore rule gets data. Every answer adds a key to
//! the reads of the subject that asked, also when it comes from the memo (spec §5.5, L6).

use std::collections::{BTreeSet, HashMap};
use std::rc::Rc;

use crate::kinds::KindRegistry;
use crate::lore::facts::{InLink, InputFacts, RecordFacts};
use crate::lore::vocab::DeveloperWords;
use crate::settings::{self, Parsed, ValidationSettings};

/// Why a fact could not be read: the store failed. Rules never fail on content (L5).
#[derive(Debug, thiserror::Error)]
pub enum FactError {
    /// SQLite reported an error.
    #[error("the index cache failed: {0}")]
    Sqlite(#[from] rusqlite::Error),
}

/// One read of a subject, as the `deps` table stores it (L6).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum ReadKey {
    /// The records with an id.
    Record(String),
    /// The links that name an id.
    Inbound(String),
    /// An input file, by project-relative path.
    File(String),
}

impl ReadKey {
    /// The stored text: `rec:<id>`, `in:<id>` or `file:<path>`.
    pub(crate) fn as_db(&self) -> String {
        match self {
            ReadKey::Record(id) => format!("rec:{id}"),
            ReadKey::Inbound(id) => format!("in:{id}"),
            ReadKey::File(path) => format!("file:{path}"),
        }
    }
}

/// Where a [`Context`] gets its facts. The store answers by id as rules ask, or from facts it
/// loaded at once; both answer every question alike.
pub(crate) trait FactSource {
    /// The facts of every record with the id `id`, ordered by path.
    fn records(&mut self, id: &str) -> Result<Vec<RecordFacts>, FactError>;
    /// The links that name `id`, ordered by source path and file order.
    fn linking_to(&mut self, id: &str) -> Result<Vec<InLink>, FactError>;
    /// The input file at `path`, none when the project has none there.
    fn input(&mut self, path: &str) -> Result<Option<InputFacts>, FactError>;
}

/// What lore rules read through. It asks its source once per question and keeps the answer for
/// the rest of the lore step: the step changes no fact, so every later answer is the same. Each
/// answer is recorded as a read of the subject whose rules run, also an answer from the memo.
pub struct Context<'a> {
    source: Box<dyn FactSource + 'a>,
    registry: &'a KindRegistry,
    /// The reads of the subject whose rules run.
    reads: BTreeSet<ReadKey>,
    records: HashMap<String, Vec<Rc<RecordFacts>>>,
    inbound: HashMap<String, Vec<InLink>>,
    inputs: HashMap<String, Option<Rc<InputFacts>>>,
    settings: Option<Rc<Parsed>>,
    vocab: Option<Rc<DeveloperWords>>,
}

impl<'a> Context<'a> {
    /// A context over `source`, with no reads and an empty memo.
    pub(crate) fn new(source: Box<dyn FactSource + 'a>, registry: &'a KindRegistry) -> Context<'a> {
        Context {
            source,
            registry,
            reads: BTreeSet::new(),
            records: HashMap::new(),
            inbound: HashMap::new(),
            inputs: HashMap::new(),
            settings: None,
            vocab: None,
        }
    }

    /// The kinds and rules of the project.
    #[must_use]
    pub fn registry(&self) -> &'a KindRegistry {
        self.registry
    }

    /// The facts of every record with the id `id`, ordered by path: none, one, or several when
    /// the id is duplicated. Reads `rec:<id>`.
    ///
    /// # Errors
    ///
    /// Fails when the store fails.
    pub fn records(&mut self, id: &str) -> Result<Vec<Rc<RecordFacts>>, FactError> {
        self.reads.insert(ReadKey::Record(id.to_owned()));
        self.load_records(id)
    }

    /// The links that name `id`, ordered by source path and file order. Reads `in:<id>`.
    ///
    /// # Errors
    ///
    /// Fails when the store fails.
    pub fn linking_to(&mut self, id: &str) -> Result<Vec<InLink>, FactError> {
        self.reads.insert(ReadKey::Inbound(id.to_owned()));
        if let Some(found) = self.inbound.get(id) {
            return Ok(found.clone());
        }
        let found = self.source.linking_to(id)?;
        self.inbound.insert(id.to_owned(), found.clone());
        Ok(found)
    }

    /// The input file at the project-relative `path`, none when there is none. Reads
    /// `file:<path>`.
    ///
    /// # Errors
    ///
    /// Fails when the store fails.
    pub fn input(&mut self, path: &str) -> Result<Option<Rc<InputFacts>>, FactError> {
        self.reads.insert(ReadKey::File(path.to_owned()));
        self.load_input(path)
    }

    /// The project's validation settings, read from `.theos/settings.toml` once per run:
    /// empty and without a problem when the project has no such file. Reads
    /// `file:.theos/settings.toml`.
    ///
    /// # Errors
    ///
    /// Fails when the store fails.
    pub fn settings(&mut self) -> Result<Rc<Parsed>, FactError> {
        let input = self.input(settings::SETTINGS)?;
        if let Some(parsed) = &self.settings {
            return Ok(Rc::clone(parsed));
        }
        let parsed = Rc::new(match input {
            None => Parsed::default(),
            Some(input) => {
                settings::parse(input.text.as_deref(), &crate::rules::all(self.registry))
            }
        });
        self.settings = Some(Rc::clone(&parsed));
        Ok(parsed)
    }

    /// The developer words of the project, built from the settings once per run; none only when
    /// not even the built-in patterns compile. Reads `file:.theos/settings.toml`.
    ///
    /// # Errors
    ///
    /// Fails when the store fails.
    pub(crate) fn developer_words(&mut self) -> Result<Option<Rc<DeveloperWords>>, FactError> {
        let parsed = self.settings()?;
        if self.vocab.is_none() {
            // A word list too large to compile leaves the built-in words.
            self.vocab = DeveloperWords::new(&parsed.settings)
                .or_else(|_| DeveloperWords::new(&ValidationSettings::default()))
                .ok()
                .map(Rc::new);
        }
        Ok(self.vocab.clone())
    }

    /// The facts of the record subject with `id` at `path`, without a read: a subject's own
    /// facts change only when it is re-parsed, and then it runs anyway.
    pub(crate) fn subject_record(
        &mut self,
        id: &str,
        path: &str,
    ) -> Result<Option<Rc<RecordFacts>>, FactError> {
        Ok(self
            .load_records(id)?
            .into_iter()
            .find(|record| record.path == path))
    }

    /// The input subject at `path`, without a read, as [`Context::subject_record`].
    pub(crate) fn subject_input(
        &mut self,
        path: &str,
    ) -> Result<Option<Rc<InputFacts>>, FactError> {
        self.load_input(path)
    }

    /// The reads since the last call: those of the subject whose rules ran.
    pub(crate) fn take_reads(&mut self) -> BTreeSet<ReadKey> {
        std::mem::take(&mut self.reads)
    }

    fn load_records(&mut self, id: &str) -> Result<Vec<Rc<RecordFacts>>, FactError> {
        if let Some(found) = self.records.get(id) {
            return Ok(found.clone());
        }
        let found: Vec<Rc<RecordFacts>> =
            self.source.records(id)?.into_iter().map(Rc::new).collect();
        self.records.insert(id.to_owned(), found.clone());
        Ok(found)
    }

    fn load_input(&mut self, path: &str) -> Result<Option<Rc<InputFacts>>, FactError> {
        if let Some(found) = self.inputs.get(path) {
            return Ok(found.clone());
        }
        let found = self.source.input(path)?.map(Rc::new);
        self.inputs.insert(path.to_owned(), found.clone());
        Ok(found)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::index::Slot;
    use crate::text::{Position, Span};

    /// A source that answers from fixed facts and notes every question it is asked.
    struct Fake {
        asked: Rc<RefCell<Vec<String>>>,
    }

    fn facts(path: &str, id: &str) -> RecordFacts {
        RecordFacts {
            path: path.to_owned(),
            id: id.to_owned(),
            kind: None,
            hash: String::new(),
            values: Vec::new(),
            links: Vec::new(),
            beats: Vec::new(),
            notes: Vec::new(),
            inworld: Vec::new(),
        }
    }

    impl FactSource for Fake {
        fn records(&mut self, id: &str) -> Result<Vec<RecordFacts>, FactError> {
            self.asked.borrow_mut().push(format!("records {id}"));
            Ok(match id {
                "a" => vec![facts("x/a.md", "a"), facts("y/a.md", "a")],
                _ => Vec::new(),
            })
        }

        fn linking_to(&mut self, id: &str) -> Result<Vec<InLink>, FactError> {
            self.asked.borrow_mut().push(format!("linking_to {id}"));
            Ok(vec![InLink {
                source_id: "b".to_owned(),
                source_path: "x/b.md".to_owned(),
                slot: Slot::Mention,
                beat: None,
                span: Span::default(),
                position: Position { line: 1, column: 1 },
            }])
        }

        fn input(&mut self, path: &str) -> Result<Option<InputFacts>, FactError> {
            self.asked.borrow_mut().push(format!("input {path}"));
            Ok((path == "in.toml").then(|| InputFacts {
                path: path.to_owned(),
                hash: String::new(),
                text: Some("x".to_owned()),
            }))
        }
    }

    /// The stored texts of `reads`, sorted.
    fn keys(reads: &BTreeSet<ReadKey>) -> Vec<String> {
        let mut keys: Vec<String> = reads.iter().map(ReadKey::as_db).collect();
        keys.sort();
        keys
    }

    fn with_context(test: impl FnOnce(&mut Context<'_>, &RefCell<Vec<String>>)) {
        let registry = KindRegistry::new();
        let asked = Rc::new(RefCell::new(Vec::new()));
        let source = Fake {
            asked: Rc::clone(&asked),
        };
        let mut ctx = Context::new(Box::new(source), &registry);
        test(&mut ctx, &asked);
    }

    #[test]
    fn every_answer_is_a_read_of_the_subject_that_asked() {
        with_context(|ctx, _| {
            let found = ctx.records("a").map(|r| r.len());
            assert!(matches!(found, Ok(2)), "{found:?}");
            assert!(ctx.records("nobody").is_ok_and(|r| r.is_empty()));
            assert!(ctx.linking_to("a").is_ok_and(|links| links.len() == 1));
            assert!(ctx.input("in.toml").is_ok_and(|input| input.is_some()));
            assert!(ctx.input("none.toml").is_ok_and(|input| input.is_none()));
            assert_eq!(
                keys(&ctx.take_reads()),
                [
                    "file:in.toml",
                    "file:none.toml",
                    "in:a",
                    "rec:a",
                    "rec:nobody"
                ]
            );
            assert!(ctx.take_reads().is_empty(), "the reads are taken");
            assert_eq!(ctx.registry().kinds().count(), 0);
        });
    }

    #[test]
    fn an_answer_from_the_memo_is_a_read_too() {
        with_context(|ctx, asked| {
            assert!(ctx.records("a").is_ok());
            assert!(ctx.linking_to("a").is_ok());
            assert!(ctx.input("in.toml").is_ok());
            ctx.take_reads();
            // The next subject asks the same: the source is not asked again, the reads are
            // its own.
            assert!(ctx.records("a").is_ok_and(|r| r.len() == 2));
            assert!(ctx.linking_to("a").is_ok_and(|links| links.len() == 1));
            assert!(ctx.input("in.toml").is_ok_and(|input| input.is_some()));
            assert_eq!(keys(&ctx.take_reads()), ["file:in.toml", "in:a", "rec:a"]);
            assert_eq!(
                *asked.borrow(),
                ["records a", "linking_to a", "input in.toml"]
            );
        });
    }

    #[test]
    fn a_subjects_own_facts_are_no_read() {
        with_context(|ctx, asked| {
            let own = ctx.subject_record("a", "y/a.md");
            assert!(own.is_ok_and(|own| own.is_some_and(|own| own.path == "y/a.md")));
            assert!(
                ctx.subject_record("a", "z/a.md")
                    .is_ok_and(|own| own.is_none())
            );
            assert!(ctx.subject_input("in.toml").is_ok_and(|own| own.is_some()));
            assert!(ctx.take_reads().is_empty());
            assert!(ctx.records("a").is_ok());
            assert_eq!(keys(&ctx.take_reads()), ["rec:a"]);
            assert_eq!(*asked.borrow(), ["records a", "input in.toml"]);
        });
    }

    #[test]
    fn keys_are_spelled_as_stored() {
        assert_eq!(ReadKey::Record("T-950".to_owned()).as_db(), "rec:T-950");
        assert_eq!(ReadKey::Inbound("T-950".to_owned()).as_db(), "in:T-950");
        assert_eq!(
            ReadKey::File(".theos/settings.toml".to_owned()).as_db(),
            "file:.theos/settings.toml"
        );
    }
}
