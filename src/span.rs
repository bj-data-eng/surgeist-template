#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourcePos {
    line: usize,
    column: usize,
    byte: usize,
}

impl SourcePos {
    #[allow(dead_code)]
    pub(crate) const fn new_unchecked(line: usize, column: usize, byte: usize) -> Self {
        Self { line, column, byte }
    }

    pub const fn line(self) -> usize {
        self.line
    }

    pub const fn column(self) -> usize {
        self.column
    }

    pub const fn byte(self) -> usize {
        self.byte
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceSpan {
    start: SourcePos,
    end: SourcePos,
}

impl SourceSpan {
    #[cfg(test)]
    pub(crate) fn try_new(start: SourcePos, end: SourcePos) -> Result<Self, SpanError> {
        if end.byte < start.byte {
            return Err(SpanError::EndBeforeStart { start, end });
        }

        if end.line < start.line || (end.line == start.line && end.column < start.column) {
            return Err(SpanError::EndBeforeStart { start, end });
        }

        if end.byte == start.byte && (end.line != start.line || end.column != start.column) {
            return Err(SpanError::InconsistentZeroLengthSpan { start, end });
        }

        Ok(Self { start, end })
    }

    #[allow(dead_code)]
    pub(crate) const fn new_unchecked(start: SourcePos, end: SourcePos) -> Self {
        Self { start, end }
    }

    pub const fn start(self) -> SourcePos {
        self.start
    }

    pub const fn end(self) -> SourcePos {
        self.end
    }

    pub const fn len_bytes(self) -> usize {
        self.end.byte - self.start.byte
    }
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanError {
    EndBeforeStart { start: SourcePos, end: SourcePos },
    InconsistentZeroLengthSpan { start: SourcePos, end: SourcePos },
}

#[cfg(test)]
mod tests {
    use super::{SourcePos, SourceSpan};

    #[test]
    fn span_reports_line_column_range() {
        let span = SourceSpan::try_new(
            SourcePos::new_unchecked(2, 5, 14),
            SourcePos::new_unchecked(2, 9, 18),
        )
        .expect("valid span");

        assert_eq!(span.start().line(), 2);
        assert_eq!(span.start().column(), 5);
        assert_eq!(span.end().column(), 9);
        assert_eq!(span.len_bytes(), 4);
    }

    #[test]
    fn rejects_invalid_source_spans() {
        let start = SourcePos::new_unchecked(2, 5, 14);
        let earlier_line = SourcePos::new_unchecked(1, 9, 18);
        let earlier_byte = SourcePos::new_unchecked(2, 9, 12);

        assert!(SourceSpan::try_new(start, earlier_line).is_err());
        assert!(SourceSpan::try_new(start, earlier_byte).is_err());
    }

    #[test]
    fn source_positions_are_not_publicly_constructible() {
        let pos = SourcePos::new_unchecked(3, 2, 19);

        assert_eq!(pos.line(), 3);
        assert_eq!(pos.column(), 2);
        assert_eq!(pos.byte(), 19);
    }
}
