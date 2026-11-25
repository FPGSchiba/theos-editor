//! Writing an error once, with its causes.

/// The text of `error` and of each cause, joined with `": "`. A cause whose text the message so
/// far already holds is left out, since many errors repeat their source in their own message.
#[must_use]
pub fn report(error: &dyn std::error::Error) -> String {
    let mut text = error.to_string();
    let mut cause = error.source();
    while let Some(inner) = cause {
        let inner_text = inner.to_string();
        if !text.contains(&inner_text) {
            text.push_str(": ");
            text.push_str(&inner_text);
        }
        cause = inner.source();
    }
    text
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use theos_content::formalism::{FormalismError, SourceError};
    use theos_content::lore::FactError;
    use theos_content::walk::WalkError;
    use theos_content::{IndexError, ProjectError, ReadError};

    use super::*;
    use crate::StudioError;

    fn io(text: &str) -> std::io::Error {
        std::io::Error::other(text.to_owned())
    }

    /// The nine variants of spec §5.12, each with the text of its innermost cause.
    fn variants() -> Vec<(Box<dyn std::error::Error>, &'static str)> {
        let path = PathBuf::from("x");
        vec![
            (
                Box::new(FormalismError::Source(SourceError {
                    message: "inner-source".to_owned(),
                    position: None,
                    span: None,
                })),
                "inner-source",
            ),
            (
                Box::new(FormalismError::Io {
                    path: path.clone(),
                    source: io("inner-formalism-io"),
                }),
                "inner-formalism-io",
            ),
            (
                Box::new(StudioError::Index(IndexError::Sqlite(
                    rusqlite::Error::QueryReturnedNoRows,
                ))),
                "Query returned no rows",
            ),
            (
                Box::new(StudioError::Index(IndexError::Cache {
                    path: path.clone(),
                    source: io("inner-cache"),
                })),
                "inner-cache",
            ),
            (
                Box::new(StudioError::Index(IndexError::CacheInUse {
                    path: path.clone(),
                    source: io("inner-in-use"),
                })),
                "inner-in-use",
            ),
            (
                Box::new(StudioError::Project(ProjectError::Io {
                    path: path.clone(),
                    source: io("inner-project-io"),
                })),
                "inner-project-io",
            ),
            (
                Box::new(ReadError::Io {
                    path,
                    source: io("inner-read-io"),
                }),
                "inner-read-io",
            ),
            (
                Box::new(WalkError::from(ignore::Error::Io(io("inner-denied")))),
                "inner-denied",
            ),
            (
                Box::new(FactError::Sqlite(rusqlite::Error::QueryReturnedNoRows)),
                "Query returned no rows",
            ),
        ]
    }

    #[test]
    fn each_cause_appears_once() {
        for (error, innermost) in variants() {
            let text = report(error.as_ref());
            assert_eq!(text.matches(innermost).count(), 1, "{text}");
        }
    }

    #[derive(Debug, thiserror::Error)]
    #[error("outer")]
    struct Outer(#[source] Inner);

    #[derive(Debug, thiserror::Error)]
    #[error("inner")]
    struct Inner;

    #[test]
    fn a_cause_the_text_lacks_is_added() {
        assert_eq!(report(&Outer(Inner)), "outer: inner");
    }
}
