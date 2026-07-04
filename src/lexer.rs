use crate::error::{ParseError, ParseErrorKind};
use crate::span::{SourcePos, SourceSpan};

#[derive(Debug, Clone)]
pub(crate) struct SourceCursor<'a> {
    source: &'a str,
    byte: usize,
    line: usize,
    column: usize,
}

impl<'a> SourceCursor<'a> {
    pub(crate) const fn new(source: &'a str) -> Self {
        Self {
            source,
            byte: 0,
            line: 1,
            column: 1,
        }
    }

    pub(crate) const fn pos(&self) -> SourcePos {
        SourcePos::new_unchecked(self.line, self.column, self.byte)
    }

    pub(crate) fn is_eof(&self) -> bool {
        self.byte >= self.source.len()
    }

    pub(crate) fn starts_with(&self, expected: &str) -> bool {
        self.remaining().starts_with(expected)
    }

    pub(crate) fn peek(&self) -> Option<char> {
        self.remaining().chars().next()
    }

    pub(crate) fn bump(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.byte += ch.len_utf8();
        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(ch)
    }

    pub(crate) fn skip_ws(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.bump();
        }
    }

    pub(crate) fn expect(&mut self, expected: &'static str) -> Result<SourceSpan, ParseError> {
        let start = self.pos();
        if !self.starts_with(expected) {
            return Err(self.error(
                ParseErrorKind::UnexpectedToken { expected },
                SourceSpan::new_unchecked(start, self.pos()),
            ));
        }

        for _ in expected.chars() {
            self.bump();
        }
        Ok(SourceSpan::new_unchecked(start, self.pos()))
    }

    pub(crate) fn take_until(&mut self, pattern: &str) -> Option<(&'a str, SourceSpan)> {
        let start = self.pos();
        let start_byte = self.byte;
        let relative_end = self.remaining().find(pattern)?;
        let end_byte = start_byte + relative_end;
        while self.byte < end_byte {
            self.bump();
        }
        Some((
            &self.source[start_byte..end_byte],
            SourceSpan::new_unchecked(start, self.pos()),
        ))
    }

    pub(crate) fn source(&self) -> &'a str {
        self.source
    }

    pub(crate) fn remaining(&self) -> &'a str {
        &self.source[self.byte..]
    }

    pub(crate) fn error(&self, kind: ParseErrorKind, span: SourceSpan) -> ParseError {
        ParseError::new(kind, span)
    }
}
