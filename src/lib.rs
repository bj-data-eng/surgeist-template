//! Template and DSL-facing contracts for Surgeist.
//!
//! This crate owns template-layer contracts. Keep app-authoring concepts typed
//! and reusable, while leaving cross-crate lowering and host integration to the
//! root `surgeist` facade.

#![forbid(unsafe_code)]

mod ast;
mod error;
mod expr;
mod lexer;
mod name;
mod parser;
mod render;
mod span;
mod validate;

pub use error::{ParseError, ParseErrorKind, ValidationError, ValidationErrorKind};
pub use name::{AttributeName, ComponentName, NameError, NativeElementName, VariableName};
pub use span::{SourcePos, SourceSpan};

/// Crate identity string used by smoke tests and API artifacts.
pub const CRATE_NAME: &str = "surgeist-template";

#[cfg(test)]
mod tests {
    use super::CRATE_NAME;

    #[test]
    fn exposes_crate_identity() {
        assert_eq!(CRATE_NAME, "surgeist-template");
    }
}
