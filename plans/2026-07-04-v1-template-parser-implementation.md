# Surgeist Template V1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the first strict Surgeist template parser and typed template model, supporting HTML-like UI markup, brace-delimited expressions and controls, component/widget elements, and an initial Rust rendering contract.

**Architecture:** Implement a crate-owned strict parser instead of adapting html5ever. Source text is tokenized and parsed into typed authored Template IR with source spans, then validated into a semantic template document that can be lowered to generated Rust. Components/widgets are represented as UpperCamelCase element nodes with typed attributes and children.

**Tech Stack:** Rust 2024, no unsafe code, crate-local modules under `src/`, focused unit tests, baseline checks from `README.md`.

---

## Scope Decisions

V1 supports:

- Strict HTML-like element syntax with explicit open/close matching.
- Text nodes.
- Template comments using `{* ... *}`.
- Expression interpolation using `{$expr}` in text and quoted attribute values.
- Simple variable paths such as `$user.name` and `$items[0]`.
- Scalar literals: strings, integers, floats, booleans, and null.
- Basic expressions: parentheses, unary `!`, numeric `+ - * / %`, comparisons, `&&`, `||`.
- `{if}`, `{elseif}`, `{else}`, `{/if}`.
- `{foreach $items as $item}`, `{foreachelse}`, `{/foreach}`.
- Native lowercase elements such as `<div>`.
- Registered widget/component elements with UpperCamelCase names such as `<Panel>`.
- Boolean attributes and expression attributes such as `<Button disabled enabled={$is_enabled}>`.
- A first Rust rendering API that emits a structured Rust code string from validated IR.

V1 deliberately excludes:

- html5ever integration.
- Browser-style HTML error recovery.
- Template-side mutation, file inclusion, inheritance, dynamic evaluation, debugger directives, loops other than `foreach`, and template-defined functions.
- Function calls, method calls, object access, static access, namespaces, and request/global escape hatches.
- Arbitrary block control syntax inside an HTML start tag, such as `<Button {if $x}disabled{/if}>`.
- A runtime plugin system. Widget/component names are validated through a typed registry abstraction.

## File Structure

- `src/lib.rs`: public crate front door; re-export stable template API types.
- `src/span.rs`: source positions and spans.
- `src/error.rs`: typed parse, validation, and render errors.
- `src/name.rs`: checked names for variables, attributes, native elements, and components.
- `src/ast.rs`: authored template IR produced by parsing.
- `src/expr.rs`: expression AST and parser-facing expression helpers.
- `src/lexer.rs`: parser-owned source cursor for HTML/template mixed syntax.
- `src/parser.rs`: strict recursive-descent parser that builds authored IR.
- `src/validate.rs`: semantic validation, including native element versus component lookup.
- `src/render.rs`: initial Rust rendering contract from validated IR.
- `tests/template_v1.rs`: end-to-end parser/validator/render tests.

Each task below must go through the crate coordinator workflow in `AGENTS.md`: one worker for implementation, one separate reviewer for the scoped diff, reconciliation, focused checks, then a task-scoped commit.

## Task 1: Source Spans And Typed Errors

**Files:**
- Create: `src/span.rs`
- Create: `src/error.rs`
- Modify: `src/lib.rs`
- Create: `src/name.rs`
- Test: inline unit tests in `src/span.rs` and `src/error.rs`

- [ ] **Step 1: Write failing tests for source locations and errors**

Add this to `src/span.rs`:

```rust
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
```

Add this to `src/error.rs`:

```rust
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
        let error = ParseError::new(ParseErrorKind::UnexpectedToken { expected: "tag name" }, span);

        assert!(matches!(
            error.kind(),
            ParseErrorKind::UnexpectedToken { expected: "tag name" }
        ));
        assert_eq!(error.span(), span);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```sh
cargo test -p surgeist-template
```

Expected: FAIL because `src/span.rs`, `src/error.rs`, and their public types do not exist.

- [ ] **Step 3: Implement spans and errors**

Create `src/span.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourcePos {
    line: usize,
    column: usize,
    byte: usize,
}

