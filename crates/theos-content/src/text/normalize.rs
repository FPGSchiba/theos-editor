//! Line-ending normalisation.

use std::borrow::Cow;

/// Turns every `\r\n` and every lone `\r` into `\n`. Returns the LF text and whether anything
/// changed; unchanged input is borrowed.
#[must_use]
pub fn normalize(input: &str) -> (Cow<'_, str>, bool) {
    if !input.contains('\r') {
        return (Cow::Borrowed(input), false);
    }
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\r' {
            chars.next_if_eq(&'\n');
            out.push('\n');
        } else {
            out.push(c);
        }
    }
    (Cow::Owned(out), true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::borrow::Cow;

    // AC: CE-09.4
    #[test]
    fn crlf_and_lone_cr_become_lf() {
        let (text, changed) = normalize("a\r\nb\rc\n");
        assert_eq!(text, "a\nb\nc\n");
        assert!(changed);

        let (text, changed) = normalize("a\nb");
        assert!(matches!(text, Cow::Borrowed("a\nb")));
        assert!(!changed);

        assert_eq!(normalize("\r\r\n").0, "\n\n");
    }
}
