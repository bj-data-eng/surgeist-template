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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum UnaryOp {
    Not,
    Neg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PathField(VariableName);

impl PathField {
    pub fn try_new(name: impl Into<String>) -> Result<Self, NameError> {
        Ok(Self(VariableName::try_new(name)?))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PathIndex(u64);

impl PathIndex {
    pub const fn new(index: u64) -> Self {
        Self(index)
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

pub fn parse_simple_expr(source: &str) -> Result<Expr, &'static str> {
    let trimmed = source.trim();
    if trimmed.is_empty() {
        return Err("empty expression");
    }

    match trimmed {
        "true" => return Ok(Expr::Literal(Literal::Bool(true))),
        "false" => return Ok(Expr::Literal(Literal::Bool(false))),
        "null" => return Ok(Expr::Literal(Literal::Null)),
        _ => {}
    }

    if let Some(path) = trimmed.strip_prefix('$') {
        return parse_variable_path(path).map(Expr::Variable);
    }

    if let Some(value) = parse_quoted_string(trimmed)? {
        return Ok(Expr::Literal(Literal::String(value)));
    }

    if let Ok(value) = trimmed.parse::<i64>() {
        return Ok(Expr::Literal(Literal::Int(value)));
    }

    if trimmed.contains('.')
        && let Ok(value) = trimmed.parse::<f64>()
    {
        return Ok(Expr::Literal(Literal::Float(value)));
    }

    Err("unsupported expression")
}

fn parse_variable_path(path: &str) -> Result<VariablePath, &'static str> {
    if path.is_empty() {
        return Err("empty variable path");
    }

    let mut rest = path;
    let root_end = rest.find(['.', '[']).unwrap_or(rest.len());
    let root = &rest[..root_end];
    if root.is_empty() {
        return Err("empty variable root");
    }
    rest = &rest[root_end..];

    let mut segments = Vec::new();
    while !rest.is_empty() {
        if let Some(after_dot) = rest.strip_prefix('.') {
            if after_dot.is_empty() {
                return Err("empty path field");
            }
            let field_end = after_dot.find(['.', '[']).unwrap_or(after_dot.len());
            let field = &after_dot[..field_end];
            if field.is_empty() {
                return Err("empty path field");
            }
            segments.push(PathSegment::Field(
                PathField::try_new(field).map_err(|_| "invalid path field")?,
            ));
            rest = &after_dot[field_end..];
            continue;
        }

        if let Some(after_open) = rest.strip_prefix('[') {
            let Some(close_index) = after_open.find(']') else {
                return Err("unclosed path index");
            };
            let index = &after_open[..close_index];
            if index.is_empty() {
                return Err("empty path index");
            }
            if !index.chars().all(|ch| ch.is_ascii_digit()) {
                return Err("invalid path index");
            }
            let parsed = index.parse::<u64>().map_err(|_| "invalid path index")?;
            segments.push(PathSegment::Index(PathIndex::new(parsed)));
            rest = &after_open[close_index + 1..];
            continue;
        }

        return Err("invalid variable path");
    }

    VariablePath::try_new(root, segments).map_err(|_| "invalid variable path")
}

fn parse_quoted_string(source: &str) -> Result<Option<String>, &'static str> {
    if !source.starts_with('"') && !source.starts_with('\'') {
        return Ok(None);
    }

    let quote = source.chars().next().expect("checked quote");
    if !source.ends_with(quote) || source.len() == quote.len_utf8() {
        return Err("unterminated string literal");
    }

    let inner = &source[quote.len_utf8()..source.len() - quote.len_utf8()];
    let mut value = String::new();
    let mut chars = inner.chars();
    while let Some(ch) = chars.next() {
        if ch == quote {
            return Err("unescaped quote in string literal");
        }
        if ch == '\\' {
            let Some(escaped) = chars.next() else {
                return Err("unterminated escape");
            };
            match escaped {
                '\\' => value.push('\\'),
                '"' => value.push('"'),
                '\'' => value.push('\''),
                'n' => value.push('\n'),
                'r' => value.push('\r'),
                't' => value.push('\t'),
                _ => return Err("unsupported escape"),
            }
        } else {
            value.push(ch);
        }
    }

    Ok(Some(value))
}

#[cfg(test)]
mod tests {
    use super::{
        BinaryOp, Expr, Literal, PathField, PathIndex, PathSegment, VariablePath, parse_simple_expr,
    };

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

        assert!(matches!(
            expr,
            Expr::Binary {
                op: BinaryOp::Eq,
                ..
            }
        ));
    }

    #[test]
    fn variable_path_rejects_adjacent_indexes() {
        let result = VariablePath::try_new(
            "items",
            vec![
                PathSegment::Index(PathIndex::new(0)),
                PathSegment::Index(PathIndex::new(1)),
            ],
        );

        assert!(result.is_err());
    }

    #[test]
    fn variable_path_validates_root_and_field_names() {
        assert!(VariablePath::try_new("2items", Vec::new()).is_err());
        assert!(PathField::try_new("profile-name").is_err());
    }

    #[test]
    fn path_index_exposes_value() {
        assert_eq!(PathIndex::new(42).get(), 42);
    }

    #[test]
    fn parses_task_three_simple_expressions() {
        assert!(matches!(
            parse_simple_expr("true").expect("bool"),
            Expr::Literal(Literal::Bool(true))
        ));
        assert!(matches!(
            parse_simple_expr("42").expect("int"),
            Expr::Literal(Literal::Int(42))
        ));
        assert!(matches!(
            parse_simple_expr("4.25").expect("float"),
            Expr::Literal(Literal::Float(4.25))
        ));
        assert!(matches!(
            parse_simple_expr(r#""hello""#).expect("string"),
            Expr::Literal(Literal::String(value)) if value == "hello"
        ));
        assert!(matches!(
            parse_simple_expr("$items[0].name").expect("path"),
            Expr::Variable(_)
        ));
    }

    #[test]
    fn rejects_task_three_malformed_paths() {
        for source in [
            "$",
            "$.name",
            "$items.",
            "$items..name",
            "$items[]",
            "$items[-1]",
            "$items[abc]",
            "$items[0",
            "$items[0][1]",
        ] {
            assert!(parse_simple_expr(source).is_err(), "{source}");
        }
    }
}