impl SourcePos {
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
```

Create `src/error.rs`:

```rust
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

```

Create `src/name.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VariableName(String);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AttributeName(String);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NativeElementName(String);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ComponentName(String);

impl VariableName {
    pub fn try_new(name: impl Into<String>) -> Result<Self, NameError> {
        let name = name.into();
        if is_identifier(&name) {
            Ok(Self(name))
        } else {
            Err(NameError::InvalidVariableName { name })
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AttributeName {
    pub fn try_new(name: impl Into<String>) -> Result<Self, NameError> {
        let name = name.into();
        if is_kebab_or_identifier(&name) {
            Ok(Self(name))
        } else {
            Err(NameError::InvalidAttributeName { name })
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl NativeElementName {
    pub fn try_new(name: impl Into<String>) -> Result<Self, NameError> {
        let name = name.into();
        if is_kebab_or_identifier(&name) && name.starts_with(|ch: char| ch.is_ascii_lowercase()) {
            Ok(Self(name))
        } else {
            Err(NameError::InvalidNativeElementName { name })
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl ComponentName {
    pub fn try_new(name: impl Into<String>) -> Result<Self, NameError> {
        let name = name.into();
        if is_identifier(&name) && name.starts_with(|ch: char| ch.is_ascii_uppercase()) {
            Ok(Self(name))
        } else {
            Err(NameError::InvalidComponentName { name })
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameError {
    InvalidVariableName { name: String },
    InvalidAttributeName { name: String },
    InvalidNativeElementName { name: String },
    InvalidComponentName { name: String },
}

fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn is_kebab_or_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch == '-' || ch.is_ascii_alphanumeric())
}
```

Modify `src/lib.rs`:

```rust
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

/// Crate identity string used by smoke tests and API artifacts.
pub const CRATE_NAME: &str = "surgeist-template";
```

Create placeholder modules for imports until later tasks fill them:

```rust
// src/ast.rs
```

```rust
// src/expr.rs
```

```rust
// src/lexer.rs
```

```rust
// src/parser.rs
```

```rust
// src/render.rs
```

```rust
// src/validate.rs
```

- [ ] **Step 4: Run tests to verify they pass**

Run:

```sh
cargo test -p surgeist-template
```

Expected: PASS.

- [ ] **Step 5: Run task checks**

Run:

```sh
cargo fmt --check
cargo test -p surgeist-template
```

Expected: PASS.

- [ ] **Step 6: Review and commit**

Coordinator assigns a clean-context reviewer to inspect Task 1 against `guidance/surgeist-rust-modeling-guide.md`.

If reviewer is clean, run:

```sh
git add src/lib.rs src/span.rs src/error.rs src/name.rs src/ast.rs src/expr.rs src/lexer.rs src/parser.rs src/render.rs src/validate.rs
git commit -m "Add template source spans and errors"
```

## Task 2: Authored Template IR And Expression Model

**Files:**
- Modify: `src/ast.rs`
- Modify: `src/expr.rs`
- Modify: `src/lib.rs`
- Test: inline unit tests in `src/ast.rs` and `src/expr.rs`

- [ ] **Step 1: Write failing tests for typed authored IR**

Add this to `src/expr.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::{BinaryOp, Expr, Literal, PathField, PathIndex, PathSegment, VariablePath};

    #[test]
    fn variable_path_preserves_segments() {
        let path = VariablePath::try_new(
            "user",
            vec![
                PathSegment::Field(PathField::try_new("profile").expect("valid field")),
                PathSegment::Field(PathField::try_new("names").expect("valid field")),
                PathSegment::Index(PathIndex::new(0)),
            ],
        )
        .expect("valid path");

        assert_eq!(path.root(), "user");
        assert_eq!(path.segments().len(), 3);
        assert_eq!(path.segments()[2], PathSegment::Index(PathIndex::new(0)));
    }

    #[test]
    fn binary_expression_names_operator() {
        let expr = Expr::Binary {
            op: BinaryOp::Eq,
            left: Box::new(Expr::Variable(
                VariablePath::try_new("state", Vec::new()).expect("valid path"),
            )),
            right: Box::new(Expr::Literal(Literal::Bool(true))),
        };

        assert!(matches!(expr, Expr::Binary { op: BinaryOp::Eq, .. }));
    }

    #[test]
    fn variable_path_rejects_adjacent_indexes() {
        let result = VariablePath::try_new(
            "items",
            vec![PathSegment::Index(PathIndex::new(0)), PathSegment::Index(PathIndex::new(1))],
        );

        assert!(result.is_err());
    }

}
```

Add this to `src/ast.rs`:

```rust
#[cfg(test)]
mod tests {
    use crate::expr::{Expr, Literal};
    use crate::span::{SourcePos, SourceSpan};

    use super::{AttrValue, Attribute, ElementName, Node, TemplateDocument};

    #[test]
    fn document_preserves_component_element() {
        let span = SourceSpan::try_new(
            SourcePos::new_unchecked(1, 1, 0),
            SourcePos::new_unchecked(1, 8, 7),
        )
        .expect("valid span");
        let node = Node::element(
            ElementName::component("Panel").expect("valid component"),
            vec![Attribute::try_new(
                "title",
                AttrValue::Expression(Expr::Literal(Literal::String("Hello".to_owned()))),
                span,
            )
            .expect("valid attribute")],
            vec![Node::text("Body", span)],
            span,
        );
        let document = TemplateDocument::from_nodes(vec![node]);

        assert_eq!(document.nodes().len(), 1);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```sh
cargo test -p surgeist-template
```

Expected: FAIL because expression and IR types do not exist.

- [ ] **Step 3: Implement expression AST**

Replace `src/expr.rs` with:

```rust
use crate::name::{NameError, VariableName};

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Literal(Literal),
    Variable(VariablePath),
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Null,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Not,
    Neg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Or,
    And,
    Eq,
    Ne,
    Gt,
    Ge,
    Lt,
    Le,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariablePath {
    root: VariableName,
    segments: Vec<PathSegment>,
}

impl VariablePath {
    pub fn try_new(root: impl Into<String>, segments: Vec<PathSegment>) -> Result<Self, NameError> {
        let root = root.into();
        let mut previous_was_index = false;
        for segment in &segments {
            match segment {
                PathSegment::Field(_) => previous_was_index = false,
                PathSegment::Index(_) if previous_was_index => {
                    return Err(NameError::InvalidVariableName {
                        name: "adjacent indexes are not valid in v1 paths".to_owned(),
                    });
                }
                PathSegment::Index(_) => previous_was_index = true,
            }
        }
        Ok(Self {
            root: VariableName::try_new(root)?,
            segments,
        })
    }

    pub fn root(&self) -> &str {
        self.root.as_str()
    }

    pub fn segments(&self) -> &[PathSegment] {
        &self.segments
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathSegment {
    Field(PathField),
    Index(PathIndex),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathField(VariableName);

impl PathField {
    pub fn try_new(name: impl Into<String>) -> Result<Self, NameError> {
        Ok(Self(VariableName::try_new(name)?))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathIndex(u64);

impl PathIndex {
    pub fn new(index: u64) -> Self {
        Self(index)
    }

    pub fn get(self) -> u64 {
        self.0
    }
}
```

- [ ] **Step 4: Implement authored Template IR**

Replace `src/ast.rs` with:

```rust
use crate::expr::Expr;
use crate::name::{AttributeName, ComponentName, NameError, NativeElementName, VariableName};
use crate::span::SourceSpan;

#[derive(Debug, Clone, PartialEq)]
pub struct TemplateDocument {
    nodes: Vec<Node>,
}

impl TemplateDocument {
    pub(crate) fn from_nodes(nodes: Vec<Node>) -> Self {
        Self { nodes }
    }

    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Element(ElementNode),
    Text(TextNode),
    Interpolation(InterpolationNode),
    If(IfNode),
    ForEach(ForEachNode),
}

impl Node {
    pub(crate) fn element(
        name: ElementName,
        attributes: Vec<Attribute>,
        children: Vec<Node>,
        span: SourceSpan,
    ) -> Self {
        Self::Element(ElementNode {
            name,
            attributes,
            children,
            span,
        })
    }

    pub(crate) fn text(value: impl Into<String>, span: SourceSpan) -> Self {
        Self::Text(TextNode {
            value: value.into(),
            span,
        })
    }

    pub(crate) fn interpolation(expr: Expr, span: SourceSpan) -> Self {
        Self::Interpolation(InterpolationNode { expr, span })
    }

    pub(crate) fn if_block(branches: Vec<IfBranch>, else_children: Vec<Node>, span: SourceSpan) -> Self {
        Self::If(IfNode {
            branches,
            else_children,
            span,
        })
    }

    pub(crate) fn foreach(
        collection: Expr,
        item_name: VariableName,
        children: Vec<Node>,
        else_children: Vec<Node>,
        span: SourceSpan,
    ) -> Self {
        Self::ForEach(ForEachNode {
            collection,
            item_name,
            children,
            else_children,
            span,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ElementNode {
    name: ElementName,
    attributes: Vec<Attribute>,
    children: Vec<Node>,
    span: SourceSpan,
}

impl ElementNode {
    pub fn name(&self) -> &ElementName {
        &self.name
    }

    pub fn attributes(&self) -> &[Attribute] {
        &self.attributes
    }

    pub fn children(&self) -> &[Node] {
        &self.children
    }

    pub fn span(&self) -> SourceSpan {
        self.span
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextNode {
    value: String,
    span: SourceSpan,
}

impl TextNode {
    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn span(&self) -> SourceSpan {
        self.span
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct InterpolationNode {
    expr: Expr,
    span: SourceSpan,
}

impl InterpolationNode {
    pub fn expr(&self) -> &Expr {
        &self.expr
    }

    pub fn span(&self) -> SourceSpan {
        self.span
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfNode {
    branches: Vec<IfBranch>,
    else_children: Vec<Node>,
    span: SourceSpan,
}

impl IfNode {
    pub fn branches(&self) -> &[IfBranch] {
        &self.branches
    }

    pub fn else_children(&self) -> &[Node] {
        &self.else_children
    }

    pub fn span(&self) -> SourceSpan {
        self.span
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForEachNode {
    collection: Expr,
    item_name: VariableName,
    children: Vec<Node>,
    else_children: Vec<Node>,
    span: SourceSpan,
}

impl ForEachNode {
    pub fn collection(&self) -> &Expr {
        &self.collection
    }

    pub fn item_name(&self) -> &VariableName {
        &self.item_name
    }

    pub fn children(&self) -> &[Node] {
        &self.children
    }

    pub fn else_children(&self) -> &[Node] {
        &self.else_children
    }

    pub fn span(&self) -> SourceSpan {
        self.span
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ElementName {
    Native(NativeElementName),
    Component(ComponentName),
}

impl ElementName {
    pub fn native(name: impl Into<String>) -> Result<Self, NameError> {
        Ok(Self::Native(NativeElementName::try_new(name)?))
    }

    pub fn component(name: impl Into<String>) -> Result<Self, NameError> {
        Ok(Self::Component(ComponentName::try_new(name)?))
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Native(name) => name.as_str(),
            Self::Component(name) => name.as_str(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Attribute {
    name: AttributeName,
    value: AttrValue,
    span: SourceSpan,
}

impl Attribute {
    pub fn try_new(name: impl Into<String>, value: AttrValue, span: SourceSpan) -> Result<Self, NameError> {
        Ok(Self {
            name: AttributeName::try_new(name)?,
            value,
            span,
        })
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn value(&self) -> &AttrValue {
        &self.value
    }

    pub fn span(&self) -> SourceSpan {
        self.span
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AttrValue {
    Bool,
    Static(String),
    Expression(Expr),
    Interpolated(Vec<AttrPart>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum AttrPart {
    Text(String),
    Expr(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfBranch {
    condition: Expr,
    children: Vec<Node>,
}

impl IfBranch {
    pub fn new(condition: Expr, children: Vec<Node>) -> Self {
        Self { condition, children }
    }

    pub fn condition(&self) -> &Expr {
        &self.condition
    }

    pub fn children(&self) -> &[Node] {
        &self.children
    }
}
```

- [ ] **Step 5: Re-export stable front-door API**

Modify `src/lib.rs` to add:

```rust
pub use ast::{
    AttrPart, AttrValue, Attribute, ElementName, ElementNode, ForEachNode, IfBranch, IfNode,
    InterpolationNode, Node, TemplateDocument, TextNode,
};
pub use error::{ParseError, ParseErrorKind, ValidationError, ValidationErrorKind};
pub use expr::{BinaryOp, Expr, Literal, PathField, PathIndex, PathSegment, UnaryOp, VariablePath};
pub use name::{AttributeName, ComponentName, NameError, NativeElementName, VariableName};
pub use span::{SourcePos, SourceSpan};
```

- [ ] **Step 6: Run tests to verify they pass**

Run:

```sh
cargo test -p surgeist-template
```

Expected: PASS.

- [ ] **Step 7: Run task checks**

Run:

```sh
cargo fmt --check
cargo test -p surgeist-template
```

Expected: PASS.

- [ ] **Step 8: Review and commit**

Coordinator assigns a separate reviewer for Task 2. Reviewer checks that IR types encode phases and ownership, avoid public mutable fields, and keep invalid states harder to express.

If reviewer is clean, run:

```sh
git add src/lib.rs src/ast.rs src/expr.rs
git commit -m "Add authored template IR"
```

## Task 3: Strict Mixed HTML/Template Parser

**Files:**
- Modify: `src/parser.rs`
- Modify: `src/lexer.rs`
- Modify: `src/expr.rs`
- Test: `tests/template_v1.rs`

- [ ] **Step 1: Write failing parser tests**

Create `tests/template_v1.rs` with:

```rust
use surgeist_template::{parse_template, Node};

#[test]
fn parses_native_element_with_interpolation() {
    let document = parse_template("<div>Hello {$user.name}</div>").expect("template parses");

    assert_eq!(document.nodes().len(), 1);
    let Node::Element(element) = &document.nodes()[0] else {
        panic!("expected element");
    };
    assert_eq!(element.name().as_str(), "div");
    assert_eq!(element.children().len(), 2);
}

#[test]
fn parses_component_with_attrs_and_children() {
    let document = parse_template(
        r#"<Panel title={$project.name} collapsible><Text>{$project.summary}</Text></Panel>"#,
    )
    .expect("template parses");

    let Node::Element(element) = &document.nodes()[0] else {
        panic!("expected component element");
    };

    assert_eq!(element.name().as_str(), "Panel");
    assert_eq!(element.attributes().len(), 2);
    assert_eq!(element.children().len(), 1);
}

#[test]
fn rejects_mismatched_close_tag() {
    let error = parse_template("<div></span>").expect_err("mismatched close tag fails");

    assert!(format!("{error:?}").contains("MismatchedCloseTag"));
}

#[test]
fn rejects_invalid_element_name_as_name_error() {
    let error = parse_template("<my:tag></my:tag>").expect_err("invalid element name fails");

    assert!(format!("{error:?}").contains("InvalidName"));
}

#[test]
fn reports_multiline_and_invalid_header_spans() {
    let document = parse_template("<Panel>\n  Héllo\n</Panel>").expect("template parses");
    let Node::Element(element) = &document.nodes()[0] else {
        panic!("expected element");
    };
    let Node::Text(text) = &element.children()[0] else {
        panic!("expected text");
    };
    assert_eq!(text.span().start().line(), 1);
    assert_eq!(text.span().end().line(), 3);

    let error = parse_template("{if $items[abc]}<Panel />{/if}")
        .expect_err("invalid control header expression fails");
    assert_eq!(error.span().start().line(), 1);
    assert!(error.span().len_bytes() >= "$items[abc]".len());

    let foreach_error = parse_template("{foreach $items as $é}<Panel />{/foreach}")
        .expect_err("invalid foreach item fails");
    assert_eq!(foreach_error.span().start().column(), 20);
    assert_eq!(foreach_error.span().end().column(), 22);
    assert_eq!(foreach_error.span().len_bytes(), "$é".len());

    let missing_item_dollar = parse_template("{foreach $items as item}<Panel />{/foreach}")
        .expect_err("foreach item must use a dollar-prefixed variable");
    assert_eq!(missing_item_dollar.span().start().column(), 20);

    let multiple_as = parse_template("{foreach $items as $item as $other}<Panel />{/foreach}")
        .expect_err("foreach header accepts only one as clause");
    assert!(format!("{multiple_as:?}").contains("InvalidExpression"));
}

#[test]
fn rejects_stray_or_unknown_template_tags() {
    let stray = parse_template("{else}<Text>nope</Text>").expect_err("stray else fails");
    let unknown = parse_template("{foo bar}<Text>nope</Text>").expect_err("unknown tag fails");
    let quoted_attr = parse_template(r#"<Panel title="{foo}" />"#).expect_err("unknown quoted attr tag fails");
    let unquoted_attr = parse_template(r#"<Panel title={foo} />"#).expect_err("unknown unquoted attr tag fails");
    let embedded_attr = parse_template(r#"<Panel title=foo{bar} />"#).expect_err("embedded attr tag fails");
    let trailing_expr_attr =
        parse_template(r#"<Panel title={$foo}bar />"#).expect_err("trailing expression attr text fails");

    assert!(format!("{stray:?}").contains("StrayTemplateTag"));
    assert!(format!("{unknown:?}").contains("UnsupportedTemplateTag"));
    assert!(format!("{quoted_attr:?}").contains("UnsupportedTemplateTag"));
    assert!(format!("{unquoted_attr:?}").contains("UnsupportedTemplateTag"));
    assert!(format!("{embedded_attr:?}").contains("UnsupportedTemplateTag"));
    assert!(format!("{trailing_expr_attr:?}").contains("UnexpectedToken"));
}

#[test]
fn parses_if_and_foreach_blocks() {
    let document = parse_template(
        r#"{if $visible}<Panel>{foreach $items as $item}<Text>{$item.name}</Text>{foreachelse}<Text>Empty</Text>{/foreach}</Panel>{else}<Text>Hidden</Text>{/if}"#,
    )
    .expect("template parses");

    assert_eq!(document.nodes().len(), 1);
    assert!(matches!(document.nodes()[0], Node::If(_)));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```sh
cargo test -p surgeist-template --test template_v1
```

Expected: FAIL because `parse_template` is not defined.

- [ ] **Step 3: Implement expression parsing helpers**

Replace `src/expr.rs` with the Task 2 types plus these parsing helpers:

```rust
pub fn parse_simple_expr(source: &str) -> Result<Expr, &'static str> {
    let trimmed = source.trim();
    if trimmed.is_empty() {
        return Err("empty expression");
    }
    if trimmed == "true" {
        return Ok(Expr::Literal(Literal::Bool(true)));
    }
    if trimmed == "false" {
        return Ok(Expr::Literal(Literal::Bool(false)));
    }
    if trimmed == "null" {
        return Ok(Expr::Literal(Literal::Null));
    }
    if let Some(path) = trimmed.strip_prefix('$') {
        return parse_variable_path(path).map(Expr::Variable);
    }
    if let Ok(value) = trimmed.parse::<i64>() {
        return Ok(Expr::Literal(Literal::Int(value)));
    }
    if let Ok(value) = trimmed.parse::<f64>() {
        return Ok(Expr::Literal(Literal::Float(value)));
    }
    if let Some(value) = parse_quoted_string(trimmed) {
        return Ok(Expr::Literal(Literal::String(value)));
    }

    Err("unsupported expression")
}

fn parse_variable_path(source: &str) -> Result<VariablePath, &'static str> {
    let mut parts = source.split('.');
    let root_part = parts.next().ok_or("missing variable root")?;
    let (root, root_suffix) = root_part.split_once('[').map_or((root_part, None), |(root, suffix)| {
        (root, Some(suffix))
    });
    if !is_ident(root) {
        return Err("invalid variable root");
    }
    let mut segments = Vec::new();
    if let Some(suffix) = root_suffix {
        let index = suffix.strip_suffix(']').ok_or("invalid index segment")?;
        push_index_suffix(index, &mut segments)?;
    }
    for part in parts {
        push_path_part(part, &mut segments)?;
    }
    VariablePath::try_new(root, segments).map_err(|_| "invalid variable root")
}

fn push_path_part(source: &str, segments: &mut Vec<PathSegment>) -> Result<(), &'static str> {
    let Some((field, rest)) = source.split_once('[') else {
        if !is_ident(source) {
            return Err("invalid path segment");
        }
        let field = crate::expr::PathField::try_new(source).map_err(|_| "invalid path segment")?;
        segments.push(PathSegment::Field(field));
        return Ok(());
    };
    if field.is_empty() || !is_ident(field) {
        return Err("invalid path segment");
    }
    let field = crate::expr::PathField::try_new(field).map_err(|_| "invalid path segment")?;
    segments.push(PathSegment::Field(field));
    let index = rest.strip_suffix(']').ok_or("invalid index segment")?;
    push_index_suffix(index, segments)?;
    Ok(())
}

fn push_index_suffix(source: &str, segments: &mut Vec<PathSegment>) -> Result<(), &'static str> {
    let index = source.parse::<u64>().map_err(|_| "invalid index segment")?;
    segments.push(PathSegment::Index(crate::expr::PathIndex::new(index)));
    Ok(())
}

fn parse_quoted_string(source: &str) -> Option<String> {
    let value = source.strip_prefix('"')?.strip_suffix('"')?;
    Some(value.to_owned())
}

fn is_ident(source: &str) -> bool {
    let mut chars = source.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}
```

This helper intentionally supports only variables, scalar literals, dotted paths, and numeric bracket indexes. A later task expands operators.

- [ ] **Step 4: Implement parser-owned source cursor**

Replace `src/lexer.rs` with:

```rust
use crate::error::{ParseError, ParseErrorKind};
use crate::span::{SourcePos, SourceSpan};

#[derive(Debug)]
pub(crate) struct SourceCursor<'a> {
    source: &'a str,
    byte: usize,
    line: usize,
    column: usize,
}

impl<'a> SourceCursor<'a> {
    pub(crate) fn new(source: &'a str) -> Self {
        Self {
            source,
            byte: 0,
            line: 1,
            column: 1,
        }
    }

    pub(crate) fn pos(&self) -> SourcePos {
        SourcePos::new_unchecked(self.line, self.column, self.byte)
    }

    pub(crate) fn is_eof(&self) -> bool {
        self.byte >= self.source.len()
    }

    pub(crate) fn starts_with(&self, prefix: &str) -> bool {
        self.source[self.byte..].starts_with(prefix)
    }

    pub(crate) fn peek(&self) -> Option<char> {
        self.source[self.byte..].chars().next()
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

    pub(crate) fn expect(&mut self, expected: &'static str) -> Result<(), ParseError> {
        if !self.starts_with(expected) {
            return Err(self.error(ParseErrorKind::UnexpectedToken { expected }));
        }
        for _ in expected.chars() {
            self.bump();
        }
        Ok(())
    }

    pub(crate) fn skip_ws(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.bump();
        }
    }

    pub(crate) fn take_until(&mut self, needle: &str) -> Result<&'a str, ParseError> {
        let start = self.byte;
        let Some(relative_end) = self.source[self.byte..].find(needle) else {
            return Err(self.error(ParseErrorKind::UnexpectedEof));
        };
        let end = self.byte + relative_end;
        while self.byte < end {
            self.bump();
        }
        Ok(&self.source[start..end])
    }

    pub(crate) fn error(&self, kind: ParseErrorKind) -> ParseError {
        ParseError::new(kind, SourceSpan::new_unchecked(self.pos(), self.pos()))
    }
}
```

- [ ] **Step 5: Implement parser front door and strict element parsing**

Replace `src/parser.rs` with:

```rust
use crate::ast::{AttrPart, AttrValue, Attribute, ElementName, IfBranch, Node, TemplateDocument};
use crate::error::{ParseError, ParseErrorKind};
use crate::expr::parse_simple_expr;
use crate::lexer::SourceCursor;
use crate::name::VariableName;
use crate::span::{SourcePos, SourceSpan};

pub fn parse_template(source: &str) -> Result<TemplateDocument, ParseError> {
    Parser::new(source).parse_document()
}

struct Parser<'a> {
    cursor: SourceCursor<'a>,
}

impl<'a> Parser<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            cursor: SourceCursor::new(source),
        }
    }

    fn parse_document(mut self) -> Result<TemplateDocument, ParseError> {
        let nodes = self.parse_nodes(None)?;
        Ok(TemplateDocument::from_nodes(nodes))
    }

    fn parse_nodes(&mut self, closing: Option<&str>) -> Result<Vec<Node>, ParseError> {
        let mut nodes = Vec::new();
        while !self.is_eof() {
            if let Some(name) = closing {
                if self.starts_with("</") {
                    let close = self.parse_close_tag()?;
                    if close == name {
                        return Ok(nodes);
                    }
                    return Err(self.error(ParseErrorKind::MismatchedCloseTag {
                        expected: name.to_owned(),
                        found: close,
                    }));
                }
            }
            if self.starts_with("{*") {
                self.parse_comment()?;
            } else if self.starts_with("{$") {
                nodes.push(self.parse_interpolation()?);
            } else if self.starts_with("{if ") {
                nodes.push(self.parse_if()?);
            } else if self.starts_with("{foreach ") {
                nodes.push(self.parse_foreach()?);
            } else if self.starts_with("{") {
                return Err(self.parse_unexpected_template_tag());
            } else if self.starts_with("<") {
                nodes.push(self.parse_element()?);
            } else {
                nodes.push(self.parse_text());
            }
        }
        if let Some(name) = closing {
            return Err(self.error(ParseErrorKind::UnclosedElement {
                name: name.to_owned(),
            }));
        }
        Ok(nodes)
    }

    fn parse_element(&mut self) -> Result<Node, ParseError> {
        let start = self.pos();
        self.expect("<")?;
        let name_start = self.pos();
        let raw_name = self.take_element_name(name_start)?;
        let name = classify_element_name(&raw_name, name_start, self.pos())?;
        let mut attributes = Vec::new();
        loop {
            self.skip_ws();
            if self.starts_with("/>") {
                self.expect("/>")?;
                let span = SourceSpan::new_unchecked(start, self.pos());
                return Ok(Node::element(
                    name,
                    attributes,
                    Vec::new(),
                    span,
                ));
            }
            if self.starts_with(">") {
                self.expect(">")?;
                break;
            }
            attributes.push(self.parse_attribute()?);
        }
        let children = self.parse_nodes(Some(&raw_name))?;
        let span = SourceSpan::new_unchecked(start, self.pos());
        Ok(Node::element(
            name,
            attributes,
            children,
            span,
        ))
    }

    fn parse_attribute(&mut self) -> Result<Attribute, ParseError> {
        let start = self.pos();
        let name = self.take_name()?;
        self.skip_ws();
        if !self.starts_with("=") {
            return Attribute::try_new(name, AttrValue::Bool, SourceSpan::new_unchecked(start, self.pos()))
                .map_err(|_| self.error(ParseErrorKind::InvalidName { name: "attribute".to_owned() }));
        }
        self.expect("=")?;
        self.skip_ws();
        let value = if self.starts_with("{$") {
            let expr = self.parse_braced_expr()?;
            self.expect_attr_value_boundary()?;
            AttrValue::Expression(expr)
        } else if self.starts_with("{") {
            return Err(self.parse_unexpected_template_tag());
        } else if self.starts_with("\"") {
            self.parse_quoted_attr()?
        } else {
            AttrValue::Static(self.take_unquoted_value()?)
        };
        Attribute::try_new(name, value, SourceSpan::new_unchecked(start, self.pos()))
            .map_err(|_| self.error(ParseErrorKind::InvalidName { name: "attribute".to_owned() }))
    }

    fn parse_interpolation(&mut self) -> Result<Node, ParseError> {
        let start = self.pos();
        let expr = self.parse_braced_expr()?;
        Ok(Node::interpolation(expr, SourceSpan::new_unchecked(start, self.pos())))
    }

    fn expect_attr_value_boundary(&self) -> Result<(), ParseError> {
        if self.is_eof()
            || self.peek().is_some_and(char::is_whitespace)
            || self.starts_with(">")
            || self.starts_with("/>")
        {
            Ok(())
        } else {
            Err(self.error(ParseErrorKind::UnexpectedToken {
                expected: "attribute value boundary",
            }))
        }
    }

    fn parse_braced_expr(&mut self) -> Result<crate::expr::Expr, ParseError> {
        self.expect("{")?;
        let expr_start = self.pos();
        let expr_source = self.take_until("}")?;
        let expr_end = self.pos();
        self.expect("}")?;
        parse_simple_expr(expr_source).map_err(|reason| {
            ParseError::new(
                ParseErrorKind::InvalidExpression { reason },
                SourceSpan::new_unchecked(expr_start, expr_end),
            )
        })
    }

    fn parse_expr_source(
        &self,
        expr_source: &str,
        expr_start: SourcePos,
        expr_end: SourcePos,
    ) -> Result<crate::expr::Expr, ParseError> {
        parse_simple_expr(expr_source).map_err(|reason| {
            ParseError::new(
                ParseErrorKind::InvalidExpression { reason },
                SourceSpan::new_unchecked(expr_start, expr_end),
            )
        })
    }

    fn parse_quoted_attr(&mut self) -> Result<AttrValue, ParseError> {
        self.expect("\"")?;
        let mut parts = Vec::new();
        let mut text = String::new();
        while !self.is_eof() && !self.starts_with("\"") {
            if self.starts_with("{$") {
                if !text.is_empty() {
                    parts.push(AttrPart::Text(std::mem::take(&mut text)));
                }
                parts.push(AttrPart::Expr(self.parse_braced_expr()?));
            } else if self.starts_with("{") {
                return Err(self.parse_unexpected_template_tag());
            } else {
                text.push(self.bump().expect("not eof"));
            }
        }
        self.expect("\"")?;
        if parts.is_empty() {
            return Ok(AttrValue::Static(text));
        }
        if !text.is_empty() {
            parts.push(AttrPart::Text(text));
        }
        Ok(AttrValue::Interpolated(parts))
    }

    fn parse_if(&mut self) -> Result<Node, ParseError> {
        let start = self.pos();
        self.expect("{if ")?;
        let condition_start = self.pos();
        let condition_source = self.take_until("}")?;
        let condition_end = self.pos();
        self.expect("}")?;
        let condition = self.parse_expr_source(condition_source, condition_start, condition_end)?;
        let children = self.parse_until_template_tags(&["{elseif ", "{else}", "{/if}"])?;
        let mut branches = vec![IfBranch::new(condition, children)];
        while self.starts_with("{elseif ") {
            self.expect("{elseif ")?;
            let condition_start = self.pos();
            let condition_source = self.take_until("}")?;
            let condition_end = self.pos();
            self.expect("}")?;
            let condition = self.parse_expr_source(condition_source, condition_start, condition_end)?;
            let children = self.parse_until_template_tags(&["{elseif ", "{else}", "{/if}"])?;
            branches.push(IfBranch::new(condition, children));
        }
        let else_children = if self.starts_with("{else}") {
            self.expect("{else}")?;
            self.parse_until_template_tags(&["{/if}"])?
        } else {
            Vec::new()
        };
        self.expect("{/if}")?;
        Ok(Node::if_block(
            branches,
            else_children,
            SourceSpan::new_unchecked(start, self.pos()),
        ))
    }

    fn parse_foreach(&mut self) -> Result<Node, ParseError> {
        let start = self.pos();
        self.expect("{foreach ")?;
        let header_start = self.pos();
        let header = self.take_until("}")?;
        let header_end = self.pos();
        self.expect("}")?;
        let header = parse_foreach_header(header, header_start, header_end)?;
        let collection =
            self.parse_expr_source(header.collection_source, header_start, header.collection_end)?;
        let Some(item_name_source) = header.item_source.strip_prefix('$') else {
            return Err(ParseError::new(
                ParseErrorKind::InvalidName { name: header.item_source.to_owned() },
                SourceSpan::new_unchecked(header.item_start, header.item_end),
            ));
        };
        if item_name_source.starts_with('$') {
            return Err(ParseError::new(
                ParseErrorKind::InvalidName { name: header.item_source.to_owned() },
                SourceSpan::new_unchecked(header.item_start, header.item_end),
            ));
        }
        let item_name = VariableName::try_new(item_name_source).map_err(|_| {
            ParseError::new(
                ParseErrorKind::InvalidName { name: header.item_source.to_owned() },
                SourceSpan::new_unchecked(header.item_start, header.item_end),
            )
        })?;
        let children = self.parse_until_template_tags(&["{foreachelse}", "{/foreach}"])?;
        let else_children = if self.starts_with("{foreachelse}") {
            self.expect("{foreachelse}")?;
            self.parse_until_template_tags(&["{/foreach}"])?
        } else {
            Vec::new()
        };
        self.expect("{/foreach}")?;
        Ok(Node::foreach(
            collection,
            item_name,
            children,
            else_children,
            SourceSpan::new_unchecked(start, self.pos()),
        ))
    }

    fn parse_until_template_tags(&mut self, sentinels: &[&str]) -> Result<Vec<Node>, ParseError> {
        let mut nodes = Vec::new();
        while !self.is_eof() && !sentinels.iter().any(|sentinel| self.starts_with(sentinel)) {
            if self.starts_with("{*") {
                self.parse_comment()?;
            } else if self.starts_with("{$") {
                nodes.push(self.parse_interpolation()?);
            } else if self.starts_with("{if ") {
                nodes.push(self.parse_if()?);
            } else if self.starts_with("{foreach ") {
                nodes.push(self.parse_foreach()?);
            } else if self.starts_with("{") {
                return Err(self.parse_unexpected_template_tag());
            } else if self.starts_with("<") {
                nodes.push(self.parse_element()?);
            } else {
                nodes.push(self.parse_text());
            }
        }
        Ok(nodes)
    }

    fn parse_text(&mut self) -> Node {
        let start = self.pos();
        let mut value = String::new();
        while !self.is_eof()
            && !self.starts_with("<")
            && !self.starts_with("{$")
            && !self.starts_with("{if ")
            && !self.starts_with("{foreach ")
            && !self.starts_with("{*")
            && !self.starts_with("{")
        {
            value.push(self.bump().expect("not eof"));
        }
        Node::text(value, SourceSpan::new_unchecked(start, self.pos()))
    }

    fn parse_close_tag(&mut self) -> Result<String, ParseError> {
        self.expect("</")?;
        let name = self.take_name()?;
        self.skip_ws();
        self.expect(">")?;
        Ok(name)
    }

    fn parse_comment(&mut self) -> Result<(), ParseError> {
        self.expect("{*")?;
        self.take_until("*}")?;
        self.expect("*}")?;
        Ok(())
    }

    fn parse_unexpected_template_tag(&mut self) -> ParseError {
        let start = self.pos();
        let tag = match self.take_until("}") {
            Ok(body) => {
                let _ = self.expect("}");
                format!("{{{body}}}")
            }
            Err(_) => "{".to_owned(),
        };
        let span = SourceSpan::new_unchecked(start, self.pos());
        let kind = if matches!(
            tag.as_str(),
            "{else}" | "{/if}" | "{foreachelse}" | "{/foreach}" | "{/for}" | "{/while}"
        ) {
            ParseErrorKind::StrayTemplateTag { tag }
        } else {
            ParseErrorKind::UnsupportedTemplateTag { tag }
        };
        ParseError::new(kind, span)
    }

    fn take_name(&mut self) -> Result<String, ParseError> {
        let mut name = String::new();
        while let Some(ch) = self.peek() {
            if ch == '_' || ch == '-' || ch.is_ascii_alphanumeric() {
                name.push(self.bump().expect("peeked"));
            } else {
                break;
            }
        }
        if name.is_empty() {
            return Err(self.error(ParseErrorKind::UnexpectedToken { expected: "name" }));
        }
        Ok(name)
    }

    fn take_element_name(&mut self, start: SourcePos) -> Result<String, ParseError> {
        let mut name = String::new();
        while let Some(ch) = self.peek() {
            if ch.is_whitespace() || ch == '>' || ch == '/' {
                break;
            }
            name.push(self.bump().expect("peeked"));
        }
        if name.is_empty() {
            return Err(self.error(ParseErrorKind::UnexpectedToken { expected: "element name" }));
        }
        if !name.chars().all(|ch| ch == '_' || ch == '-' || ch.is_ascii_alphanumeric()) {
            return Err(ParseError::new(
                ParseErrorKind::InvalidName { name },
                SourceSpan::new_unchecked(start, self.pos()),
            ));
        }
        Ok(name)
    }

    fn take_unquoted_value(&mut self) -> Result<String, ParseError> {
        let mut value = String::new();
        while let Some(ch) = self.peek() {
            if ch.is_whitespace() || ch == '>' {
                break;
            }
            if ch == '{' {
                return Err(self.parse_unexpected_template_tag());
            }
            value.push(self.bump().expect("peeked"));
        }
        if value.is_empty() {
            return Err(self.error(ParseErrorKind::UnexpectedToken { expected: "attribute value" }));
        }
        Ok(value)
    }

    fn take_until(&mut self, needle: &str) -> Result<&'a str, ParseError> {
        self.cursor.take_until(needle)
    }

