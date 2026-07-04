use surgeist_template::{
    AttrPart, AttrValue, Expr, Literal, Node, ParseErrorKind, PathSegment, parse_template,
};

#[test]
fn parses_nested_elements_with_text_and_interpolation() {
    let document =
        parse_template("<Panel><div>Hello {$user.name}</div></Panel>").expect("template parses");

    assert_eq!(document.nodes().len(), 1);
    let Node::Element(panel) = &document.nodes()[0] else {
        panic!("expected component element");
    };
    assert_eq!(panel.name().as_str(), "Panel");

    let Node::Element(div) = &panel.children()[0] else {
        panic!("expected native element");
    };
    assert_eq!(div.name().as_str(), "div");
    assert_eq!(div.children().len(), 2);
    assert!(matches!(div.children()[0], Node::Text(_)));
    assert!(matches!(div.children()[1], Node::Interpolation(_)));
}

#[test]
fn rejects_mismatched_close_tag() {
    let error = parse_template("<div></span>").expect_err("mismatched close tag fails");

    assert!(matches!(
        error.kind(),
        ParseErrorKind::MismatchedCloseTag { expected, found }
            if expected == "div" && found == "span"
    ));
}

#[test]
fn parses_self_closing_elements_and_attribute_forms() {
    let document = parse_template(
        r#"<Panel disabled count={$count} title="Hello {$user.name}" data-id=main />"#,
    )
    .expect("template parses");

    let Node::Element(panel) = &document.nodes()[0] else {
        panic!("expected component element");
    };

    assert_eq!(panel.attributes().len(), 4);
    assert!(matches!(panel.attributes()[0].value(), AttrValue::Bool));
    assert!(matches!(
        panel.attributes()[1].value(),
        AttrValue::Expression(Expr::Variable(_))
    ));
    let AttrValue::Interpolated(parts) = panel.attributes()[2].value() else {
        panic!("expected interpolated attribute");
    };
    assert!(matches!(parts[0], AttrPart::Text(_)));
    assert!(matches!(parts[1], AttrPart::Expr(_)));
    assert_eq!(
        panel.attributes()[3].value(),
        &AttrValue::Static("main".to_owned())
    );
    assert!(panel.children().is_empty());
}

#[test]
fn parses_if_and_foreach_blocks() {
    let document = parse_template(
        r#"{if $visible}<Panel>{foreach $items as $item}<Text>{$item.name}</Text>{foreachelse}<Text>Empty</Text>{/foreach}</Panel>{elseif false}<Text>Hidden</Text>{else}<Text>Gone</Text>{/if}"#,
    )
    .expect("template parses");

    let Node::If(if_node) = &document.nodes()[0] else {
        panic!("expected if node");
    };
    assert_eq!(if_node.branches().len(), 2);
    assert_eq!(if_node.else_children().len(), 1);

    let Node::Element(panel) = &if_node.branches()[0].children()[0] else {
        panic!("expected panel child");
    };
    assert!(matches!(panel.children()[0], Node::ForEach(_)));
}

