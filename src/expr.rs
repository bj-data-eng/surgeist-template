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
}
