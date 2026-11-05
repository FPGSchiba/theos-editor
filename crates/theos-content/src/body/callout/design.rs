//! The design title: free text with links; never malformed.

use super::{CalloutTitle, TitleGrammar, TitleInput, Token};
use crate::body::links::Link;
use crate::body::malformed::Malformed;

/// A parsed `[!design]` title.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Design {
    /// The whole title, when not empty.
    pub text: Option<Token>,
    /// The links in the title.
    pub links: Vec<Link>,
}

/// The grammar of `[!design]`.
pub(crate) struct DesignGrammar;

impl TitleGrammar for DesignGrammar {
    fn name(&self) -> &'static str {
        "design"
    }

    fn captioned(&self) -> bool {
        false
    }

    fn parse(&self, input: &TitleInput<'_>) -> Result<CalloutTitle, Malformed> {
        let text = input
            .title_span
            .get(input.index.text())
            .filter(|t| !t.is_empty())
            .map(|t| Token {
                text: t.to_owned(),
                span: input.title_span,
            });
        Ok(CalloutTitle::Design(Design {
            text,
            links: input.links.to_vec(),
        }))
    }
}