    fn expect(&mut self, expected: &'static str) -> Result<(), ParseError> {
        self.cursor.expect(expected)
    }

    fn skip_ws(&mut self) {
        self.cursor.skip_ws();
    }

    fn starts_with(&self, prefix: &str) -> bool {
        self.cursor.starts_with(prefix)
    }

    fn is_eof(&self) -> bool {
        self.cursor.is_eof()
    }

    fn peek(&self) -> Option<char> {
        self.cursor.peek()
    }

    fn bump(&mut self) -> Option<char> {
        self.cursor.bump()
    }

    fn pos(&self) -> SourcePos {
        self.cursor.pos()
    }

    fn error(&self, kind: ParseErrorKind) -> ParseError {
        self.cursor.error(kind)
    }
}

fn advance_pos_from(start: SourcePos, source: &str) -> SourcePos {
    let mut line = start.line();
    let mut column = start.column();
    let mut byte = start.byte();
    for ch in source.chars() {
        byte += ch.len_utf8();
        if ch == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }
    SourcePos::new_unchecked(line, column, byte)
}

struct ForeachHeader<'a> {
    collection_source: &'a str,
    collection_end: SourcePos,
    item_source: &'a str,
    item_start: SourcePos,
    item_end: SourcePos,
}

fn parse_foreach_header<'a>(
    header: &'a str,
    header_start: SourcePos,
    header_end: SourcePos,
) -> Result<ForeachHeader<'a>, ParseError> {
    let Some(as_offset) = header.find(" as ") else {
        return Err(ParseError::new(
            ParseErrorKind::InvalidExpression { reason: "expected foreach as" },
            SourceSpan::new_unchecked(header_start, header_end),
        ));
    };
    let item_offset = as_offset + " as ".len();
    if header[item_offset..].contains(" as ") {
        return Err(ParseError::new(
            ParseErrorKind::InvalidExpression { reason: "multiple foreach as clauses" },
            SourceSpan::new_unchecked(header_start, header_end),
        ));
    }
    let collection_source = &header[..as_offset];
    let item_source = &header[item_offset..];
    if collection_source.is_empty() || collection_source != collection_source.trim() {
        return Err(ParseError::new(
            ParseErrorKind::InvalidExpression { reason: "invalid foreach collection" },
            SourceSpan::new_unchecked(header_start, advance_pos_from(header_start, collection_source)),
        ));
    }
    let item_start = advance_pos_from(header_start, &header[..item_offset]);
    let item_end = advance_pos_from(item_start, item_source);
    if item_source.is_empty()
        || item_source != item_source.trim()
        || item_source.chars().any(char::is_whitespace)
    {
        return Err(ParseError::new(
            ParseErrorKind::InvalidName { name: item_source.to_owned() },
            SourceSpan::new_unchecked(item_start, item_end),
        ));
    }
    Ok(ForeachHeader {
        collection_source,
        collection_end: advance_pos_from(header_start, collection_source),
        item_source,
        item_start,
        item_end,
    })
}

