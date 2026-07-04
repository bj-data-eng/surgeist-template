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
        let root = root.into();
        if is_denied_variable_root(&root) {
            return Err(NameError::InvalidVariableName { name: root });
        }

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
    let tokens = tokenize(source)?;
    if tokens.is_empty() {
        return Err("empty expression");
    }

    let mut parser = ExprParser::new(tokens);
    let expr = parser.parse_expr(1)?;
    if parser.is_eof() {
        Ok(expr)
    } else {
        Err("trailing expression syntax")
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Literal(Literal),
    Variable(VariablePath),
    Unary(UnaryOp),
    Binary(BinaryOp),
    LParen,
    RParen,
}

struct ExprParser {
    tokens: Vec<Token>,
    pos: usize,
}

impl ExprParser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn is_eof(&self) -> bool {
        self.pos == self.tokens.len()
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn bump(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.pos).cloned()?;
        self.pos += 1;
        Some(token)
    }

    fn parse_expr(&mut self, min_precedence: u8) -> Result<Expr, &'static str> {
        let mut left = self.parse_unary()?;
        while let Some(Token::Binary(op)) = self.peek() {
            let op = *op;
            let precedence = binary_precedence(op);
            if precedence < min_precedence {
                break;
            }
            self.bump();
            let right = self.parse_expr(precedence + 1)?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, &'static str> {
        match self.peek() {
            Some(Token::Unary(UnaryOp::Not)) => {
                self.bump();
                Ok(Expr::Unary {
                    op: UnaryOp::Not,
                    expr: Box::new(self.parse_unary()?),
                })
            }
            Some(Token::Binary(BinaryOp::Sub)) => {
                self.bump();
                Ok(Expr::Unary {
                    op: UnaryOp::Neg,
                    expr: Box::new(self.parse_unary()?),
                })
            }
            _ => self.parse_primary(),
        }
    }

    fn parse_primary(&mut self) -> Result<Expr, &'static str> {
        match self.bump() {
            Some(Token::Literal(literal)) => Ok(Expr::Literal(literal)),
            Some(Token::Variable(path)) => Ok(Expr::Variable(path)),
            Some(Token::LParen) => {
                let expr = self.parse_expr(1)?;
                match self.bump() {
                    Some(Token::RParen) => Ok(expr),
                    _ => Err("unclosed parenthesized expression"),
                }
            }
            Some(Token::RParen) => Err("unexpected closing parenthesis"),
            Some(Token::Unary(_)) | Some(Token::Binary(_)) => Err("expected expression"),
            None => Err("expected expression"),
        }
    }
}

fn binary_precedence(op: BinaryOp) -> u8 {
    match op {
        BinaryOp::Or => 1,
        BinaryOp::And => 2,
        BinaryOp::Eq | BinaryOp::Ne | BinaryOp::Gt | BinaryOp::Ge | BinaryOp::Lt | BinaryOp::Le => {
            3
        }
        BinaryOp::Add | BinaryOp::Sub => 4,
        BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem => 5,
    }
}

fn tokenize(source: &str) -> Result<Vec<Token>, &'static str> {
    let mut tokens = Vec::new();
    let mut offset = 0;
    while offset < source.len() {
        let rest = &source[offset..];
        let ch = rest.chars().next().expect("offset is within source");
        if ch.is_whitespace() {
            offset += ch.len_utf8();
            continue;
        }

        if ch == '$' {
            let end = scan_variable_token(source, offset);
            let variable = &source[offset + 1..end];
            tokens.push(Token::Variable(parse_variable_path(variable)?));
            offset = end;
            continue;
        }

        if ch == '"' || ch == '\'' {
            let (literal, end) = parse_string_token(source, offset)?;
            tokens.push(Token::Literal(Literal::String(literal)));
            offset = end;
            continue;
        }

        if ch.is_ascii_digit() {
            let (literal, end) = parse_number_token(source, offset)?;
            tokens.push(Token::Literal(literal));
            offset = end;
            continue;
        }

        if is_identifier_start(ch) {
            let end = scan_identifier(source, offset);
            let ident = &source[offset..end];
            let literal = match ident {
                "true" => Literal::Bool(true),
                "false" => Literal::Bool(false),
                "null" => Literal::Null,
                _ => return Err("unsupported identifier"),
            };
            tokens.push(Token::Literal(literal));
            offset = end;
            continue;
        }

        if let Some((token, end)) = scan_symbol_token(source, offset)? {
            tokens.push(token);
            offset = end;
            continue;
        }

        return Err("unsupported expression syntax");
    }

    Ok(tokens)
}