#[test]
fn rejects_stray_or_unknown_template_tags_in_nodes_and_attributes() {
    let stray = parse_template("{else}<Text>nope</Text>").expect_err("stray else fails");
    let closing = parse_template("{/foreach}").expect_err("stray foreach close fails");
    let unknown = parse_template("{foo bar}<Text>nope</Text>").expect_err("unknown tag fails");
    let quoted_attr =
        parse_template(r#"<Panel title="{foo}" />"#).expect_err("unknown quoted attr tag fails");
    let unquoted_attr =
        parse_template(r#"<Panel title={foo} />"#).expect_err("unknown unquoted attr tag fails");
    let embedded_attr =
        parse_template(r#"<Panel title=foo{bar} />"#).expect_err("embedded attr tag fails");
    let trailing_expr_attr =
        parse_template(r#"<Panel title={$foo}bar />"#).expect_err("trailing expression attr fails");

    assert!(matches!(
        stray.kind(),
        ParseErrorKind::StrayTemplateTag { tag } if tag == "else"
    ));
    assert!(matches!(
        closing.kind(),
        ParseErrorKind::StrayTemplateTag { tag } if tag == "/foreach"
    ));
    assert!(matches!(
        unknown.kind(),
        ParseErrorKind::UnsupportedTemplateTag { tag } if tag == "foo"
    ));
    assert!(matches!(
        quoted_attr.kind(),
        ParseErrorKind::UnsupportedTemplateTag { tag } if tag == "foo"
    ));
    assert!(matches!(
        unquoted_attr.kind(),
        ParseErrorKind::UnsupportedTemplateTag { tag } if tag == "foo"
    ));
    assert!(matches!(
        embedded_attr.kind(),
        ParseErrorKind::UnsupportedTemplateTag { tag } if tag == "bar"
    ));
    assert!(matches!(
        trailing_expr_attr.kind(),
        ParseErrorKind::UnexpectedToken {
            expected: "attribute boundary"
        }
    ));
}

#[test]
fn reports_multiline_text_and_utf8_foreach_item_error_spans() {
    let document = parse_template("<Panel>\n  Héllo\n</Panel>").expect("template parses");
    let Node::Element(element) = &document.nodes()[0] else {
        panic!("expected element");
    };
    let Node::Text(text) = &element.children()[0] else {
        panic!("expected text");
    };
    assert_eq!(text.span().start().line(), 1);
    assert_eq!(text.span().end().line(), 3);

    let foreach_error = parse_template("{foreach $items as $é}<Panel />{/foreach}")
        .expect_err("invalid foreach item fails");
    assert_eq!(foreach_error.span().start().column(), 20);
    assert_eq!(foreach_error.span().end().column(), 22);
    assert_eq!(foreach_error.span().len_bytes(), "$é".len());

    let missing_item_dollar = parse_template("{foreach $items as item}<Panel />{/foreach}")
        .expect_err("foreach item must use dollar prefix");
    assert_eq!(missing_item_dollar.span().start().column(), 20);

    let multiple_as = parse_template("{foreach $items as $item as $other}<Panel />{/foreach}")
        .expect_err("foreach header accepts exactly one as clause");
    assert!(matches!(
        multiple_as.kind(),
        ParseErrorKind::InvalidExpression { .. }
    ));
}

#[test]
fn parses_scalar_literals_and_indexed_variable_paths() {
    let document = parse_template(
        r#"<Panel a={true} b={false} c={null} d={42} e={3.5} f={"hi"} g={$items[0].name} />"#,
    )
    .expect("template parses");

    let Node::Element(panel) = &document.nodes()[0] else {
        panic!("expected element");
    };

    assert!(matches!(
        panel.attributes()[0].value(),
        AttrValue::Expression(Expr::Literal(Literal::Bool(true)))
    ));
    assert!(matches!(
        panel.attributes()[2].value(),
        AttrValue::Expression(Expr::Literal(Literal::Null))
    ));
    assert!(matches!(
        panel.attributes()[3].value(),
        AttrValue::Expression(Expr::Literal(Literal::Int(42)))
    ));
    assert!(matches!(
        panel.attributes()[4].value(),
        AttrValue::Expression(Expr::Literal(Literal::Float(3.5)))
    ));
    assert!(matches!(
        panel.attributes()[5].value(),
        AttrValue::Expression(Expr::Literal(Literal::String(value))) if value == "hi"
    ));
    let AttrValue::Expression(Expr::Variable(path)) = panel.attributes()[6].value() else {
        panic!("expected variable path");
    };
    assert_eq!(path.root(), "items");
    assert!(matches!(path.segments()[0], PathSegment::Index(_)));
    assert!(matches!(path.segments()[1], PathSegment::Field(_)));
}

#[test]
fn rejects_malformed_indexed_variable_paths() {
    for source in [
        r#"<Panel value={$} />"#,
        r#"<Panel value={$.name} />"#,
        r#"<Panel value={$items.} />"#,
        r#"<Panel value={$items..name} />"#,
        r#"<Panel value={$items[]} />"#,
        r#"<Panel value={$items[-1]} />"#,
        r#"<Panel value={$items[abc]} />"#,
        r#"<Panel value={$items[0} />"#,
        r#"<Panel value={$items[0][1]} />"#,
    ] {
        let error = parse_template(source).expect_err(source);
        assert!(matches!(
            error.kind(),
            ParseErrorKind::InvalidExpression { .. }
                | ParseErrorKind::UnexpectedEof
                | ParseErrorKind::UnexpectedToken { .. }
        ));
    }
}
