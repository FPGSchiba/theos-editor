//! Rule `block-scalar`: `d: |` or `d: >`. Position: the scalar event, at the `|` or `>`.

use yaml_rust2::scanner::{TScalarStyle, TokenType};

use super::{Rule, error_at, last_indicator};
use crate::yaml::{Input, YamlError, YamlRule};

pub(super) struct BlockScalar;

impl Rule for BlockScalar {
    fn check(&self, input: &Input<'_>) -> Option<YamlError> {
        let content = input.tokens.iter().find(|token| {
            matches!(
                token.value,
                TokenType::Scalar(TScalarStyle::Literal | TScalarStyle::Folded, _)
            )
        })?;
        // The crate reports the content; the `|` or `>` is the first one after the `:` or `-`.
        let from = last_indicator(input, content.offset)?.offset;
        let header = input.index.text().get(from..content.offset)?;
        let offset = from + header.find(['|', '>'])?;
        error_at(
            input,
            YamlRule::BlockScalar,
            offset,
            "block scalars are not allowed",
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::test_support::front_error;

    // AC: CE-09.3
    #[test]
    fn block_scalars_are_rejected() {
        let want = Some(("block-scalar", "2:4".to_owned()));
        assert_eq!(front_error("d: |\n  x\n"), want);
        assert_eq!(front_error("e: >\n  x\n"), want);
    }
}