fn scan_symbol_token(source: &str, offset: usize) -> Result<Option<(Token, usize)>, &'static str> {
    let rest = &source[offset..];
    for (symbol, token) in [
        ("||", Token::Binary(BinaryOp::Or)),
        ("&&", Token::Binary(BinaryOp::And)),
        ("==", Token::Binary(BinaryOp::Eq)),
        ("!=", Token::Binary(BinaryOp::Ne)),
        (">=", Token::Binary(BinaryOp::Ge)),
        ("<=", Token::Binary(BinaryOp::Le)),
    ] {
        if rest.starts_with(symbol) {
            return Ok(Some((token, offset + symbol.len())));
        }
    }

    if rest.starts_with("->") || rest.starts_with("??") {
        return Err("unsupported expression syntax");
    }

    let ch = rest.chars().next().expect("offset is within source");
    let token = match ch {
        '!' => Token::Unary(UnaryOp::Not),
        '-' => Token::Binary(BinaryOp::Sub),
        '>' => Token::Binary(BinaryOp::Gt),
        '<' => Token::Binary(BinaryOp::Lt),
        '+' => Token::Binary(BinaryOp::Add),
        '*' => Token::Binary(BinaryOp::Mul),
        '/' => Token::Binary(BinaryOp::Div),
        '%' => Token::Binary(BinaryOp::Rem),
        '(' => Token::LParen,
        ')' => Token::RParen,
        '=' | '?' | ':' | ',' | '[' | ']' | '.' | '&' | '|' => {
            return Err("unsupported expression syntax");
        }
        _ => return Ok(None),
    };
    Ok(Some((token, offset + ch.len_utf8())))
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
    if is_denied_variable_root(root) {
        return Err("unsupported variable root");
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

fn is_denied_variable_root(root: &str) -> bool {
    matches!(
        root,
        "GLOBALS"
            | "_REQUEST"
            | "_GET"
            | "_POST"
            | "_COOKIE"
            | "_SERVER"
            | "_SESSION"
            | "_ENV"
            | "_FILES"
    )
}

fn scan_variable_token(source: &str, offset: usize) -> usize {
    let mut end = offset + '$'.len_utf8();
    for (relative, ch) in source[end..].char_indices() {
        if ch.is_whitespace() || is_variable_boundary(ch) {
            return end + relative;
        }
    }
    end = source.len();
    end
}

fn is_variable_boundary(ch: char) -> bool {
    matches!(
        ch,
        '(' | ')'
            | '!'
            | '+'
            | '-'
            | '*'
            | '/'
            | '%'
            | '<'
            | '>'
            | '='
            | '&'
            | '|'
            | '?'
            | ':'
            | ','
    )
}

fn parse_string_token(source: &str, offset: usize) -> Result<(String, usize), &'static str> {
    let quote = source[offset..].chars().next().expect("checked offset");
    let mut value = String::new();
    let mut escaped = false;
    let mut cursor = offset + quote.len_utf8();

    while cursor < source.len() {
        let ch = source[cursor..].chars().next().expect("cursor in source");
        cursor += ch.len_utf8();
        if escaped {
            match ch {
                '\\' => value.push('\\'),
                '"' => value.push('"'),
                '\'' => value.push('\''),
                'n' => value.push('\n'),
                'r' => value.push('\r'),
                't' => value.push('\t'),
                _ => return Err("unsupported escape"),
            }
            escaped = false;
            continue;
        }

        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == quote {
            return Ok((value, cursor));
        }
        value.push(ch);
    }

    if escaped {
        Err("unterminated escape")
    } else {
        Err("unterminated string literal")
    }
}

fn parse_number_token(source: &str, offset: usize) -> Result<(Literal, usize), &'static str> {
    let mut cursor = offset;
    while let Some(ch) = source[cursor..].chars().next()
        && ch.is_ascii_digit()
    {
        cursor += ch.len_utf8();
    }

    let mut is_float = false;
    if source[cursor..].starts_with('.') {
        let after_dot = cursor + '.'.len_utf8();
        if let Some(next) = source[after_dot..].chars().next()
            && next.is_ascii_digit()
        {
            is_float = true;
            cursor = after_dot;
            while let Some(ch) = source[cursor..].chars().next()
                && ch.is_ascii_digit()
            {
                cursor += ch.len_utf8();
            }
        }
    }

    let value = &source[offset..cursor];
    if is_float {
        Ok((
            Literal::Float(value.parse::<f64>().map_err(|_| "invalid float literal")?),
            cursor,
        ))
    } else {
        Ok((
            Literal::Int(value.parse::<i64>().map_err(|_| "invalid int literal")?),
            cursor,
        ))
    }
}

