# surgeist-template

Strict template parsing, validation, and rendering contracts for Surgeist.

This crate owns the template layer for Surgeist. It parses authored template
source into typed IR, validates that IR against explicit element registries, and
emits the initial Rust template construction surface. Host integration and
cross-crate lowering remain owned by the root `surgeist` facade.

## V1 Template Surface

V1 is intentionally strict. Templates use HTML-like element syntax with exact
open/close matching, self-closing elements, text nodes, and `{* ... *}`
comments. The parser does not perform browser-style recovery.

Element names are typed by shape. Lowercase names such as `<div>` are native
elements. UpperCamelCase names such as `<Panel>` are components. Both must be
registered before validation succeeds.

Text may contain interpolation with `{$expr}`. Quoted attributes may also
contain interpolated parts, while unquoted brace attributes such as
`enabled={$is_enabled}` are expression attributes. Bare attributes are boolean
attributes, and ordinary attribute values are static strings.

Supported control flow is limited to:

```text
{if $visible}...{elseif $fallback}...{else}...{/if}
{foreach $items as $item}...{foreachelse}...{/foreach}
```

Expressions are a compact symbolic subset: scalar literals, variable paths such
as `$user.name` and `$items[0]`, parentheses, unary `!` and `-`, arithmetic,
comparisons, `&&`, and `||`. Function calls, method calls, mutation, includes,
inheritance, globals, and arbitrary template blocks inside start tags are not
part of V1.

Validation is registry-driven. `NativeElementRegistry` and `ComponentRegistry`
declare known element names, allowed attributes, and accepted attribute value
kinds (`Bool`, `Static`, `Expression`, or `Interpolated`). Validation returns a
`ValidatedTemplate` that preserves the typed node structure.

Rendering is currently a code-generation contract, not a runtime evaluator.
`render_to_rust` emits a structured Rust expression string for a validated
template using symbolic expression strings in the generated calls.

```rust
use surgeist_template::{
    AttributeKind, AttributeRule, AttributeSpec, ComponentRegistry, ComponentSpec,
    NativeElementRegistry, NativeElementSpec, parse_template, render_to_rust,
    validate_template,
};

let document = parse_template(
    r#"<Panel title="Hello {$user.name}" visible={$visible}><div>{$message}</div></Panel>"#,
)
.expect("template parses");

let native = NativeElementRegistry::try_from_specs(vec![
    NativeElementSpec::try_new("div", Vec::new()).expect("native spec"),
])
.expect("native registry");

let components = ComponentRegistry::try_from_specs(vec![
    ComponentSpec::try_new(
        "Panel",
        vec![
            AttributeSpec::try_new(
                "title",
                AttributeRule::any(
                    AttributeKind::Static,
                    [AttributeKind::Interpolated],
                ),
            )
            .expect("title attr"),
            AttributeSpec::try_new("visible", AttributeRule::one(AttributeKind::Expression))
                .expect("visible attr"),
        ],
    )
    .expect("component spec"),
])
.expect("component registry");

let validated = validate_template(&document, &native, &components).expect("template validates");
let rust_source = render_to_rust(&validated);

assert!(rust_source.contains(r#"::surgeist::template::component("Panel""#));
```

## Baseline Checks

Run these before handing off crate-local template work:

```sh
cargo test -p surgeist-template
cargo clippy -p surgeist-template --all-targets -- -D warnings
cargo fmt --check
```
