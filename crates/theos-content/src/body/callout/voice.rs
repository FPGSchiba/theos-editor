//! The voice title: a speaker, then beliefs, then at most one tier.

use super::title::TitleToken;
use super::{CalloutTitle, TitleGrammar, TitleInput, Token};
use crate::body::links::Link;
use crate::body::malformed::{Malformed, MalformedKind};
use crate::text::Span;

/// A parsed voice title `[[speaker]] · [[belief]]… · tier`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Voice {
    /// Who speaks.
    pub speaker: Link,
    /// The beliefs voiced.
    pub beliefs: Vec<Link>,
    /// The tier, when the title ends in a bare word.
    pub tier: Option<Token>,
}

/// The grammar of `[!voice]`.
pub(crate) struct VoiceGrammar;

impl TitleGrammar for VoiceGrammar {
    fn name(&self) -> &'static str {
        "voice"
    }

    fn parse(&self, input: &TitleInput<'_>) -> Result<CalloutTitle, Malformed> {
        let fail = |span: Span, message: &str| {
            Malformed::new(input.index, MalformedKind::CalloutTitle, message, span)
        };
        let mut tokens = input.tokens.iter();
        let speaker = match tokens.next() {
            Some(TitleToken::Link(link)) => link.clone(),
            Some(other) => return Err(fail(other.span(), "a voice title starts with the speaker")),
            None => return Err(fail(input.title_span, "a voice title needs a speaker")),
        };
        let mut beliefs = Vec::new();
        let mut tier = None;
        for token in tokens {
            match (token, &tier) {
                (TitleToken::Link(link), None) => beliefs.push(link.clone()),
                (TitleToken::Bare(bare), None) => tier = Some(bare.clone()),
                (other, _) => {
                    return Err(fail(
                        other.span(),
                        "expected a belief link, or one tier as the last token",
                    ));
                }
            }
        }
        Ok(CalloutTitle::Voice(Voice {
            speaker,
            beliefs,
            tier,
        }))
    }
}