fn scan_identifier(source: &str, offset: usize) -> usize {
    let mut end = offset;
    for (relative, ch) in source[offset..].char_indices() {
        if relative == 0 {
            end += ch.len_utf8();
            continue;
        }
        if !is_identifier_continue(ch) {
            break;
        }
        end += ch.len_utf8();
    }
    end
}

fn is_identifier_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

fn is_identifier_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

#[cfg(test)]
mod tests {
    use super::{
        BinaryOp, Expr, Literal, PathField, PathIndex, PathSegment, UnaryOp, VariablePath,
        parse_simple_expr,
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
    fn variable_path_rejects_escape_hatch_roots() {
        for root in [
            "GLOBALS", "_REQUEST", "_GET", "_POST", "_COOKIE", "_SERVER", "_SESSION", "_ENV",
            "_FILES",
        ] {
            assert!(VariablePath::try_new(root, Vec::new()).is_err(), "{root}");
        }
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
    fn parses_boolean_binary_condition() {
        let expr = parse_simple_expr("$visible && $enabled").expect("condition");

        assert!(matches!(
            expr,
            Expr::Binary {
                op: BinaryOp::And,
                ..
            }
        ));
    }

    #[test]
    fn parses_comparison_expression() {
        let expr = parse_simple_expr("$count > 0").expect("comparison");

        assert!(matches!(
            expr,
            Expr::Binary {
                op: BinaryOp::Gt,
                ..
            }
        ));
    }

    #[test]
    fn parses_parenthesized_unary_expression() {
        let expr = parse_simple_expr("!($visible && false)").expect("unary");

        let Expr::Unary {
            op: UnaryOp::Not,
            expr,
        } = expr
        else {
            panic!("expected not expression");
        };
        assert!(matches!(
            *expr,
            Expr::Binary {
                op: BinaryOp::And,
                ..
            }
        ));
    }

    #[test]
    fn parses_unary_minus_as_explicit_unary_expression() {
        let expr = parse_simple_expr("-42").expect("negative literal");

        assert!(matches!(
            expr,
            Expr::Unary {
                op: UnaryOp::Neg,
                expr,
            } if matches!(*expr, Expr::Literal(Literal::Int(42)))
        ));
    }

    #[test]
    fn parses_arithmetic_precedence() {
        let expr = parse_simple_expr("$a + 2 * 3").expect("arithmetic");

        let Expr::Binary {
            op: BinaryOp::Add,
            right,
            ..
        } = expr
        else {
            panic!("expected addition");
        };
        assert!(matches!(
            *right,
            Expr::Binary {
                op: BinaryOp::Mul,
                ..
            }
        ));
    }

    #[test]
    fn parses_same_precedence_binary_operators_left_associatively() {
        let expr = parse_simple_expr("$a - 2 - 1").expect("subtractions");

        let Expr::Binary {
            op: BinaryOp::Sub,
            left,
            right,
        } = expr
        else {
            panic!("expected outer subtraction");
        };
        assert!(matches!(*right, Expr::Literal(Literal::Int(1))));
        assert!(matches!(
            *left,
            Expr::Binary {
                op: BinaryOp::Sub,
                ..
            }
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

    #[test]
    fn rejects_unsupported_expression_forms() {
        for source in [
            "$format($name)",
            "$user->name",
            r#"$user.name = "Ada""#,
            "$[$a, $b]",
            "[$a, $b]",
            "$visible ? $enabled : false",
            "$name ?? \"Anonymous\"",
            "$a, $b",
        ] {
            assert!(parse_simple_expr(source).is_err(), "{source}");
        }
    }

    #[test]
    fn rejects_trailing_tokens_after_complete_expression() {
        for source in ["$a 1", "true false", "($a) $b"] {
            assert!(parse_simple_expr(source).is_err(), "{source}");
        }
    }

    #[test]
    fn rejects_request_and_global_escape_hatch_roots() {
        for source in [
            "$GLOBALS",
            "$GLOBALS.user",
            "$_REQUEST",
            "$_GET.name",
            "$_POST[0]",
            "$_COOKIE",
            "$_SERVER",
            "$_SESSION",
            "$_ENV",
            "$_FILES",
        ] {
            assert!(parse_simple_expr(source).is_err(), "{source}");
        }
    }
}
