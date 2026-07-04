use crate::expr::Expr;
use crate::name::{AttributeName, ComponentName, NameError, NativeElementName, VariableName};
use crate::span::SourceSpan;

#[derive(Debug, Clone, PartialEq)]
pub struct TemplateDocument {
    nodes: Vec<Node>,
}

impl TemplateDocument {
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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

    #[allow(dead_code)]
    pub(crate) fn text(value: impl Into<String>, span: SourceSpan) -> Self {
        Self::Text(TextNode {
            value: value.into(),
            span,
        })
    }

    #[allow(dead_code)]
    pub(crate) fn interpolation(expr: Expr, span: SourceSpan) -> Self {
        Self::Interpolation(InterpolationNode { expr, span })
    }

    #[allow(dead_code)]
    pub(crate) fn if_block(
        branches: Vec<IfBranch>,
        else_children: Vec<Node>,
        span: SourceSpan,
    ) -> Self {
        Self::If(IfNode {
            branches,
            else_children,
            span,
        })
    }

    #[allow(dead_code)]
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

    pub const fn span(&self) -> SourceSpan {
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

    pub const fn span(&self) -> SourceSpan {
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

    pub const fn span(&self) -> SourceSpan {
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

    pub const fn span(&self) -> SourceSpan {
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

    pub const fn span(&self) -> SourceSpan {
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
    pub fn try_new(
        name: impl Into<String>,
        value: AttrValue,
        span: SourceSpan,
    ) -> Result<Self, NameError> {
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

    pub const fn span(&self) -> SourceSpan {
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
        Self {
            condition,
            children,
        }
    }

    pub fn condition(&self) -> &Expr {
        &self.condition
    }

    pub fn children(&self) -> &[Node] {
        &self.children
    }
}

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
            vec![
                Attribute::try_new(
                    "title",
                    AttrValue::Expression(Expr::Literal(Literal::String("Hello".to_owned()))),
                    span,
                )
                .expect("valid attribute"),
            ],
            vec![Node::text("Body", span)],
            span,
        );
        let document = TemplateDocument::from_nodes(vec![node]);

        assert_eq!(document.nodes().len(), 1);
        let Node::Element(element) = &document.nodes()[0] else {
            panic!("expected element node");
        };
        assert_eq!(element.name().as_str(), "Panel");
        assert_eq!(element.attributes()[0].name(), "title");
        assert_eq!(element.children().len(), 1);
    }

    #[test]
    fn element_names_validate_native_and_component_shapes() {
        assert!(ElementName::native("section").is_ok());
        assert!(ElementName::native("Panel").is_err());
        assert!(ElementName::component("Panel").is_ok());
        assert!(ElementName::component("section").is_err());
    }
}
