use crate::ast::{AttrPart, AttrValue, Attribute, ElementName, IfBranch, Node, TemplateDocument};
use crate::error::{ParseError, ParseErrorKind};
use crate::expr::parse_simple_expr;
use crate::lexer::SourceCursor;
use crate::name::VariableName;
use crate::span::{SourcePos, SourceSpan};

pub fn parse_template(source: &str) -> Result<TemplateDocument, ParseError> {
    let mut parser = Parser::new(source);
    let nodes = parser.parse_nodes(&[])?;
    if !parser.cursor.is_eof() {
        let start = parser.cursor.pos();
        return Err(parser.error_here(
            ParseErrorKind::UnexpectedToken {
                expected: "template node",
            },
            start,
        ));
    }
    Ok(TemplateDocument::from_nodes(nodes))
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

    fn parse_nodes(&mut self, stop_tags: &[&str]) -> Result<Vec<Node>, ParseError> {
        let mut nodes = Vec::new();
        loop {
            self.skip_comments()?;
            if self.cursor.is_eof() || self.starts_any(stop_tags) || self.cursor.starts_with("</") {
                break;
            }

            if self.cursor.starts_with("<") {
                nodes.push(self.parse_element()?);
            } else if self.cursor.starts_with("{$") {
                nodes.push(self.parse_interpolation()?);
            } else if self.cursor.starts_with("{") {
                nodes.push(self.parse_template_tag()?);
            } else {
                nodes.push(self.parse_text());
            }
        }
        Ok(nodes)
    }

    fn parse_element(&mut self) -> Result<Node, ParseError> {
        let start = self.cursor.pos();
        self.cursor.expect("<")?;
        if self.cursor.starts_with("/") {
            return Err(self.error_here(
                ParseErrorKind::UnexpectedToken {
                    expected: "open tag",
                },
                start,
            ));
        }

        let name_start = self.cursor.pos();
        let raw_name = self.take_name_like();
        if raw_name.is_empty() {
            return Err(self.error_here(
                ParseErrorKind::UnexpectedToken {
                    expected: "element name",
                },
                name_start,
            ));
        }
        let name_span = SourceSpan::new_unchecked(name_start, self.cursor.pos());
        let name = classify_element_name(raw_name).map_err(|_| {
            self.cursor.error(
                ParseErrorKind::InvalidName {
                    name: raw_name.to_owned(),
                },
                name_span,
            )
        })?;

        let (attributes, self_closing) = self.parse_attributes()?;
        if self_closing {
            return Ok(Node::element(
                name,
                attributes,
                Vec::new(),
                SourceSpan::new_unchecked(start, self.cursor.pos()),
            ));
        }

        let children = self.parse_nodes(&[])?;
        if !self.cursor.starts_with("</") {
            return Err(self.error_here(
                ParseErrorKind::UnclosedElement {
                    name: raw_name.to_owned(),
                },
                start,
            ));
        }

        self.cursor.expect("</")?;
        let close_start = self.cursor.pos();
        let close_name = self.take_name_like();
        if close_name.is_empty() {
            return Err(self.error_here(
                ParseErrorKind::UnexpectedToken {
                    expected: "close tag name",
                },
                close_start,
            ));
        }
        self.cursor.skip_ws();
        self.cursor.expect(">")?;
        if close_name != raw_name {
            return Err(self.cursor.error(
                ParseErrorKind::MismatchedCloseTag {
                    expected: raw_name.to_owned(),
                    found: close_name.to_owned(),
                },
                SourceSpan::new_unchecked(close_start, self.cursor.pos()),
            ));
        }

        Ok(Node::element(
            name,
            attributes,
            children,
            SourceSpan::new_unchecked(start, self.cursor.pos()),
        ))
    }

    fn parse_attributes(&mut self) -> Result<(Vec<Attribute>, bool), ParseError> {
        let mut attributes = Vec::new();
        loop {
            self.cursor.skip_ws();
            if self.cursor.starts_with("/>") {
                self.cursor.expect("/>")?;
                return Ok((attributes, true));
            }
            if self.cursor.starts_with(">") {
                self.cursor.expect(">")?;
                return Ok((attributes, false));
            }
            if self.cursor.is_eof() {
                return Err(self.unexpected_eof());
            }
            if self.cursor.starts_with("{") {
                return Err(self.unsupported_current_brace_tag()?);
            }

            let attr_start = self.cursor.pos();
            let name = self.take_attr_name();
            if name.is_empty() {
                return Err(self.error_here(
                    ParseErrorKind::UnexpectedToken {
                        expected: "attribute name",
                    },
                    attr_start,
                ));
            }

            let value = if self.consume_ws_then_equals()? {
                self.cursor.skip_ws();
                self.parse_attr_value()?
            } else {
                AttrValue::Bool
            };

            let attr_span = SourceSpan::new_unchecked(attr_start, self.cursor.pos());
            let attribute = Attribute::try_new(name, value, attr_span).map_err(|_| {
                self.cursor.error(
                    ParseErrorKind::InvalidName {
                        name: name.to_owned(),
                    },
                    attr_span,
                )
            })?;
            attributes.push(attribute);
        }
    }

    fn parse_attr_value(&mut self) -> Result<AttrValue, ParseError> {
        if self.cursor.starts_with("{") {
            let expr = self.parse_braced_expression()?;
            if !self.is_attr_boundary() {
                return Err(self.error_here(
                    ParseErrorKind::UnexpectedToken {
                        expected: "attribute boundary",
                    },
                    self.cursor.pos(),
                ));
            }
            return Ok(AttrValue::Expression(expr));
        }
        if self.cursor.starts_with("\"") || self.cursor.starts_with("'") {
            return self.parse_quoted_attr_value();
        }

        let value_start = self.cursor.pos();
        let start_byte = value_start.byte();
        while let Some(ch) = self.cursor.peek() {
            if ch.is_whitespace() || ch == '>' || ch == '/' {
                break;
            }
            if ch == '{' {
                return Err(self.unsupported_current_brace_tag()?);
            }
            self.cursor.bump();
        }
        if self.cursor.pos().byte() == start_byte {
            return Err(self.error_here(
                ParseErrorKind::UnexpectedToken {
                    expected: "attribute value",
                },
                value_start,
            ));
        }
        Ok(AttrValue::Static(
            self.slice(start_byte, self.cursor.pos().byte()).to_owned(),
        ))
    }

    fn parse_quoted_attr_value(&mut self) -> Result<AttrValue, ParseError> {
        let quote = self.cursor.bump().expect("checked quote");
        let mut parts = Vec::new();
        let mut text_start = self.cursor.pos().byte();

        loop {
            if self.cursor.is_eof() {
                return Err(self.unexpected_eof());
            }
            if self.cursor.peek() == Some(quote) {
                if text_start < self.cursor.pos().byte() {
                    parts.push(AttrPart::Text(
                        self.slice(text_start, self.cursor.pos().byte()).to_owned(),
                    ));
                }
                self.cursor.bump();
                break;
            }
            if self.cursor.starts_with("{$") {
                if text_start < self.cursor.pos().byte() {
                    parts.push(AttrPart::Text(
                        self.slice(text_start, self.cursor.pos().byte()).to_owned(),
                    ));
                }
                parts.push(AttrPart::Expr(self.parse_braced_expression()?));
                text_start = self.cursor.pos().byte();
                continue;
            }
            if self.cursor.starts_with("{") {
                return Err(self.unsupported_current_brace_tag()?);
            }
            self.cursor.bump();
        }

        if parts.is_empty() {
            Ok(AttrValue::Static(String::new()))
        } else if parts.len() == 1 {
            match parts.pop().expect("one part") {
                AttrPart::Text(value) => Ok(AttrValue::Static(value)),
                AttrPart::Expr(expr) => Ok(AttrValue::Interpolated(vec![AttrPart::Expr(expr)])),
            }
        } else {
            Ok(AttrValue::Interpolated(parts))
        }
    }

    fn parse_template_tag(&mut self) -> Result<Node, ParseError> {
        let tag = self.read_brace_tag()?;
        let content = tag.content.trim();
        if let Some(header) = content.strip_prefix("if").and_then(strip_required_space) {
            let header_span = self.subspan_for(tag.content, header, tag.content_span);
            return self.parse_if(tag.start, header, header_span);
        }
        if let Some(header) = content
            .strip_prefix("foreach")
            .and_then(strip_required_space)
        {
            let header_span = self.subspan_for(tag.content, header, tag.content_span);
            return self.parse_foreach(tag.start, header, header_span);
        }

        let name = tag_name(content);
        if is_stray_tag(name) {
            return Err(self.cursor.error(
                ParseErrorKind::StrayTemplateTag {
                    tag: name.to_owned(),
                },
                tag.full_span,
            ));
        }
        Err(self.cursor.error(
            ParseErrorKind::UnsupportedTemplateTag {
                tag: name.to_owned(),
            },
            tag.full_span,
        ))
    }

    fn parse_if(
        &mut self,
        start: SourcePos,
        header: &str,
        header_span: SourceSpan,
    ) -> Result<Node, ParseError> {
        let first_condition = parse_simple_expr(header).map_err(|reason| {
            self.cursor
                .error(ParseErrorKind::InvalidExpression { reason }, header_span)
        })?;
        let first_children = self.parse_nodes(&["{elseif", "{else}", "{/if}"])?;
        let mut branches = vec![IfBranch::new(first_condition, first_children)];
        let mut else_children = Vec::new();

        loop {
            if self.cursor.starts_with("{elseif") {
                let tag = self.read_brace_tag()?;
                let content = tag.content.trim();
                let header = content
                    .strip_prefix("elseif")
                    .and_then(strip_required_space)
                    .ok_or_else(|| {
                        self.cursor.error(
                            ParseErrorKind::UnsupportedTemplateTag {
                                tag: tag_name(content).to_owned(),
                            },
                            tag.full_span,
                        )
                    })?;
                let condition = parse_simple_expr(header).map_err(|reason| {
                    self.cursor.error(
                        ParseErrorKind::InvalidExpression { reason },
                        tag.content_span,
                    )
                })?;
                let children = self.parse_nodes(&["{elseif", "{else}", "{/if}"])?;
                branches.push(IfBranch::new(condition, children));
            } else if self.cursor.starts_with("{else}") {
                self.read_brace_tag()?;
                else_children = self.parse_nodes(&["{/if}"])?;
                self.cursor.expect("{/if}")?;
                break;
            } else if self.cursor.starts_with("{/if}") {
                self.cursor.expect("{/if}")?;
                break;
            } else {
                return Err(self.unexpected_eof());
            }
        }

        Ok(Node::if_block(
            branches,
            else_children,
            SourceSpan::new_unchecked(start, self.cursor.pos()),
        ))
    }

    fn parse_foreach(
        &mut self,
        start: SourcePos,
        header: &str,
        header_span: SourceSpan,
    ) -> Result<Node, ParseError> {
        let (collection_source, item_source, item_span) =
            self.parse_foreach_header(header, header_span)?;
        let collection = parse_simple_expr(collection_source).map_err(|reason| {
            self.cursor
                .error(ParseErrorKind::InvalidExpression { reason }, header_span)
        })?;
        let item_name = item_source.strip_prefix('$').ok_or_else(|| {
            self.cursor.error(
                ParseErrorKind::InvalidExpression {
                    reason: "foreach item must start with $",
                },
                item_span,
            )
        })?;
        let item_name = VariableName::try_new(item_name).map_err(|_| {
            self.cursor.error(
                ParseErrorKind::InvalidExpression {
                    reason: "invalid foreach item name",
                },
                item_span,
            )
        })?;

        let children = self.parse_nodes(&["{foreachelse}", "{/foreach}"])?;
        let mut else_children = Vec::new();
        if self.cursor.starts_with("{foreachelse}") {
            self.cursor.expect("{foreachelse}")?;
            else_children = self.parse_nodes(&["{/foreach}"])?;
        }
        if !self.cursor.starts_with("{/foreach}") {
            return Err(self.unexpected_eof());
        }
        self.cursor.expect("{/foreach}")?;

        Ok(Node::foreach(
            collection,
            item_name,
            children,
            else_children,
            SourceSpan::new_unchecked(start, self.cursor.pos()),
        ))
    }

    fn parse_foreach_header<'b>(
        &self,
        header: &'b str,
        header_span: SourceSpan,
    ) -> Result<(&'b str, &'b str, SourceSpan), ParseError> {
        let tokens = whitespace_tokens(header, header_span.start().byte());
        let as_positions: Vec<usize> = tokens
            .iter()
            .enumerate()
            .filter_map(|(index, token)| (token.text == "as").then_some(index))
            .collect();
        if as_positions.len() != 1 {
            return Err(self.cursor.error(
                ParseErrorKind::InvalidExpression {
                    reason: "foreach header must contain exactly one as clause",
                },
                header_span,
            ));
        }

        let as_index = as_positions[0];
        if as_index == 0 || as_index + 2 != tokens.len() {
            return Err(self.cursor.error(
                ParseErrorKind::InvalidExpression {
                    reason: "foreach header must contain collection, as, and one item",
                },
                header_span,
            ));
        }

        let as_token = &tokens[as_index];
        let collection_end = as_token.start - header_span.start().byte();
        let collection = header[..collection_end].trim();
        let item = &tokens[as_index + 1];
        Ok((
            collection,
            item.text,
            SourceSpan::new_unchecked(self.pos_at(item.start), self.pos_at(item.end)),
        ))
    }

    fn parse_interpolation(&mut self) -> Result<Node, ParseError> {
        let start = self.cursor.pos();
        let expr = self.parse_braced_expression()?;
        Ok(Node::interpolation(
            expr,
            SourceSpan::new_unchecked(start, self.cursor.pos()),
        ))
    }

    fn parse_braced_expression(&mut self) -> Result<crate::expr::Expr, ParseError> {
        let tag = self.read_brace_tag()?;
        let content = tag.content.trim();
        parse_simple_expr(content).map_err(|reason| {
            if content.starts_with('$')
                || content.starts_with('"')
                || content.starts_with('\'')
                || content
                    .chars()
                    .next()
                    .is_some_and(|ch| ch == '-' || ch == '+' || ch.is_ascii_digit())
            {
                self.cursor.error(
                    ParseErrorKind::InvalidExpression { reason },
                    tag.content_span,
                )
            } else {
                self.cursor.error(
                    ParseErrorKind::UnsupportedTemplateTag {
                        tag: tag_name(content).to_owned(),
                    },
                    tag.full_span,
                )
            }
        })
    }

    fn parse_text(&mut self) -> Node {
        let start = self.cursor.pos();
        let start_byte = start.byte();
        while !self.cursor.is_eof()
            && !self.cursor.starts_with("<")
            && !self.cursor.starts_with("{")
        {
            self.cursor.bump();
        }
        Node::text(
            self.slice(start_byte, self.cursor.pos().byte()).to_owned(),
            SourceSpan::new_unchecked(start, self.cursor.pos()),
        )
    }

    fn skip_comments(&mut self) -> Result<(), ParseError> {
        while self.cursor.starts_with("{*") {
            self.cursor.expect("{*")?;
            if self.cursor.take_until("*}").is_none() {
                return Err(self.unexpected_eof());
            }
            self.cursor.expect("*}")?;
        }
        Ok(())
    }

    fn read_brace_tag(&mut self) -> Result<BraceTag<'a>, ParseError> {
        let start = self.cursor.pos();
        self.cursor.expect("{")?;
        let Some((content, content_span)) = self.cursor.take_until("}") else {
            return Err(self.unexpected_eof());
        };
        self.cursor.expect("}")?;
        Ok(BraceTag {
            start,
            content,
            content_span,
            full_span: SourceSpan::new_unchecked(start, self.cursor.pos()),
        })
    }

    fn unsupported_current_brace_tag(&mut self) -> Result<ParseError, ParseError> {
        let tag = self.read_brace_tag()?;
        let name = tag_name(tag.content.trim());
        Ok(self.cursor.error(
            ParseErrorKind::UnsupportedTemplateTag {
                tag: name.to_owned(),
            },
            tag.full_span,
        ))
    }

    fn consume_ws_then_equals(&mut self) -> Result<bool, ParseError> {
        self.cursor.skip_ws();
        if self.cursor.starts_with("=") {
            self.cursor.expect("=")?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn take_name_like(&mut self) -> &'a str {
        let start = self.cursor.pos().byte();
        while let Some(ch) = self.cursor.peek() {
            if ch.is_whitespace() || ch == '>' || ch == '/' {
                break;
            }
            self.cursor.bump();
        }
        self.slice(start, self.cursor.pos().byte())
    }

    fn take_attr_name(&mut self) -> &'a str {
        let start = self.cursor.pos().byte();
        while let Some(ch) = self.cursor.peek() {
            if ch.is_whitespace() || ch == '=' || ch == '>' || ch == '/' || ch == '{' {
                break;
            }
            self.cursor.bump();
        }
        self.slice(start, self.cursor.pos().byte())
    }

    fn starts_any(&self, patterns: &[&str]) -> bool {
        patterns
            .iter()
            .any(|pattern| self.cursor.starts_with(pattern))
    }

    fn is_attr_boundary(&self) -> bool {
        self.cursor.is_eof()
            || self
                .cursor
                .peek()
                .is_some_and(|ch| ch.is_whitespace() || ch == '>' || ch == '/')
    }

    fn unexpected_eof(&self) -> ParseError {
        self.cursor.error(
            ParseErrorKind::UnexpectedEof,
            SourceSpan::new_unchecked(self.cursor.pos(), self.cursor.pos()),
        )
    }

    fn error_here(&self, kind: ParseErrorKind, start: SourcePos) -> ParseError {
        self.cursor
            .error(kind, SourceSpan::new_unchecked(start, self.cursor.pos()))
    }

    fn slice(&self, start: usize, end: usize) -> &'a str {
        &self.cursor.source()[start..end]
    }

    fn pos_at(&self, byte: usize) -> SourcePos {
        let mut line = 1;
        let mut column = 1;
        let mut current = 0;
        for ch in self.cursor.source().chars() {
            if current >= byte {
                break;
            }
            current += ch.len_utf8();
            if ch == '\n' {
                line += 1;
                column = 1;
            } else {
                column += 1;
            }
        }
        SourcePos::new_unchecked(line, column, byte)
    }

    fn subspan_for(&self, haystack: &str, needle: &str, haystack_span: SourceSpan) -> SourceSpan {
        let offset = haystack.find(needle).unwrap_or(0);
        let start = haystack_span.start().byte() + offset;
        let end = start + needle.len();
        SourceSpan::new_unchecked(self.pos_at(start), self.pos_at(end))
    }
}

