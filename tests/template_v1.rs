use surgeist_template::{
    AttrPart, AttrValue, AttributeKind, AttributeRule, AttributeSpec, BinaryOp, ComponentRegistry,
    ComponentSpec, Expr, Literal, NativeElementRegistry, NativeElementSpec, Node, ParseErrorKind,
    PathSegment, RegistryError, UnaryOp, ValidationErrorKind, parse_template, render_to_rust,
    validate_template,
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
fn parses_expression_operators_in_template_syntax() {
    let document = parse_template(
        r#"{if !($visible && false)}<Panel count={$count > 0} total={$a + 2 * 3} />{/if}"#,
    )
    .expect("template parses");

    let Node::If(if_node) = &document.nodes()[0] else {
        panic!("expected if node");
    };
    let Expr::Unary {
        op: UnaryOp::Not,
        expr,
    } = if_node.branches()[0].condition()
    else {
        panic!("expected not condition");
    };
    assert!(matches!(
        **expr,
        Expr::Binary {
            op: BinaryOp::And,
            ..
        }
    ));

    let Node::Element(panel) = &if_node.branches()[0].children()[0] else {
        panic!("expected panel child");
    };
    assert!(matches!(
        panel.attributes()[0].value(),
        AttrValue::Expression(Expr::Binary {
            op: BinaryOp::Gt,
            ..
        })
    ));

    let AttrValue::Expression(Expr::Binary {
        op: BinaryOp::Add,
        right,
        ..
    }) = panel.attributes()[1].value()
    else {
        panic!("expected addition attribute");
    };
    assert!(matches!(
        **right,
        Expr::Binary {
            op: BinaryOp::Mul,
            ..
        }
    ));
}

#[test]
fn parses_spaced_foreach_collection_expression() {
    let document = parse_template(r#"{foreach $a + $b as $item}<Text>{$item}</Text>{/foreach}"#)
        .expect("template parses");

    let Node::ForEach(for_each) = &document.nodes()[0] else {
        panic!("expected foreach node");
    };
    assert!(matches!(
        for_each.collection(),
        Expr::Binary {
            op: BinaryOp::Add,
            ..
        }
    ));
    assert_eq!(for_each.item_name().as_str(), "item");
}

#[test]
fn parses_foreach_header_with_flexible_as_whitespace() {
    for source in [
        r#"{foreach $items  as $item}<Text>{$item}</Text>{/foreach}"#,
        "{foreach $items\tas\t$item}<Text>{$item}</Text>{/foreach}",
        "{foreach $items\nas\n$item}<Text>{$item}</Text>{/foreach}",
    ] {
        let document = parse_template(source).expect(source);

        let Node::ForEach(for_each) = &document.nodes()[0] else {
            panic!("expected foreach node");
        };
        assert!(matches!(for_each.collection(), Expr::Variable(_)));
        assert_eq!(for_each.item_name().as_str(), "item");
    }
}

#[test]
fn rejects_extra_tokens_after_foreach_item_binding() {
    let error = parse_template("{foreach $items as $item extra}<Text>{$item}</Text>{/foreach}")
        .expect_err("extra item token fails");

    assert!(matches!(
        error.kind(),
        ParseErrorKind::InvalidExpression { .. }
    ));
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

#[test]
fn validates_known_native_and_component_names() {
    let document = parse_template("<Panel><div>Body</div></Panel>").expect("template parses");
    let native = NativeElementRegistry::try_from_specs(vec![
        NativeElementSpec::try_new("div", Vec::new()).expect("valid native spec"),
    ])
    .expect("valid native registry");
    let components = ComponentRegistry::try_from_specs(vec![
        ComponentSpec::try_new("Panel", Vec::new()).expect("valid component spec"),
    ])
    .expect("valid component registry");

    let validated = validate_template(&document, &native, &components).expect("template validates");

    assert_eq!(validated.nodes().len(), 1);
}

#[test]
fn rejects_unknown_component() {
    let document = parse_template("<MissingWidget />").expect("template parses");
    let native = NativeElementRegistry::try_from_specs(Vec::new()).expect("valid native registry");
    let components = ComponentRegistry::try_from_specs(vec![
        ComponentSpec::try_new("Panel", Vec::new()).expect("valid component spec"),
    ])
    .expect("valid component registry");

    let error =
        validate_template(&document, &native, &components).expect_err("unknown component fails");

    assert!(matches!(
        error.kind(),
        ValidationErrorKind::UnknownComponent { name } if name == "MissingWidget"
    ));
}

#[test]
fn rejects_unknown_native_element() {
    let document = parse_template("<section />").expect("template parses");
    let native = NativeElementRegistry::try_from_specs(vec![
        NativeElementSpec::try_new("div", Vec::new()).expect("valid native spec"),
    ])
    .expect("valid native registry");
    let components =
        ComponentRegistry::try_from_specs(Vec::new()).expect("valid component registry");

    let error =
        validate_template(&document, &native, &components).expect_err("unknown native fails");

    assert!(matches!(
        error.kind(),
        ValidationErrorKind::UnknownNativeElement { name } if name == "section"
    ));
}

#[test]
fn rejects_duplicate_authored_attributes() {
    let document = parse_template(r#"<Panel title="one" title="two" />"#).expect("template parses");
    let native = NativeElementRegistry::try_from_specs(Vec::new()).expect("valid native registry");
    let components = ComponentRegistry::try_from_specs(vec![
        ComponentSpec::try_new(
            "Panel",
            vec![
                AttributeSpec::try_new("title", AttributeRule::one(AttributeKind::Static))
                    .expect("valid attr spec"),
            ],
        )
        .expect("valid component spec"),
    ])
    .expect("valid component registry");

    let error =
        validate_template(&document, &native, &components).expect_err("duplicate attrs fail");

    assert!(matches!(
        error.kind(),
        ValidationErrorKind::DuplicateAttribute { name } if name == "title"
    ));
}

#[test]
fn rejects_unknown_attribute_and_invalid_value_kind() {
    let native = NativeElementRegistry::try_from_specs(Vec::new()).expect("valid native registry");
    let components = ComponentRegistry::try_from_specs(vec![
        ComponentSpec::try_new(
            "Panel",
            vec![
                AttributeSpec::try_new("title", AttributeRule::one(AttributeKind::Expression))
                    .expect("valid attr spec"),
            ],
        )
        .expect("valid component spec"),
    ])
    .expect("valid component registry");

    let unknown = parse_template(r#"<Panel missing="value" />"#).expect("template parses");
    let unknown_error =
        validate_template(&unknown, &native, &components).expect_err("unknown attr fails");
    assert!(matches!(
        unknown_error.kind(),
        ValidationErrorKind::InvalidAttribute { element, attribute }
            if element == "Panel" && attribute == "missing"
    ));

    let wrong_kind = parse_template(r#"<Panel title="static" />"#).expect("template parses");
    let wrong_kind_error =
        validate_template(&wrong_kind, &native, &components).expect_err("wrong attr kind fails");
    assert!(matches!(
        wrong_kind_error.kind(),
        ValidationErrorKind::InvalidAttributeValue { element, attribute }
            if element == "Panel" && attribute == "title"
    ));
}

#[test]
fn rejects_duplicate_attribute_specs() {
    let error = ComponentSpec::try_new(
        "Panel",
        vec![
            AttributeSpec::try_new("title", AttributeRule::one(AttributeKind::Static))
                .expect("valid attr spec"),
            AttributeSpec::try_new("title", AttributeRule::one(AttributeKind::Expression))
                .expect("valid attr spec"),
        ],
    )
    .expect_err("duplicate attr spec fails");

    assert!(matches!(
        error,
        RegistryError::DuplicateAttributeSpec { name } if name == "title"
    ));
}

#[test]
fn rejects_duplicate_registry_specs_and_invalid_spec_names() {
    let duplicate_native = NativeElementRegistry::try_from_specs(vec![
        NativeElementSpec::try_new("div", Vec::new()).expect("valid native spec"),
        NativeElementSpec::try_new("div", Vec::new()).expect("valid native spec"),
    ])
    .expect_err("duplicate native spec fails");
    assert!(matches!(
        duplicate_native,
        RegistryError::DuplicateNativeElement { name } if name == "div"
    ));

    let duplicate_component = ComponentRegistry::try_from_specs(vec![
        ComponentSpec::try_new("Panel", Vec::new()).expect("valid component spec"),
        ComponentSpec::try_new("Panel", Vec::new()).expect("valid component spec"),
    ])
    .expect_err("duplicate component spec fails");
    assert!(matches!(
        duplicate_component,
        RegistryError::DuplicateComponent { name } if name == "Panel"
    ));

    let invalid_native =
        NativeElementSpec::try_new("Panel", Vec::new()).expect_err("invalid native name fails");
    assert!(matches!(invalid_native, RegistryError::Name(_)));

    let invalid_component =
        ComponentSpec::try_new("panel", Vec::new()).expect_err("invalid component name fails");
    assert!(matches!(invalid_component, RegistryError::Name(_)));
}

#[test]
fn multi_kind_attribute_rule_accepts_interpolated_title() {
    let document =
        parse_template(r#"<Panel title="Hello {$user.name}" />"#).expect("template parses");
    let native = NativeElementRegistry::try_from_specs(Vec::new()).expect("valid native registry");
    let components = ComponentRegistry::try_from_specs(vec![
        ComponentSpec::try_new(
            "Panel",
            vec![
                AttributeSpec::try_new(
                    "title",
                    AttributeRule::any(
                        AttributeKind::Static,
                        [AttributeKind::Interpolated, AttributeKind::Expression],
                    ),
                )
                .expect("valid attr spec"),
            ],
        )
        .expect("valid component spec"),
    ])
    .expect("valid component registry");

    validate_template(&document, &native, &components).expect("interpolated title validates");
}

#[test]
fn parent_unknown_component_error_wins_over_child_validation() {
    let document =
        parse_template("<MissingWidget><UnknownChild /></MissingWidget>").expect("template parses");
    let native = NativeElementRegistry::try_from_specs(Vec::new()).expect("valid native registry");
    let components =
        ComponentRegistry::try_from_specs(Vec::new()).expect("valid component registry");

    let error = validate_template(&document, &native, &components).expect_err("validation fails");

    assert!(matches!(
        error.kind(),
        ValidationErrorKind::UnknownComponent { name } if name == "MissingWidget"
    ));
}

#[test]
fn renders_validated_component_with_expression_attribute_and_interpolation_child() {
    let document = parse_template(r#"<Panel count={$count}>Hello {$user.name}</Panel>"#)
        .expect("template parses");
    let native = NativeElementRegistry::try_from_specs(Vec::new()).expect("valid native registry");
    let components = ComponentRegistry::try_from_specs(vec![
        ComponentSpec::try_new(
            "Panel",
            vec![
                AttributeSpec::try_new("count", AttributeRule::one(AttributeKind::Expression))
                    .expect("valid attr spec"),
            ],
        )
        .expect("valid component spec"),
    ])
    .expect("valid component registry");
    let validated = validate_template(&document, &native, &components).expect("template validates");

    assert_eq!(
        render_to_rust(&validated),
        r#"::surgeist::template::template(vec![::surgeist::template::component("Panel", vec![::surgeist::template::attr_expr("count", "$count")], vec![::surgeist::template::text("Hello "), ::surgeist::template::expr("$user.name")])])"#
    );
}

#[test]
fn renders_keyword_variable_expression_as_symbolic_string() {
    let document = parse_template(r#"<Panel value={$match} />"#).expect("template parses");
    let native = NativeElementRegistry::try_from_specs(Vec::new()).expect("valid native registry");
    let components = ComponentRegistry::try_from_specs(vec![
        ComponentSpec::try_new(
            "Panel",
            vec![
                AttributeSpec::try_new("value", AttributeRule::one(AttributeKind::Expression))
                    .expect("valid attr spec"),
            ],
        )
        .expect("valid component spec"),
    ])
    .expect("valid component registry");
    let validated = validate_template(&document, &native, &components).expect("template validates");

    assert_eq!(
        render_to_rust(&validated),
        r#"::surgeist::template::template(vec![::surgeist::template::component("Panel", vec![::surgeist::template::attr_expr("value", "$match")], vec![])])"#
    );
}