fn classify_element_name(
    name: &str,
    start: SourcePos,
    end: SourcePos,
) -> Result<ElementName, ParseError> {
    let span = SourceSpan::new_unchecked(start, end);
    let Some(first) = name.chars().next() else {
        return Err(ParseError::new(
            ParseErrorKind::InvalidName { name: name.to_owned() },
            span,
        ));
    };
    if first.is_ascii_uppercase() {
        ElementName::component(name).map_err(|_| {
            ParseError::new(
                ParseErrorKind::InvalidName { name: name.to_owned() },
                span,
            )
        })
    } else {
        ElementName::native(name).map_err(|_| {
            ParseError::new(
                ParseErrorKind::InvalidName { name: name.to_owned() },
                span,
            )
        })
    }
}
```

Keep `SourceCursor` `pub(crate)` and avoid exposing tokenization details from `lib.rs`; this preserves `parse_template` as the public front door.

- [ ] **Step 6: Re-export parse front door**

Modify `src/lib.rs` to add:

```rust
pub use parser::parse_template;
```

- [ ] **Step 7: Run tests to verify they pass**

Run:

```sh
cargo test -p surgeist-template --test template_v1
```

Expected: PASS.

- [ ] **Step 8: Run task checks**

Run:

```sh
cargo fmt --check
cargo test -p surgeist-template
```

Expected: PASS.

- [ ] **Step 9: Review and commit**

Coordinator assigns a separate reviewer for Task 3. Reviewer checks strict parsing behavior, span propagation, and that no browser-style recovery path was introduced.

If reviewer is clean, run:

```sh
git add src/lib.rs src/parser.rs src/lexer.rs src/expr.rs tests/template_v1.rs
git commit -m "Add strict template parser"
```

## Task 4: Semantic Validation And Component Registry

**Files:**
- Modify: `src/validate.rs`
- Modify: `src/lib.rs`
- Test: `tests/template_v1.rs`

- [ ] **Step 1: Write failing validation tests**

Append to `tests/template_v1.rs`:

```rust
use surgeist_template::{
    validate_template, AttributeKind, AttributeRule, AttributeSpec, ComponentRegistry,
    ComponentSpec, NativeElementRegistry, NativeElementSpec, RegistryError,
};