struct BraceTag<'a> {
    start: SourcePos,
    content: &'a str,
    content_span: SourceSpan,
    full_span: SourceSpan,
}

struct HeaderToken<'a> {
    text: &'a str,
    start: usize,
    end: usize,
}

fn whitespace_tokens(source: &str, absolute_start: usize) -> Vec<HeaderToken<'_>> {
    let mut tokens = Vec::new();
    let mut token_start = None;
    for (offset, ch) in source.char_indices() {
        if ch.is_whitespace() {
            if let Some(start) = token_start.take() {
                tokens.push(HeaderToken {
                    text: &source[start..offset],
                    start: absolute_start + start,
                    end: absolute_start + offset,
                });
            }
        } else if token_start.is_none() {
            token_start = Some(offset);
        }
    }
    if let Some(start) = token_start {
        tokens.push(HeaderToken {
            text: &source[start..],
            start: absolute_start + start,
            end: absolute_start + source.len(),
        });
    }
    tokens
}

fn classify_element_name(name: &str) -> Result<ElementName, ()> {
    let Some(first) = name.chars().next() else {
        return Err(());
    };
    if first.is_ascii_uppercase() {
        ElementName::component(name).map_err(|_| ())
    } else if first.is_ascii_lowercase() {
        ElementName::native(name).map_err(|_| ())
    } else {
        Err(())
    }
}

fn strip_required_space(source: &str) -> Option<&str> {
    source
        .chars()
        .next()
        .is_some_and(char::is_whitespace)
        .then(|| source.trim())
}

fn tag_name(content: &str) -> &str {
    content.split_whitespace().next().unwrap_or(content).trim()
}

fn is_stray_tag(name: &str) -> bool {
    matches!(name, "else" | "elseif" | "foreachelse" | "/if" | "/foreach")
}
