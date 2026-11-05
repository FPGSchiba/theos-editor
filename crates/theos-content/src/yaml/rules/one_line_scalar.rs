//! Rule `one-line-scalar` (and `escape`): a scalar whose decoded text is not its own spelling in
//! the source, because it spans lines, or uses an escape other than `\\` and `\"`. Position: the
//! start of the scalar.

use yaml_rust2::Event;
use yaml_rust2::scanner::TScalarStyle;

use super::Rule;
use crate::yaml::scalar::{ScalarStyle, spelled_end};
use crate::yaml::{Input, YamlError, YamlRule};

pub(super) struct OneLineScalar;

impl Rule for OneLineScalar {
    fn check(&self, input: &Input<'_>) -> Option<YamlError> {
        input.events.iter().find_map(|event| {
            let Event::Scalar(text, style, ..) = &event.value else {
                return None;
            };
            let style = match style {
                TScalarStyle::Plain => ScalarStyle::Plain,
                TScalarStyle::SingleQuoted => ScalarStyle::SingleQuoted,
                TScalarStyle::DoubleQuoted => ScalarStyle::DoubleQuoted,
                _ => return None,
            };
            let source = input.index.text();
            let first_difference = spelled_end(source, event.offset, style, text).err()?;
            let is_escape = style == ScalarStyle::DoubleQuoted
                && source.as_bytes().get(first_difference) == Some(&b'\\');
            Some(if is_escape {
                input.error(
                    YamlRule::Escape,
                    event,
                    "only \\\\ and \\\" are allowed as escapes",
                )
            } else {
                input.error(
                    YamlRule::OneLineScalar,
                    event,
                    "a scalar must be written on one line",
                )
            })
        })
    }
}
