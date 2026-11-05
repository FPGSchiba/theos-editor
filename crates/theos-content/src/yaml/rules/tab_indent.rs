//! Rule `tab-indent`: a tab before the first non-blank character of a line. Position: that tab.

use super::Rule;
use super::error_at;
use crate::yaml::{Input, YamlError, YamlRule};

pub(super) struct TabIndent;

impl Rule for TabIndent {
    fn check(&self, input: &Input<'_>) -> Option<YamlError> {
        let text = input.span.get(input.index.text())?;
        let mut line_start = input.span.start;
        for line in text.split_inclusive('\n') {
            let indent = line.len() - line.trim_start_matches([' ', '\t']).len();
            let blank = line[indent..].trim_end_matches(['\r', '\n']).is_empty();
            if let Some(tab) = line[..indent].find('\t').filter(|_| !blank) {
                let offset = line_start + tab;
                return error_at(input, YamlRule::TabIndent, offset, "tabs cannot indent");
            }
            line_start += line.len();
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::test_support::front_error;

    // AC: CE-09.3
    #[test]
    fn tabs_do_not_indent() {
        let want = Some(("tab-indent", "3:1".to_owned()));
        assert_eq!(front_error("a: 1\n\tb: 2\n"), want);
        assert_eq!(front_error("l:\n\t- a\n"), want);
        assert_eq!(front_error("a: x\ty\n"), None);
    }

    #[test]
    fn blank_lines_with_tabs_are_skipped() {
        assert_eq!(front_error("a: 1\n\t\nb: 2\n"), None);
    }
}
