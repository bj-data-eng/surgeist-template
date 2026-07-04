use std::collections::{HashMap, HashSet};

use crate::ast::{AttrPart, AttrValue, Attribute, ElementName, Node, TemplateDocument};
use crate::error::{ValidationError, ValidationErrorKind};
use crate::expr::Expr;
use crate::name::{AttributeName, ComponentName, NameError, NativeElementName, VariableName};
use crate::span::SourceSpan;

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
pub struct ValidatedElement<Name> {
    name: Name,
    attributes: Vec<ValidatedAttribute>,
    children: Vec<ValidatedNode>,
    span: SourceSpan,
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

    pub const fn span(&self) -> SourceSpan {
        self.span
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedText {
    value: String,
    span: SourceSpan,
}

impl ValidatedText {
    pub fn value(&self) -> &str {
        &self.value
    }

    pub const fn span(&self) -> SourceSpan {
        self.span
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedInterpolation {
    expr: Expr,
    span: SourceSpan,
}

impl ValidatedInterpolation {
    pub fn expr(&self) -> &Expr {
        &self.expr
    }

    pub const fn span(&self) -> SourceSpan {
        self.span
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedAttribute {
    name: AttributeName,
    value: ValidatedAttrValue,
    span: SourceSpan,
}

impl ValidatedAttribute {
    pub fn name(&self) -> &AttributeName {
        &self.name
    }

    pub fn value(&self) -> &ValidatedAttrValue {
        &self.value
    }

    pub const fn span(&self) -> SourceSpan {
        self.span
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValidatedAttrValue {
    Bool,
    Static(String),
    Expression(Expr),
    Interpolated(Vec<ValidatedAttrPart>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValidatedAttrPart {
    Text(String),
    Expr(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedIf {
    branches: Vec<ValidatedIfBranch>,
    else_children: Vec<ValidatedNode>,
    span: SourceSpan,
}

impl ValidatedIf {
    pub fn branches(&self) -> &[ValidatedIfBranch] {
        &self.branches
    }

    pub fn else_children(&self) -> &[ValidatedNode] {
        &self.else_children
    }

    pub const fn span(&self) -> SourceSpan {
        self.span
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedIfBranch {
    condition: Expr,
    children: Vec<ValidatedNode>,
}

impl ValidatedIfBranch {
    pub fn condition(&self) -> &Expr {
        &self.condition
    }

    pub fn children(&self) -> &[ValidatedNode] {
        &self.children
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedForEach {
    collection: Expr,
    item_name: VariableName,
    children: Vec<ValidatedNode>,
    else_children: Vec<ValidatedNode>,
    span: SourceSpan,
}

impl ValidatedForEach {
    pub fn collection(&self) -> &Expr {
        &self.collection
    }

    pub fn item_name(&self) -> &VariableName {
        &self.item_name
    }

    pub fn children(&self) -> &[ValidatedNode] {
        &self.children
    }

    pub fn else_children(&self) -> &[ValidatedNode] {
        &self.else_children
    }

    pub const fn span(&self) -> SourceSpan {
        self.span
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NativeElementRegistry {
    specs: HashMap<NativeElementName, NativeElementSpec>,
}

impl NativeElementRegistry {
    pub fn try_from_specs(specs: Vec<NativeElementSpec>) -> Result<Self, RegistryError> {
        let mut checked = HashMap::new();
        for spec in specs {
            let name = spec.name.clone();
            if checked.insert(name.clone(), spec).is_some() {
                return Err(RegistryError::DuplicateNativeElement {
                    name: name.as_str().to_owned(),
                });
            }
        }
        Ok(Self { specs: checked })
    }

    fn spec(&self, name: &NativeElementName) -> Option<&NativeElementSpec> {
        self.specs.get(name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeElementSpec {
    name: NativeElementName,
    attributes: HashMap<AttributeName, AttributeSpec>,
}

impl NativeElementSpec {
    pub fn try_new(
        name: impl Into<String>,
        attributes: Vec<AttributeSpec>,
    ) -> Result<Self, RegistryError> {
        Ok(Self {
            name: NativeElementName::try_new(name)?,
            attributes: check_attribute_specs(attributes)?,
        })
    }

    pub fn name(&self) -> &NativeElementName {
        &self.name
    }

    fn attribute_rule(&self, name: &str) -> Option<&AttributeRule> {
        self.attributes
            .values()
            .find(|attribute| attribute.name.as_str() == name)
            .map(|attribute| &attribute.rule)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ComponentRegistry {
    specs: HashMap<ComponentName, ComponentSpec>,
}

impl ComponentRegistry {
    pub fn try_from_specs(specs: Vec<ComponentSpec>) -> Result<Self, RegistryError> {
        let mut checked = HashMap::new();
        for spec in specs {
            let name = spec.name.clone();
            if checked.insert(name.clone(), spec).is_some() {
                return Err(RegistryError::DuplicateComponent {
                    name: name.as_str().to_owned(),
                });
            }
        }
        Ok(Self { specs: checked })
    }

    fn spec(&self, name: &ComponentName) -> Option<&ComponentSpec> {
        self.specs.get(name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentSpec {
    name: ComponentName,
    attributes: HashMap<AttributeName, AttributeSpec>,
}

impl ComponentSpec {
    pub fn try_new(
        name: impl Into<String>,
        attributes: Vec<AttributeSpec>,
    ) -> Result<Self, RegistryError> {
        Ok(Self {
            name: ComponentName::try_new(name)?,
            attributes: check_attribute_specs(attributes)?,
        })
    }

    pub fn name(&self) -> &ComponentName {
        &self.name
    }

    fn attribute_rule(&self, name: &str) -> Option<&AttributeRule> {
        self.attributes
            .values()
            .find(|attribute| attribute.name.as_str() == name)
            .map(|attribute| &attribute.rule)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AttributeKind {
    Bool,
    Static,
    Expression,
    Interpolated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributeRule {
    accepted: HashSet<AttributeKind>,
}

impl AttributeRule {
    pub fn one(kind: AttributeKind) -> Self {
        Self {
            accepted: HashSet::from([kind]),
        }
    }

    pub fn any(first: AttributeKind, rest: impl IntoIterator<Item = AttributeKind>) -> Self {
        let mut accepted = HashSet::from([first]);
        accepted.extend(rest);
        Self { accepted }
    }

    pub fn accepts(&self, kind: AttributeKind) -> bool {
        self.accepted.contains(&kind)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributeSpec {
    name: AttributeName,
    rule: AttributeRule,
}

impl AttributeSpec {
    pub fn try_new(name: impl Into<String>, rule: AttributeRule) -> Result<Self, RegistryError> {
        Ok(Self {
            name: AttributeName::try_new(name)?,
            rule,
        })
    }

    pub fn name(&self) -> &AttributeName {
        &self.name
    }

    pub fn rule(&self) -> &AttributeRule {
        &self.rule
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
    native_registry: &NativeElementRegistry,
    component_registry: &ComponentRegistry,
) -> Result<ValidatedTemplate, ValidationError> {
    let mut nodes = Vec::new();
    for node in document.nodes() {
        nodes.push(validate_node(node, native_registry, component_registry)?);
    }
    Ok(ValidatedTemplate { nodes })
}

fn validate_node(
    node: &Node,
    native_registry: &NativeElementRegistry,
    component_registry: &ComponentRegistry,
) -> Result<ValidatedNode, ValidationError> {
    match node {
        Node::Element(element) => {
            reject_duplicate_authored_attributes(element.attributes())?;
            match element.name() {
                ElementName::Native(name) => {
                    let Some(spec) = native_registry.spec(name) else {
                        return Err(ValidationError::new(
                            ValidationErrorKind::UnknownNativeElement {
                                name: name.as_str().to_owned(),
                            },
                            element.span(),
                        ));
                    };
                    validate_authored_attributes(name.as_str(), element.attributes(), spec)?;
                    let children =
                        validate_children(element.children(), native_registry, component_registry)?;
                    Ok(ValidatedNode::NativeElement(ValidatedElement {
                        name: name.clone(),
                        attributes: validate_attributes(element.attributes()),
                        children,
                        span: element.span(),
                    }))
                }
                ElementName::Component(name) => {
                    let Some(spec) = component_registry.spec(name) else {
                        return Err(ValidationError::new(
                            ValidationErrorKind::UnknownComponent {
                                name: name.as_str().to_owned(),
                            },
                            element.span(),
                        ));
                    };
                    validate_authored_attributes(name.as_str(), element.attributes(), spec)?;
                    let children =
                        validate_children(element.children(), native_registry, component_registry)?;
                    Ok(ValidatedNode::ComponentElement(ValidatedElement {
                        name: name.clone(),
                        attributes: validate_attributes(element.attributes()),
                        children,
                        span: element.span(),
                    }))
                }
            }
        }
        Node::Text(text) => Ok(ValidatedNode::Text(ValidatedText {
            value: text.value().to_owned(),
            span: text.span(),
        })),
        Node::Interpolation(interpolation) => {
            Ok(ValidatedNode::Interpolation(ValidatedInterpolation {
                expr: interpolation.expr().clone(),
                span: interpolation.span(),
            }))
        }
        Node::If(if_node) => {
            let mut branches = Vec::new();
            for branch in if_node.branches() {
                branches.push(ValidatedIfBranch {
                    condition: branch.condition().clone(),
                    children: validate_children(
                        branch.children(),
                        native_registry,
                        component_registry,
                    )?,
                });
            }
            Ok(ValidatedNode::If(ValidatedIf {
                branches,
                else_children: validate_children(
                    if_node.else_children(),
                    native_registry,
                    component_registry,
                )?,
                span: if_node.span(),
            }))
        }
        Node::ForEach(for_each) => Ok(ValidatedNode::ForEach(ValidatedForEach {
            collection: for_each.collection().clone(),
            item_name: for_each.item_name().clone(),
            children: validate_children(for_each.children(), native_registry, component_registry)?,
            else_children: validate_children(
                for_each.else_children(),
                native_registry,
                component_registry,
            )?,
            span: for_each.span(),
        })),
    }
}

trait ElementSpec {
    fn attribute_rule(&self, name: &str) -> Option<&AttributeRule>;
}

impl ElementSpec for NativeElementSpec {
    fn attribute_rule(&self, name: &str) -> Option<&AttributeRule> {
        self.attribute_rule(name)
    }
}

impl ElementSpec for ComponentSpec {
    fn attribute_rule(&self, name: &str) -> Option<&AttributeRule> {
        self.attribute_rule(name)
    }
}

fn check_attribute_specs(
    attributes: Vec<AttributeSpec>,
) -> Result<HashMap<AttributeName, AttributeSpec>, RegistryError> {
    let mut checked = HashMap::new();
    for attribute in attributes {
        let name = attribute.name.clone();
        if checked.insert(name.clone(), attribute).is_some() {
            return Err(RegistryError::DuplicateAttributeSpec {
                name: name.as_str().to_owned(),
            });
        }
    }
    Ok(checked)
}

fn reject_duplicate_authored_attributes(attributes: &[Attribute]) -> Result<(), ValidationError> {
    let mut seen = HashSet::new();
    for attribute in attributes {
        if !seen.insert(attribute.name()) {
            return Err(ValidationError::new(
                ValidationErrorKind::DuplicateAttribute {
                    name: attribute.name().to_owned(),
                },
                attribute.span(),
            ));
        }
    }
    Ok(())
}

fn validate_authored_attributes(
    element: &str,
    attributes: &[Attribute],
    spec: &impl ElementSpec,
) -> Result<(), ValidationError> {
    for attribute in attributes {
        let Some(rule) = spec.attribute_rule(attribute.name()) else {
            return Err(ValidationError::new(
                ValidationErrorKind::InvalidAttribute {
                    element: element.to_owned(),
                    attribute: attribute.name().to_owned(),
                },
                attribute.span(),
            ));
        };
        validate_attribute_value(element, attribute, rule)?;
    }
    Ok(())
}

fn validate_attribute_value(
    element: &str,
    attribute: &Attribute,
    rule: &AttributeRule,
) -> Result<(), ValidationError> {
    let kind = authored_attr_kind(attribute.value());
    if rule.accepts(kind) {
        return Ok(());
    }

    Err(ValidationError::new(
        ValidationErrorKind::InvalidAttributeValue {
            element: element.to_owned(),
            attribute: attribute.name().to_owned(),
        },
        attribute.span(),
    ))
}

fn authored_attr_kind(value: &AttrValue) -> AttributeKind {
    match value {
        AttrValue::Bool => AttributeKind::Bool,
        AttrValue::Static(_) => AttributeKind::Static,
        AttrValue::Expression(_) => AttributeKind::Expression,
        AttrValue::Interpolated(_) => AttributeKind::Interpolated,
    }
}

fn validate_children(
    children: &[Node],
    native_registry: &NativeElementRegistry,
    component_registry: &ComponentRegistry,
) -> Result<Vec<ValidatedNode>, ValidationError> {
    let mut validated = Vec::new();
    for child in children {
        validated.push(validate_node(child, native_registry, component_registry)?);
    }
    Ok(validated)
}

fn validate_attributes(attributes: &[Attribute]) -> Vec<ValidatedAttribute> {
    attributes
        .iter()
        .map(|attribute| ValidatedAttribute {
            name: AttributeName::try_new(attribute.name())
                .expect("authored attribute names are validated during parsing"),
            value: validate_attr_value(attribute.value()),
            span: attribute.span(),
        })
        .collect()
}

fn validate_attr_value(value: &AttrValue) -> ValidatedAttrValue {
    match value {
        AttrValue::Bool => ValidatedAttrValue::Bool,
        AttrValue::Static(value) => ValidatedAttrValue::Static(value.clone()),
        AttrValue::Expression(expr) => ValidatedAttrValue::Expression(expr.clone()),
        AttrValue::Interpolated(parts) => ValidatedAttrValue::Interpolated(
            parts
                .iter()
                .map(|part| match part {
                    AttrPart::Text(text) => ValidatedAttrPart::Text(text.clone()),
                    AttrPart::Expr(expr) => ValidatedAttrPart::Expr(expr.clone()),
                })
                .collect(),
        ),
    }
}