#[test]
fn validates_native_and_component_names() {
    let document = parse_template("<Panel><div>Body</div></Panel>").expect("template parses");
    let native = NativeElementRegistry::try_from_specs(vec![
        surgeist_template::NativeElementSpec::try_new("div", Vec::new()).expect("valid spec"),
    ])
    .expect("valid native registry");
    let components = ComponentRegistry::try_from_specs(vec![
        ComponentSpec::try_new("Panel", Vec::new()).expect("valid spec"),
    ])
    .expect("valid component registry");

    let validated = validate_template(&document, &native, &components).expect("template validates");

    assert_eq!(validated.nodes().len(), 1);
}

#[test]
fn rejects_unknown_component() {
    let document = parse_template("<MissingWidget />").expect("template parses");
    let native = NativeElementRegistry::try_from_specs(vec![
        surgeist_template::NativeElementSpec::try_new("div", Vec::new()).expect("valid spec"),
    ])
    .expect("valid native registry");
    let components = ComponentRegistry::try_from_specs(vec![
        ComponentSpec::try_new("Panel", Vec::new()).expect("valid spec"),
    ])
    .expect("valid component registry");

    let error = validate_template(&document, &native, &components).expect_err("unknown component fails");

    assert!(format!("{error:?}").contains("UnknownComponent"));
}

