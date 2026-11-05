//! Byte ranges into a text.

/// A byte range `start..end` into a text, with `start <= end`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Span {
    /// First byte of the range.
    pub start: usize,
    /// One past the last byte of the range.
    pub end: usize,
}

impl Span {
    /// Builds a span from its two byte offsets.
    #[must_use]
    pub const fn new(start: usize, end: usize) -> Span {
        Span { start, end }
    }

    /// Number of bytes in the span.
    #[must_use]
    pub const fn len(self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// Whether the span covers no bytes.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.end <= self.start
    }

    /// The slice of `text` the span covers, or `None` when the span is out of range or does not
    /// sit on character boundaries.
    #[must_use]
    pub fn get(self, text: &str) -> Option<&str> {
        text.get(self.start..self.end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_is_checked() {
        let span = Span::new(0, 2);
        assert_eq!(span.len(), 2);
        assert!(!span.is_empty());
        assert!(Span::new(3, 3).is_empty());
        assert_eq!(span.get("abc"), Some("ab"));
        assert_eq!(Span::new(1, 2).get("è"), None);
        assert_eq!(Span::new(2, 9).get("abc"), None);
    }
}
