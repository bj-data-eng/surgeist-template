use crate::expr::{BinaryOp, Expr, Literal, PathSegment, UnaryOp, VariablePath};
use crate::validate::{
    ValidatedAttrPart, ValidatedAttrValue, ValidatedAttribute, ValidatedElement, ValidatedForEach,
    ValidatedIf, ValidatedIfBranch, ValidatedNode, ValidatedTemplate,
};

pub fn render_to_rust(template: &ValidatedTemplate) -> String {
    format!(
        "::surgeist::template::template(vec![{}])",
        render_nodes(template.nodes())
    )
}

fn render_nodes(nodes: &[ValidatedNode]) -> String {
    nodes.iter().map(render_node).collect::<Vec<_>>().join(", ")
}

fn render_node(node: &ValidatedNode) -> String {
    match node {
        ValidatedNode::NativeElement(element) => render_native_element(element),
        ValidatedNode::ComponentElement(element) => render_component_element(element),
        ValidatedNode::Text(text) => {
            format!(
                "::surgeist::template::text({})",
                render_string(text.value())
            )
        }
        ValidatedNode::Interpolation(interpolation) => {
            format!(
                "::surgeist::template::expr({})",
                render_expr_arg(interpolation.expr())
            )
        }
        ValidatedNode::If(if_node) => render_if(if_node),
        ValidatedNode::ForEach(for_each) => render_for_each(for_each),
    }
}

fn render_native_element(element: &ValidatedElement<crate::NativeElementName>) -> String {
    format!(
        "::surgeist::template::native({}, vec![{}], vec![{}])",
        render_string(element.name().as_str()),
        render_attributes(element.attributes()),
        render_nodes(element.children())
    )
}

fn render_component_element(element: &ValidatedElement<crate::ComponentName>) -> String {
    format!(
        "::surgeist::template::component({}, vec![{}], vec![{}])",
        render_string(element.name().as_str()),
        render_attributes(element.attributes()),
        render_nodes(element.children())
    )
}

fn render_attributes(attributes: &[ValidatedAttribute]) -> String {
    attributes
        .iter()
        .map(render_attribute)
        .collect::<Vec<_>>()
        .join(", ")
}

fn render_attribute(attribute: &ValidatedAttribute) -> String {
    let name = render_string(attribute.name().as_str());
    match attribute.value() {
        ValidatedAttrValue::Bool => {
            format!("::surgeist::template::attr_bool({name})")
        }
        ValidatedAttrValue::Static(value) => {
            format!(
                "::surgeist::template::attr_static({name}, {})",
                render_string(value)
            )
        }
        ValidatedAttrValue::Expression(expr) => {
            format!(
                "::surgeist::template::attr_expr({name}, {})",
                render_expr_arg(expr)
            )
        }
        ValidatedAttrValue::Interpolated(parts) => {
            format!(
                "::surgeist::template::attr_interpolated({name}, vec![{}])",
                render_attr_parts(parts)
            )
        }
    }
}

fn render_attr_parts(parts: &[ValidatedAttrPart]) -> String {
    parts
        .iter()
        .map(|part| match part {
            ValidatedAttrPart::Text(text) => {
                format!("::surgeist::template::attr_text({})", render_string(text))
            }
            ValidatedAttrPart::Expr(expr) => {
                format!(
                    "::surgeist::template::attr_expr_part({})",
                    render_expr_arg(expr)
                )
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn render_if(if_node: &ValidatedIf) -> String {
    format!(
        "::surgeist::template::if_else(vec![{}], vec![{}])",
        render_if_branches(if_node.branches()),
        render_nodes(if_node.else_children())
    )
}

fn render_if_branches(branches: &[ValidatedIfBranch]) -> String {
    branches
        .iter()
        .map(|branch| {
            format!(
                "::surgeist::template::if_branch({}, vec![{}])",
                render_expr_arg(branch.condition()),
                render_nodes(branch.children())
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn render_for_each(for_each: &ValidatedForEach) -> String {
    format!(
        "::surgeist::template::for_each({}, {}, vec![{}], vec![{}])",
        render_expr_arg(for_each.collection()),
        render_string(for_each.item_name().as_str()),
        render_nodes(for_each.children()),
        render_nodes(for_each.else_children())
    )
}

fn render_expr_arg(expr: &Expr) -> String {
    render_string(&expr_source(expr))
}

fn expr_source(expr: &Expr) -> String {
    match expr {
        Expr::Literal(literal) => literal_source(literal),
        Expr::Variable(path) => variable_path_source(path),
        Expr::Unary { op, expr } => {
            format!("({}{})", render_unary_op(*op), expr_source(expr))
        }
        Expr::Binary { op, left, right } => {
            format!(
                "({} {} {})",
                expr_source(left),
                render_binary_op(*op),
                expr_source(right)
            )
        }
    }
}

fn literal_source(literal: &Literal) -> String {
    match literal {
        Literal::String(value) => render_string(value),
        Literal::Int(value) => value.to_string(),
        Literal::Float(value) if value.is_nan() => "NaN".to_owned(),
        Literal::Float(value) if value.is_infinite() && value.is_sign_positive() => {
            "inf".to_owned()
        }
        Literal::Float(value) if value.is_infinite() => "-inf".to_owned(),
        Literal::Float(value) => value.to_string(),
        Literal::Bool(true) => "true".to_owned(),
        Literal::Bool(false) => "false".to_owned(),
        Literal::Null => "null".to_owned(),
    }
}

fn variable_path_source(path: &VariablePath) -> String {
    let mut rendered = String::from("$");
    rendered.push_str(path.root());
    for segment in path.segments() {
        match segment {
            PathSegment::Field(field) => {
                rendered.push('.');
                rendered.push_str(field.as_str());
            }
            PathSegment::Index(index) => {
                rendered.push('[');
                rendered.push_str(&index.get().to_string());
                rendered.push(']');
            }
        }
    }
    rendered
}

fn render_unary_op(op: UnaryOp) -> &'static str {
    match op {
        UnaryOp::Not => "!",
        UnaryOp::Neg => "-",
    }
}

fn render_binary_op(op: BinaryOp) -> &'static str {
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

fn render_string(value: &str) -> String {
    let mut rendered = String::from("\"");
    for ch in value.chars() {
        match ch {
            '\\' => rendered.push_str("\\\\"),
            '"' => rendered.push_str("\\\""),
            '\n' => rendered.push_str("\\n"),
            '\r' => rendered.push_str("\\r"),
            '\t' => rendered.push_str("\\t"),
            ch if ch.is_control() => {
                rendered.push_str("\\u{");
                rendered.push_str(&format!("{:x}", ch as u32));
                rendered.push('}');
            }
            ch => rendered.push(ch),
        }
    }
    rendered.push('"');
    rendered
}

#[cfg(test)]
mod tests {
    use crate::expr::{Expr, Literal};

    use super::expr_source;

    #[test]
    fn expression_source_escapes_string_literals_deterministically() {
        let expr = Expr::Literal(Literal::String("line\n\"quoted\"\\tail".to_owned()));

        assert_eq!(expr_source(&expr), r#""line\n\"quoted\"\\tail""#);
    }

    #[test]
    fn expression_source_renders_non_finite_floats_deterministically() {
        assert_eq!(expr_source(&Expr::Literal(Literal::Float(f64::NAN))), "NaN");
        assert_eq!(
            expr_source(&Expr::Literal(Literal::Float(f64::INFINITY))),
            "inf"
        );
        assert_eq!(
            expr_source(&Expr::Literal(Literal::Float(f64::NEG_INFINITY))),
            "-inf"
        );
    }
}
