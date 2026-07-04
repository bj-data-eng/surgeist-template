use crate::span::SourceSpan;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseErrorKind {
    UnexpectedEof,
    UnexpectedToken { expected: &'static str },
    UnclosedElement { name: String },
    MismatchedCloseTag { expected: String, found: String },
    InvalidName { name: String },
    InvalidExpression { reason: &'static str },
    StrayTemplateTag { tag: String },
    UnsupportedTemplateTag { tag: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    kind: ParseErrorKind,
    span: SourceSpan,
}

impl ParseError {
    #[allow(dead_code)]
    pub(crate) const fn new(kind: ParseErrorKind, span: SourceSpan) -> Self {
        Self { kind, span }
    }

    pub const fn kind(&self) -> &ParseErrorKind {
        &self.kind
    }

    pub const fn span(&self) -> SourceSpan {
        self.span
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationErrorKind {
    UnknownNativeElement { name: String },
    UnknownComponent { name: String },
    DuplicateAttribute { name: String },
    InvalidAttribute { element: String, attribute: String },
    InvalidAttributeValue { element: String, attribute: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    kind: ValidationErrorKind,
    span: SourceSpan,
}

impl ValidationError {
    #[allow(dead_code)]
    pub(crate) const fn new(kind: ValidationErrorKind, span: SourceSpan) -> Self {
        Self { kind, span }
    }

    pub const fn kind(&self) -> &ValidationErrorKind {
        &self.kind
    }

    pub const fn span(&self) -> SourceSpan {
        self.span
    }
}

#[cfg(test)]
mod tests {
    use crate::span::{SourcePos, SourceSpan};

    use super::{ParseError, ParseErrorKind};

    #[test]
    fn parse_error_carries_kind_and_span() {
        let span = SourceSpan::new_unchecked(
            SourcePos::new_unchecked(1, 1, 0),
            SourcePos::new_unchecked(1, 5, 4),
        );
        let error = ParseError::new(
            ParseErrorKind::UnexpectedToken {
                expected: "tag name",
            },
            span,
        );

        assert!(matches!(
            error.kind(),
            ParseErrorKind::UnexpectedToken {
                expected: "tag name"
            }
        ));
        assert_eq!(error.span(), span);
    }
}