#[test]
fn rejects_duplicate_authored_attributes() {
    let document = parse_template(r#"<Panel title="one" title="two" />"#).expect("template parses");
    let native = NativeElementRegistry::try_from_specs(Vec::new()).expect("valid native registry");
    let components = ComponentRegistry::try_from_specs(vec![
        ComponentSpec::try_new(
            "Panel",
            vec![AttributeSpec::try_new(
                "title",
                AttributeRule::one(AttributeKind::Static),
            )
            .expect("valid attr spec")],
        )
        .expect("valid spec"),
    ])
    .expect("valid component registry");

    let error = validate_template(&document, &native, &components).expect_err("duplicate attrs fail");

    assert!(format!("{error:?}").contains("DuplicateAttribute"));
}

#[test]
fn rejects_unknown_attribute_and_invalid_value_kind() {
    let native = NativeElementRegistry::try_from_specs(Vec::new()).expect("valid native registry");
    let components = ComponentRegistry::try_from_specs(vec![
        ComponentSpec::try_new(
            "Panel",
            vec![AttributeSpec::try_new(
                "title",
                AttributeRule::one(AttributeKind::Expression),
            )
            .expect("valid attr spec")],
        )
        .expect("valid spec"),
    ])
    .expect("valid component registry");

    let unknown = parse_template(r#"<Panel missing="value" />"#).expect("template parses");
    let unknown_error =
        validate_template(&unknown, &native, &components).expect_err("unknown attr fails");
    assert!(format!("{unknown_error:?}").contains("InvalidAttribute"));

    let wrong_kind = parse_template(r#"<Panel title="static" />"#).expect("template parses");
    let wrong_kind_error =
        validate_template(&wrong_kind, &native, &components).expect_err("wrong attr kind fails");
    assert!(format!("{wrong_kind_error:?}").contains("InvalidAttributeValue"));
}

#[test]
fn supports_multi_kind_attribute_rules_and_rejects_duplicate_attr_specs() {
    let duplicate = ComponentSpec::try_new(
        "Panel",
        vec![
            AttributeSpec::try_new("title", AttributeRule::one(AttributeKind::Static))
                .expect("valid attr spec"),
            AttributeSpec::try_new("title", AttributeRule::one(AttributeKind::Expression))
                .expect("valid attr spec"),
        ],
    )
    .expect_err("duplicate attr spec fails");
    assert!(matches!(duplicate, RegistryError::DuplicateAttributeSpec { .. }));

    let document = parse_template(r#"<Panel title="Hello {$user.name}" />"#).expect("template parses");
    let native = NativeElementRegistry::try_from_specs(vec![
        NativeElementSpec::try_new("div", Vec::new()).expect("valid spec"),
    ])
    .expect("valid native registry");
    let components = ComponentRegistry::try_from_specs(vec![
        ComponentSpec::try_new(
            "Panel",
            vec![AttributeSpec::try_new(
                "title",
                AttributeRule::any(
                    AttributeKind::Static,
                    [AttributeKind::Interpolated, AttributeKind::Expression],
                ),
            )
            .expect("valid attr spec")],
        )
        .expect("valid spec"),
    ])
    .expect("valid component registry");

    validate_template(&document, &native, &components).expect("interpolated title validates");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```sh
cargo test -p surgeist-template --test template_v1
```

Expected: FAIL because validation types and `validate_template` are not defined.

- [ ] **Step 3: Implement validation types**

Replace `src/validate.rs` with:

```rust
use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{ElementName, Node, TemplateDocument};
use crate::error::{ValidationError, ValidationErrorKind};
use crate::name::{ComponentName, NameError, NativeElementName};

#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedTemplate {
    nodes: Vec<ValidatedNode>,
}

impl ValidatedTemplate {
    pub fn nodes(&self) -> &[ValidatedNode] {
        &self.nodes
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValidatedNode {
    NativeElement(ValidatedElement<NativeElementName>),
    ComponentElement(ValidatedElement<ComponentName>),
    Text(ValidatedText),
    Interpolation(ValidatedInterpolation),
    If(ValidatedIf),
    ForEach(ValidatedForEach),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedText {
    value: String,
}

impl ValidatedText {
    pub fn value(&self) -> &str {
        &self.value
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedInterpolation {
    expr: crate::expr::Expr,
}

impl ValidatedInterpolation {
    pub fn expr(&self) -> &crate::expr::Expr {
        &self.expr
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedElement<Name> {
    name: Name,
    attributes: Vec<ValidatedAttribute>,
    children: Vec<ValidatedNode>,
}

impl<Name> ValidatedElement<Name> {
    pub fn name(&self) -> &Name {
        &self.name
    }

    pub fn attributes(&self) -> &[ValidatedAttribute] {
        &self.attributes
    }

    pub fn children(&self) -> &[ValidatedNode] {
        &self.children
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedAttribute {
    name: crate::name::AttributeName,
    value: ValidatedAttrValue,
}

impl ValidatedAttribute {
    pub fn name(&self) -> &crate::name::AttributeName {
        &self.name
    }

    pub fn value(&self) -> &ValidatedAttrValue {
        &self.value
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValidatedAttrValue {
    Bool,
    Static(String),
    Expression(crate::expr::Expr),
    Interpolated(Vec<ValidatedAttrPart>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValidatedAttrPart {
    Text(String),
    Expr(crate::expr::Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedIf {
    branches: Vec<ValidatedIfBranch>,
    else_children: Vec<ValidatedNode>,
}

impl ValidatedIf {
    pub fn branches(&self) -> &[ValidatedIfBranch] {
        &self.branches
    }

    pub fn else_children(&self) -> &[ValidatedNode] {
        &self.else_children
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedIfBranch {
    condition: crate::expr::Expr,
    children: Vec<ValidatedNode>,
}

impl ValidatedIfBranch {
    pub fn condition(&self) -> &crate::expr::Expr {
        &self.condition
    }

    pub fn children(&self) -> &[ValidatedNode] {
        &self.children
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedForEach {
    collection: crate::expr::Expr,
    item_name: crate::name::VariableName,
    children: Vec<ValidatedNode>,
    else_children: Vec<ValidatedNode>,
}

impl ValidatedForEach {
    pub fn collection(&self) -> &crate::expr::Expr {
        &self.collection
    }

    pub fn item_name(&self) -> &crate::name::VariableName {
        &self.item_name
    }

    pub fn children(&self) -> &[ValidatedNode] {
        &self.children
    }

    pub fn else_children(&self) -> &[ValidatedNode] {
        &self.else_children
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NativeElementRegistry {
    specs: BTreeMap<NativeElementName, BTreeMap<crate::name::AttributeName, AttributeRule>>,
}

impl NativeElementRegistry {
    pub fn try_from_specs(specs: Vec<NativeElementSpec>) -> Result<Self, RegistryError> {
        let mut checked = BTreeMap::new();
        for spec in specs {
            if checked.insert(spec.name.clone(), spec.attributes).is_some() {
                return Err(RegistryError::DuplicateNativeElement {
                    name: spec.name.as_str().to_owned(),
                });
            }
        }
        Ok(Self { specs: checked })
    }

    fn contains(&self, name: &NativeElementName) -> bool {
        self.specs.contains_key(name)
    }

    fn attribute_rule(&self, name: &NativeElementName, attribute: &str) -> Option<&AttributeRule> {
        self.specs
            .get(name)
            .and_then(|attrs| attrs.iter().find(|(attr, _)| attr.as_str() == attribute))
            .map(|(_, rule)| rule)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeElementSpec {
    name: NativeElementName,
    attributes: BTreeMap<crate::name::AttributeName, AttributeRule>,
}

impl NativeElementSpec {
    pub fn try_new(name: impl Into<String>, attributes: Vec<AttributeSpec>) -> Result<Self, RegistryError> {
        let mut checked_attrs = BTreeMap::new();
        for attribute in attributes {
            if checked_attrs.insert(attribute.name.clone(), attribute.rule).is_some() {
                return Err(RegistryError::DuplicateAttributeSpec {
                    name: attribute.name.as_str().to_owned(),
                });
            }
        }
        Ok(Self {
            name: NativeElementName::try_new(name)?,
            attributes: checked_attrs,
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ComponentRegistry {
    specs: BTreeMap<ComponentName, BTreeMap<crate::name::AttributeName, AttributeRule>>,
}

impl ComponentRegistry {
    pub fn try_from_specs(specs: Vec<ComponentSpec>) -> Result<Self, RegistryError> {
        let mut checked = BTreeMap::new();
        for spec in specs {
            if checked.insert(spec.name.clone(), spec.attributes).is_some() {
                return Err(RegistryError::DuplicateComponent {
                    name: spec.name.as_str().to_owned(),
                });
            }
        }
        Ok(Self { specs: checked })
    }

    fn contains(&self, name: &ComponentName) -> bool {
        self.specs.contains_key(name)
    }

    fn attribute_rule(&self, name: &ComponentName, attribute: &str) -> Option<&AttributeRule> {
        self.specs
            .get(name)
            .and_then(|attrs| attrs.iter().find(|(attr, _)| attr.as_str() == attribute))
            .map(|(_, rule)| rule)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentSpec {
    name: ComponentName,
    attributes: BTreeMap<crate::name::AttributeName, AttributeRule>,
}

impl ComponentSpec {
    pub fn try_new(name: impl Into<String>, attributes: Vec<AttributeSpec>) -> Result<Self, RegistryError> {
        let mut checked_attrs = BTreeMap::new();
        for attribute in attributes {
            if checked_attrs.insert(attribute.name.clone(), attribute.rule).is_some() {
                return Err(RegistryError::DuplicateAttributeSpec {
                    name: attribute.name.as_str().to_owned(),
                });
            }
        }
        Ok(Self {
            name: ComponentName::try_new(name)?,
            attributes: checked_attrs,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AttributeKind {
    Bool,
    Static,
    Expression,
    Interpolated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributeRule {
    accepted: BTreeSet<AttributeKind>,
}

impl AttributeRule {
    pub fn one(kind: AttributeKind) -> Self {
        Self {
            accepted: [kind].into_iter().collect(),
        }
    }

    pub fn any(first: AttributeKind, rest: impl IntoIterator<Item = AttributeKind>) -> Self {
        let mut accepted = BTreeSet::from([first]);
        accepted.extend(rest);
        Self {
            accepted,
        }
    }

    pub fn accepts(&self, kind: AttributeKind) -> bool {
        self.accepted.contains(&kind)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributeSpec {
    name: crate::name::AttributeName,
    rule: AttributeRule,
}

impl AttributeSpec {
    pub fn try_new(name: impl Into<String>, rule: AttributeRule) -> Result<Self, NameError> {
        Ok(Self {
            name: crate::name::AttributeName::try_new(name)?,
            rule,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    Name(NameError),
    DuplicateNativeElement { name: String },
    DuplicateComponent { name: String },
    DuplicateAttributeSpec { name: String },
}

impl From<NameError> for RegistryError {
    fn from(error: NameError) -> Self {
        Self::Name(error)
    }
}

pub fn validate_template(
    document: &TemplateDocument,
    native: &NativeElementRegistry,
    components: &ComponentRegistry,
) -> Result<ValidatedTemplate, ValidationError> {
    let mut nodes = Vec::new();
    for node in document.nodes() {
        nodes.push(validate_node(node, native, components)?);
    }
    Ok(ValidatedTemplate { nodes })
}

fn validate_node(
    node: &Node,
    native: &NativeElementRegistry,
    components: &ComponentRegistry,
) -> Result<ValidatedNode, ValidationError> {
    match node {
        Node::Element(element) => {
            let mut seen = BTreeSet::new();
            for attr in element.attributes() {
                if !seen.insert(attr.name().to_owned()) {
                    return Err(ValidationError::new(
                        ValidationErrorKind::DuplicateAttribute {
                            name: attr.name().to_owned(),
                        },
                        attr.span(),
                    ));
                }
            }
            match element.name() {
                ElementName::Native(name) => {
                    if !native.contains(name) {
                        return Err(ValidationError::new(
                            ValidationErrorKind::UnknownNativeElement {
                                name: name.as_str().to_owned(),
                            },
                            element.span(),
                        ));
                    }
                    for attr in element.attributes() {
                        let Some(rule) = native.attribute_rule(name, attr.name()) else {
                            return Err(invalid_attribute(name.as_str(), attr));
                        };
                        validate_attribute_value(name.as_str(), attr, rule)?;
                    }
                    let children = validate_children(element.children(), native, components)?;
                    Ok(ValidatedNode::NativeElement(ValidatedElement {
                        name: name.clone(),
                        attributes: validate_attributes(element.attributes()),
                        children,
                    }))
                }
                ElementName::Component(name) => {
                    if !components.contains(name) {
                        return Err(ValidationError::new(
                            ValidationErrorKind::UnknownComponent {
                                name: name.as_str().to_owned(),
                            },
                            element.span(),
                        ));
                    }
                    for attr in element.attributes() {
                        let Some(rule) = components.attribute_rule(name, attr.name()) else {
                            return Err(invalid_attribute(name.as_str(), attr));
                        };
                        validate_attribute_value(name.as_str(), attr, rule)?;
                    }
                    let children = validate_children(element.children(), native, components)?;
                    Ok(ValidatedNode::ComponentElement(ValidatedElement {
                        name: name.clone(),
                        attributes: validate_attributes(element.attributes()),
                        children,
                    }))
                }
            }
        }
        Node::If(if_node) => {
            let mut branches = Vec::new();
            for branch in if_node.branches() {
                let mut children = Vec::new();
                for child in branch.children() {
                    children.push(validate_node(child, native, components)?);
                }
                branches.push(ValidatedIfBranch {
                    condition: branch.condition().clone(),
                    children,
                });
            }
            let mut else_children = Vec::new();
            for child in if_node.else_children() {
                else_children.push(validate_node(child, native, components)?);
            }
            Ok(ValidatedNode::If(ValidatedIf { branches, else_children }))
        }
        Node::ForEach(for_each) => {
            let mut children = Vec::new();
            for child in for_each.children() {
                children.push(validate_node(child, native, components)?);
            }
            let mut else_children = Vec::new();
            for child in for_each.else_children() {
                else_children.push(validate_node(child, native, components)?);
            }
            Ok(ValidatedNode::ForEach(ValidatedForEach {
                collection: for_each.collection().clone(),
                item_name: for_each.item_name().clone(),
                children,
                else_children,
            }))
        }
        Node::Text(text) => Ok(ValidatedNode::Text(ValidatedText {
            value: text.value().to_owned(),
        })),
        Node::Interpolation(interpolation) => Ok(ValidatedNode::Interpolation(ValidatedInterpolation {
            expr: interpolation.expr().clone(),
        })),
    }
}

fn validate_children(
    children: &[Node],
    native: &NativeElementRegistry,
    components: &ComponentRegistry,
) -> Result<Vec<ValidatedNode>, ValidationError> {
    let mut validated = Vec::new();
    for child in children {
        validated.push(validate_node(child, native, components)?);
    }
    Ok(validated)
}

fn validate_attributes(attributes: &[crate::ast::Attribute]) -> Vec<ValidatedAttribute> {
    attributes
        .iter()
        .map(|attribute| ValidatedAttribute {
            name: crate::name::AttributeName::try_new(attribute.name())
                .expect("authored attributes are already name-checked"),
            value: match attribute.value() {
                crate::ast::AttrValue::Bool => ValidatedAttrValue::Bool,
                crate::ast::AttrValue::Static(value) => ValidatedAttrValue::Static(value.clone()),
                crate::ast::AttrValue::Expression(expr) => ValidatedAttrValue::Expression(expr.clone()),
                crate::ast::AttrValue::Interpolated(parts) => ValidatedAttrValue::Interpolated(
                    parts
                        .iter()
                        .map(|part| match part {
                            crate::ast::AttrPart::Text(text) => ValidatedAttrPart::Text(text.clone()),
                            crate::ast::AttrPart::Expr(expr) => ValidatedAttrPart::Expr(expr.clone()),
                        })
                        .collect(),
                ),
            },
        })
        .collect()
}

fn invalid_attribute(element: &str, attr: &crate::ast::Attribute) -> ValidationError {
    ValidationError::new(
        ValidationErrorKind::InvalidAttribute {
            element: element.to_owned(),
            attribute: attr.name().to_owned(),
        },
        attr.span(),
    )
}

fn validate_attribute_value(
    element: &str,
    attr: &crate::ast::Attribute,
    rule: &AttributeRule,
) -> Result<(), ValidationError> {
    let kind = authored_attr_kind(attr.value());
    if rule.accepts(kind) {
        Ok(())
    } else {
        Err(ValidationError::new(
            ValidationErrorKind::InvalidAttributeValue {
                element: element.to_owned(),
                attribute: attr.name().to_owned(),
            },
            attr.span(),
        ))
    }
}

fn authored_attr_kind(value: &crate::ast::AttrValue) -> AttributeKind {
    match value {
        crate::ast::AttrValue::Bool => AttributeKind::Bool,
        crate::ast::AttrValue::Static(_) => AttributeKind::Static,
        crate::ast::AttrValue::Expression(_) => AttributeKind::Expression,
        crate::ast::AttrValue::Interpolated(_) => AttributeKind::Interpolated,
    }
}
```

- [ ] **Step 4: Re-export validation front door**

Modify `src/lib.rs` to add:

```rust
pub use validate::{
    validate_template, AttributeKind, AttributeRule, AttributeSpec, ComponentRegistry, ComponentSpec,
    NativeElementRegistry, NativeElementSpec, RegistryError, ValidatedAttrValue,
    ValidatedAttribute, ValidatedElement, ValidatedForEach, ValidatedIf, ValidatedIfBranch,
    ValidatedNode, ValidatedTemplate,
};
```

- [ ] **Step 5: Run tests to verify they pass**

Run:

```sh
cargo test -p surgeist-template --test template_v1
```

Expected: PASS.

- [ ] **Step 6: Run task checks**

Run:

```sh
cargo fmt --check
cargo test -p surgeist-template
```

Expected: PASS.

- [ ] **Step 7: Review and commit**

Coordinator assigns a separate reviewer for Task 4. Reviewer checks that validation is a narrow boundary after authored parsing and before rendering.

If reviewer is clean, run:

```sh
git add src/lib.rs src/validate.rs tests/template_v1.rs
git commit -m "Add template semantic validation"
```

## Task 5: Initial Rust Rendering Contract

This task creates a deterministic structured Rust-code snapshot string for downstream integration planning. It does not require the root `surgeist::template` facade to exist or compile in this crate yet.

**Files:**
- Modify: `src/render.rs`
- Modify: `src/lib.rs`
- Test: `tests/template_v1.rs`

- [ ] **Step 1: Write failing render test**

Append to `tests/template_v1.rs`:

```rust
use surgeist_template::render_to_rust;

#[test]
fn renders_validated_template_to_rust_calls() {
    let document = parse_template(r#"<Panel title={$project.name}><Text>Hello {$user.name}</Text></Panel>"#)
        .expect("template parses");
    let native = NativeElementRegistry::try_from_specs(vec![
        surgeist_template::NativeElementSpec::try_new("div", Vec::new()).expect("valid spec"),
    ])
    .expect("valid native registry");
    let components = ComponentRegistry::try_from_specs(vec![
        surgeist_template::ComponentSpec::try_new(
            "Panel",
            vec![surgeist_template::AttributeSpec::try_new(
                "title",
                surgeist_template::AttributeRule::one(surgeist_template::AttributeKind::Expression),
            )
            .expect("valid attr spec")],
        )
        .expect("valid spec"),
        surgeist_template::ComponentSpec::try_new("Text", Vec::new()).expect("valid spec"),
    ])
    .expect("valid component registry");
    let validated = validate_template(&document, &native, &components).expect("template validates");

    let rust = render_to_rust(&validated);

    assert_eq!(
        rust,
        "surgeist::template::fragment(vec![surgeist::template::component(\"Panel\", vec![surgeist::template::attr(\"title\",surgeist::template::AttrValue::Expr(\"project.name\")),], vec![surgeist::template::component(\"Text\", vec![], vec![surgeist::template::text(\"Hello \"),surgeist::template::expr(\"user.name\"),]),]),])"
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```sh
cargo test -p surgeist-template --test template_v1 renders_validated_template_to_rust_calls
```

Expected: FAIL because `render_to_rust` is not defined.

- [ ] **Step 3: Implement render contract**

Replace `src/render.rs` with:

```rust
use crate::expr::{BinaryOp, Expr, Literal, PathSegment, UnaryOp};
use crate::validate::{ValidatedAttrPart, ValidatedAttrValue, ValidatedNode, ValidatedTemplate};

pub fn render_to_rust(template: &ValidatedTemplate) -> String {
    let mut output = String::from("surgeist::template::fragment(vec![");
    for node in template.nodes() {
        render_node(node, &mut output);
        output.push(',');
    }
    output.push_str("])");
    output
}

fn render_node(node: &ValidatedNode, output: &mut String) {
    match node {
        ValidatedNode::NativeElement(element) => {
            output.push_str("surgeist::template::native(");
            push_quoted(element.name().as_str(), output);
            output.push_str(", vec![");
            for attr in element.attributes() {
                output.push_str("surgeist::template::attr(");
                push_quoted(attr.name().as_str(), output);
                output.push(',');
                render_attr_value(attr.value(), output);
                output.push_str("),");
            }
            output.push_str("], vec![");
            for child in element.children() {
                render_node(child, output);
                output.push(',');
            }
            output.push_str("])");
        }
        ValidatedNode::ComponentElement(element) => {
            output.push_str("surgeist::template::component(");
            push_quoted(element.name().as_str(), output);
            output.push_str(", vec![");
            for attr in element.attributes() {
                output.push_str("surgeist::template::attr(");
                push_quoted(attr.name().as_str(), output);
                output.push(',');
                render_attr_value(attr.value(), output);
                output.push_str("),");
            }
            output.push_str("], vec![");
            for child in element.children() {
                render_node(child, output);
                output.push(',');
            }
            output.push_str("])");
        }
        ValidatedNode::Text(text) => {
            output.push_str("surgeist::template::text(");
            push_quoted(text.value(), output);
            output.push(')');
        }
        ValidatedNode::Interpolation(interpolation) => {
            output.push_str("surgeist::template::expr(");
            push_quoted(&expr_to_string(interpolation.expr()), output);
            output.push(')');
        }
        ValidatedNode::If(if_node) => {
            output.push_str("surgeist::template::if_block(vec![");
            for branch in if_node.branches() {
                output.push('(');
                push_quoted(&expr_to_string(branch.condition()), output);
                output.push_str(", vec![");
                for child in branch.children() {
                    render_node(child, output);
                    output.push(',');
                }
                output.push_str("]),");
            }
            output.push_str("], vec![");
            for child in if_node.else_children() {
                render_node(child, output);
                output.push(',');
            }
            output.push_str("])");
        }
        ValidatedNode::ForEach(for_each) => {
            output.push_str("surgeist::template::foreach_block(");
            push_quoted(&expr_to_string(for_each.collection()), output);
            output.push(',');
            push_quoted(for_each.item_name().as_str(), output);
            output.push_str(", vec![");
            for child in for_each.children() {
                render_node(child, output);
                output.push(',');
            }
            output.push_str("], vec![");
            for child in for_each.else_children() {
                render_node(child, output);
                output.push(',');
            }
            output.push_str("])");
        }
    }
}

fn render_attr_value(value: &ValidatedAttrValue, output: &mut String) {
    match value {
        ValidatedAttrValue::Bool => output.push_str("surgeist::template::AttrValue::Bool(true)"),
        ValidatedAttrValue::Static(value) => {
            output.push_str("surgeist::template::AttrValue::Static(");
            push_quoted(value, output);
            output.push(')');
        }
        ValidatedAttrValue::Expression(expr) => {
            output.push_str("surgeist::template::AttrValue::Expr(");
            push_quoted(&expr_to_string(expr), output);
            output.push(')');
        }
        ValidatedAttrValue::Interpolated(parts) => {
            output.push_str("surgeist::template::AttrValue::Interpolated(vec![");
            for part in parts {
                match part {
                    ValidatedAttrPart::Text(text) => {
                        output.push_str("surgeist::template::AttrPart::Text(");
                        push_quoted(text, output);
                        output.push_str("),");
                    }
                    ValidatedAttrPart::Expr(expr) => {
                        output.push_str("surgeist::template::AttrPart::Expr(");
                        push_quoted(&expr_to_string(expr), output);
                        output.push_str("),");
                    }
                }
            }
            output.push_str("])");
        }
    }
}

fn expr_to_string(expr: &Expr) -> String {
    match expr {
        Expr::Literal(Literal::String(value)) => format!("\"{value}\""),
        Expr::Literal(Literal::Int(value)) => value.to_string(),
        Expr::Literal(Literal::Float(value)) => value.to_string(),
        Expr::Literal(Literal::Bool(value)) => value.to_string(),
        Expr::Literal(Literal::Null) => "null".to_owned(),
        Expr::Variable(path) => {
            let mut value = path.root().to_owned();
            for segment in path.segments() {
                match segment {
                    PathSegment::Field(field) => {
                        value.push('.');
                        value.push_str(field.as_str());
                    }
                    PathSegment::Index(index) => {
                        value.push('[');
                        value.push_str(&index.get().to_string());
                        value.push(']');
                    }
                }
            }
            value
        }
        Expr::Unary { op, expr } => format!("{}{}", unary_op_to_str(*op), expr_to_string(expr)),
        Expr::Binary { op, left, right } => format!(
            "({} {} {})",
            expr_to_string(left),
            binary_op_to_str(*op),
            expr_to_string(right)
        ),
    }
}

fn unary_op_to_str(op: UnaryOp) -> &'static str {
    match op {
        UnaryOp::Not => "!",
        UnaryOp::Neg => "-",
    }
}

fn binary_op_to_str(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Or => "||",
        BinaryOp::And => "&&",
        BinaryOp::Eq => "==",
        BinaryOp::Ne => "!=",
        BinaryOp::Gt => ">",
        BinaryOp::Ge => ">=",
        BinaryOp::Lt => "<",
        BinaryOp::Le => "<=",
        BinaryOp::Add => "+",
        BinaryOp::Sub => "-",
        BinaryOp::Mul => "*",
        BinaryOp::Div => "/",
        BinaryOp::Rem => "%",
    }
}

fn push_quoted(value: &str, output: &mut String) {
    output.push('"');
    for ch in value.chars() {
        match ch {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            ch => output.push(ch),
        }
    }
    output.push('"');
}
```

- [ ] **Step 4: Re-export render front door**

Modify `src/lib.rs` to add:

```rust
pub use render::render_to_rust;
```

- [ ] **Step 5: Run test to verify it passes**

Run:

```sh
cargo test -p surgeist-template --test template_v1 renders_validated_template_to_rust_calls
```

Expected: PASS.

- [ ] **Step 6: Run task checks**

Run:

```sh
cargo fmt --check
cargo test -p surgeist-template
```

Expected: PASS.

- [ ] **Step 7: Review and commit**

Coordinator assigns a separate reviewer for Task 5. Reviewer checks that rendering consumes validated IR rather than raw source and does not leak sibling-crate private APIs.

If reviewer is clean, run:

```sh
git add src/lib.rs src/render.rs tests/template_v1.rs
git commit -m "Add initial template Rust renderer"
```

## Task 6: Broaden Expression Operators Without Expanding Runtime Power

**Files:**
- Modify: `src/expr.rs`
- Test: `tests/template_v1.rs`

- [ ] **Step 1: Write failing expression tests**

Append to `tests/template_v1.rs`:

```rust
use surgeist_template::{BinaryOp, Expr};

#[test]
fn parses_basic_boolean_condition() {
    let document = parse_template("{if $visible && $enabled}<Text>Shown</Text>{/if}")
        .expect("template parses");

    let Node::If(if_node) = &document.nodes()[0] else {
        panic!("expected if node");
    };
    assert!(matches!(
        if_node.branches()[0].condition(),
        Expr::Binary {
            op: BinaryOp::And,
            ..
        }
    ));
}

#[test]
fn parses_comparison_condition() {
    let document = parse_template("{if $count > 0}<Text>Rows</Text>{/if}")
        .expect("template parses");

    let Node::If(if_node) = &document.nodes()[0] else {
        panic!("expected if node");
    };
    assert!(matches!(
        if_node.branches()[0].condition(),
        Expr::Binary {
            op: BinaryOp::Gt,
            ..
        }
    ));
}

#[test]
fn parses_indexed_variable_path() {
    let document = parse_template("<Text>{$items[0].name}</Text>").expect("template parses");

    let Node::Element(element) = &document.nodes()[0] else {
        panic!("expected element");
    };
    let Node::Interpolation(interpolation) = &element.children()[0] else {
        panic!("expected interpolation");
    };
    let Expr::Variable(path) = interpolation.expr() else {
        panic!("expected variable path");
    };
    assert_eq!(path.root(), "items");
    assert_eq!(
        path.segments()[0],
        surgeist_template::PathSegment::Index(surgeist_template::PathIndex::new(0))
    );
}

#[test]
fn rejects_malformed_indexed_variable_paths() {
    for source in [
        "<Text>{$items[]}</Text>",
        "<Text>{$items[-1]}</Text>",
        "<Text>{$items[abc]}</Text>",
        "<Text>{$items[0.name}</Text>",
        "<Text>{$items[0][1]}</Text>",
        "<Text>{$.name}</Text>",
        "<Text>{$items.}</Text>",
        "<Text>{$items..name}</Text>",
        "<Text>{$items.[0]}</Text>",
    ] {
        let error = parse_template(source).expect_err("malformed index path fails");
        assert!(format!("{error:?}").contains("InvalidExpression"));
    }
}

#[test]
fn rejects_unsupported_expression_power() {
    for source in [
        "<Text>{$format($name)}</Text>",
        "<Text>{$user->name}</Text>",
        "<Text>{$user.name = \"Ada\"}</Text>",
        "<Text>{$[$a, $b]}</Text>",
        "<Text>{$enabled ? \"yes\" : \"no\"}</Text>",
        "<Text>{$value ?? \"fallback\"}</Text>",
    ] {
        let error = parse_template(source).expect_err("unsupported expression fails");
        assert!(format!("{error:?}").contains("InvalidExpression"));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```sh
cargo test -p surgeist-template --test template_v1
```

Expected: FAIL because `parse_simple_expr` does not yet support operators and the Pratt parser has not locked in indexed paths.

- [ ] **Step 3: Replace expression helper with a small Pratt parser**

In `src/expr.rs`, keep the public AST from Task 2 and replace `parse_simple_expr` with a tokenizer plus precedence parser that supports only:

```text
primary: variable path including numeric bracket indexes, scalar literal, parenthesized expression
unary: !, -
binary: ||, &&, ==, !=, >, >=, <, <=, +, -, *, /, %
```

Use this exact public function signature:

```rust
pub fn parse_simple_expr(source: &str) -> Result<Expr, &'static str>
```

Use this operator precedence from lowest to highest:

```rust
fn infix_binding_power(op: BinaryOp) -> (u8, u8) {
    match op {
        BinaryOp::Or => (1, 2),
        BinaryOp::And => (3, 4),
        BinaryOp::Eq | BinaryOp::Ne | BinaryOp::Gt | BinaryOp::Ge | BinaryOp::Lt | BinaryOp::Le => (5, 6),
        BinaryOp::Add | BinaryOp::Sub => (7, 8),
        BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem => (9, 10),
    }
}
```

Reject function calls, object access, assignment, ternary, null coalescing, arrays, and request/global escape hatches with `Err("unsupported expression")`.

- [ ] **Step 4: Run tests to verify they pass**

Run:

```sh
cargo test -p surgeist-template --test template_v1
```

Expected: PASS.

- [ ] **Step 5: Run task checks**

Run:

```sh
cargo fmt --check
cargo test -p surgeist-template
```

Expected: PASS.

- [ ] **Step 6: Review and commit**

Coordinator assigns a separate reviewer for Task 6. Reviewer checks that expression support remains presentation-focused and does not turn templates into general application code.

If reviewer is clean, run:

```sh
git add src/expr.rs tests/template_v1.rs
git commit -m "Add focused template expression parser"
```

## Task 7: Documentation And Final Verification

**Files:**
- Modify: `README.md`
- Test: full crate checks

- [ ] **Step 1: Add README usage and scope documentation**

Append this section to `README.md`:

```markdown
## Template V1 Scope

`surgeist-template` parses strict HTML-like UI templates into typed authored
Template IR. It supports brace-delimited interpolation and control blocks for view
composition, while keeping Rust responsible for model, controller, state, and
complex widget behavior.

Supported v1 syntax:

- HTML-like elements with strict matching close tags
- UpperCamelCase component/widget elements
- text nodes and `{$expr}` interpolation
- quoted attribute interpolation and boolean attributes
- `{if}`, `{elseif}`, `{else}`, `{/if}`
- `{foreach $items as $item}`, `{foreachelse}`, `{/foreach}`

Excluded v1 syntax:

- browser-style HTML recovery
- template-side mutation, file inclusion, inheritance, dynamic evaluation,
  debugger directives, and loops other than `foreach`
- runtime escape hatches such as functions, object methods, static access,
  namespaces, and request globals

Complex widgets should be implemented in Rust and composed from templates as
registered component elements.
```

- [ ] **Step 2: Run documentation task checks**

Run:

```sh
cargo test -p surgeist-template
cargo fmt --check
```

Expected: PASS.

- [ ] **Step 3: Review and commit documentation**

Coordinator assigns a separate reviewer for the README-only documentation change. Reviewer checks that the documented v1 scope matches the implemented parser, validator, renderer, and explicit exclusions.

If reviewer is clean, run:

```sh
git add README.md
git commit -m "Document template v1 scope"
```

- [ ] **Step 4: Final holistic clean-context review**

Coordinator assigns a final holistic reviewer with only:

- this plan
- `README.md`
- `Cargo.toml`
- `src/`
- `tests/`
- `guidance/surgeist-rust-modeling-guide.md`
- `git diff --stat`
- `git diff`

Reviewer must answer:

```text
Is the complete result clean against guidance/surgeist-rust-modeling-guide.md?
Check typed phase boundaries, invalid-state prevention, public API front doors,
strict parser behavior, component registry boundaries, and crate ownership.
Return findings ordered by severity. If clean, say "clean".
```

- [ ] **Step 5: Reconcile review**

If the final reviewer reports findings, create follow-up scoped worker/reviewer cycles and commit each logical fix before re-running the final holistic review.

If the final reviewer says `clean`, proceed.

- [ ] **Step 6: Run final focused checks on committed result**

Run:

```sh
cargo test -p surgeist-template
cargo clippy -p surgeist-template --all-targets -- -D warnings
cargo fmt --check
git status --short --branch
```

Expected: PASS and no uncommitted implementation or documentation changes.

## Final Completion Criteria

The implementation is complete only when all are true:

- Each task-scoped worker/reviewer cycle is clean.
- `cargo test -p surgeist-template` passes.
- `cargo clippy -p surgeist-template --all-targets -- -D warnings` passes.
- `cargo fmt --check` passes.
- Final holistic clean-context review returns `clean` against `guidance/surgeist-rust-modeling-guide.md`.
- `git status --short --branch` shows no uncommitted implementation changes.

## Coordinator Notes

- Keep work inside `/Users/codex/Development/surgeist-template`.
- Do not edit sibling crate repos.
- Do not update root `surgeist` submodule pointers.
- Use current `main`; do not create a branch or worktree unless explicitly directed.
- Use `apply_patch` for manual edits.
- Push only if root integration or the user requires a fetchable commit.
