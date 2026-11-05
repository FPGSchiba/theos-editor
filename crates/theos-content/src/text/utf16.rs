//! Byte offset to UTF-16 offset conversion, for editors that count UTF-16 code units.

use super::index::{PositionError, TextIndex};

impl TextIndex {
    /// The UTF-16 offset of a byte offset; the end of the text is a valid offset.
    ///
    /// # Errors
    ///
    /// [`PositionError::OutOfRange`] past the end, [`PositionError::NotCharBoundary`] inside a
    /// character.
    pub fn utf16_offset(&self, offset: usize) -> Result<usize, PositionError> {
        if offset > self.text.len() {
            return Err(PositionError::OutOfRange {
                offset,
                len: self.text.len(),
            });
        }
        if !self.text.is_char_boundary(offset) {
            return Err(PositionError::NotCharBoundary { offset });
        }
        let line = self.line_starts.partition_point(|&start| start <= offset) - 1;
        let start = self.line_starts[line];
        let within: usize = self.text[start..offset].chars().map(char::len_utf16).sum();
        Ok(self.utf16_starts[line] + within)
    }

    /// The byte offset of a UTF-16 offset; the end of the text is a valid offset.
    ///
    /// # Errors
    ///
    /// [`PositionError::OutOfRange`] past the end (its `offset` is the UTF-16 offset and `len`
    /// the text length in UTF-16 code units), [`PositionError::InsideSurrogatePair`] between the
    /// two halves of a character outside the basic multilingual plane.
    pub fn offset_from_utf16(&self, utf16: usize) -> Result<usize, PositionError> {
        let line = self.utf16_starts.partition_point(|&start| start <= utf16) - 1;
        let start = self.line_starts[line];
        let mut position = self.utf16_starts[line];
        for (i, c) in self.text[start..].char_indices() {
            if position == utf16 {
                return Ok(start + i);
            }
            position += c.len_utf16();
            if position > utf16 {
                return Err(PositionError::InsideSurrogatePair { utf16 });
            }
            if c == '\n' {
                break;
            }
        }
        if position == utf16 && line + 1 == self.line_starts.len() {
            return Ok(self.text.len());
        }
        Err(PositionError::OutOfRange {
            offset: utf16,
            len: position,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::index::{PositionError, TextIndex};

    const P10: &str = "— · “d” ‘s’ café cafe\u{0301} 5\u{00A0}km 😀\n";

    #[test]
    fn utf16_counts_surrogate_pairs() {
        let index = TextIndex::new("a😀b".to_string());
        for (byte, utf16) in [(0, 0), (1, 1), (5, 3), (6, 4)] {
            assert_eq!(index.utf16_offset(byte), Ok(utf16), "byte {byte}");
            assert_eq!(index.offset_from_utf16(utf16), Ok(byte), "utf16 {utf16}");
        }
        assert_eq!(
            index.offset_from_utf16(2),
            Err(PositionError::InsideSurrogatePair { utf16: 2 })
        );
    }

    #[test]
    fn utf16_round_trips() {
        let text = format!("{P10}second 😀 line é\nlast");
        let index = TextIndex::new(text.clone());
        for offset in (0..=text.len()).filter(|o| text.is_char_boundary(*o)) {
            let utf16 = index.utf16_offset(offset).unwrap();
            assert_eq!(
                index.offset_from_utf16(utf16),
                Ok(offset),
                "offset {offset}"
            );
        }
        let end = index.utf16_offset(text.len()).unwrap();
        assert_eq!(end, text.encode_utf16().count());
    }

    #[test]
    fn utf16_errors() {
        let index = TextIndex::new("aé😀".to_string());
        assert_eq!(
            index.utf16_offset(2),
            Err(PositionError::NotCharBoundary { offset: 2 })
        );
        assert_eq!(
            index.utf16_offset(99),
            Err(PositionError::OutOfRange { offset: 99, len: 7 })
        );
        assert_eq!(
            index.offset_from_utf16(99),
            Err(PositionError::OutOfRange { offset: 99, len: 4 })
        );
    }
}
