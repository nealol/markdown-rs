//! Tests for the CodeHike-style decorated blocks mdast transform.
//!
//! See [`markdown::Constructs::code_hike_blocks`].

use markdown::{
    mdast::{CodeHikeBlock, CodeHikeCode, CodeHikeImage, CodeHikeText, Node},
    to_mdast, Constructs, ParseOptions,
};
mod test_utils;

fn code_hike_opts() -> ParseOptions {
    ParseOptions::code_hike()
}

/// Return the root’s children, panicking if the tree is not a `Root`.
fn root_children(node: &Node) -> &Vec<Node> {
    match node {
        Node::Root(r) => &r.children,
        _ => panic!("expected Root, got {:?}", node),
    }
}

#[test]
fn disabled_by_default() {
    let tree = to_mdast("## !mordor Barad-dur\nText", &ParseOptions::default()).unwrap();
    let children = root_children(&tree);
    assert!(
        matches!(children.first(), Some(Node::Heading(_))),
        "should leave normal heading when disabled, got {:?}",
        children.first()
    );
}

#[test]
fn heading_block_basic() {
    let tree = to_mdast("## !mordor Barad-dur\nThe Dark Tower", &code_hike_opts()).unwrap();
    let children = root_children(&tree);
    assert_eq!(children.len(), 1, "should produce a single block");

    match &children[0] {
        Node::CodeHikeBlock(CodeHikeBlock {
            name,
            list,
            title,
            depth,
            children,
            ..
        }) => {
            assert_eq!(name, "mordor");
            assert!(!*list);
            assert_eq!(title, "Barad-dur");
            assert_eq!(*depth, 2);
            assert_eq!(children.len(), 1, "should collect one paragraph");
            assert!(
                matches!(children.first(), Some(Node::Paragraph(_))),
                "child should be a paragraph"
            );
        }
        other => panic!("expected CodeHikeBlock, got {:?}", other),
    }
}

#[test]
fn heading_block_title_uses_full_heading_text() {
    let tree = to_mdast("## !mordor Barad-*dur*", &code_hike_opts()).unwrap();
    let children = root_children(&tree);

    match &children[0] {
        Node::CodeHikeBlock(CodeHikeBlock { title, .. }) => {
            assert_eq!(title, "Barad-dur");
        }
        other => panic!("expected CodeHikeBlock, got {:?}", other),
    }
}

#[test]
fn heading_block_stops_at_same_or_higher_heading() {
    let tree = to_mdast("## !a A\nx\n## Normal\ny", &code_hike_opts()).unwrap();
    let children = root_children(&tree);
    assert_eq!(
        children.len(),
        3,
        "should produce block, heading, paragraph; got {:?}",
        children
    );
    assert!(matches!(children[0], Node::CodeHikeBlock(_)));
    assert!(matches!(children[1], Node::Heading(_)));
    assert!(matches!(children[2], Node::Paragraph(_)));
}

#[test]
fn nested_headings() {
    let tree = to_mdast(
        "## !master\nThe One Ring\n\n### !!rings Elves\nThree rings",
        &code_hike_opts(),
    )
    .unwrap();
    let children = root_children(&tree);
    assert_eq!(children.len(), 1, "should produce a single top-level block");

    let outer = match &children[0] {
        Node::CodeHikeBlock(b) => b,
        other => panic!("expected outer CodeHikeBlock, got {:?}", other),
    };
    assert_eq!(outer.name, "master");
    assert!(!outer.list);
    assert_eq!(outer.depth, 2);
    // First child is the paragraph, second is the nested block.
    assert_eq!(outer.children.len(), 2);

    let inner = match &outer.children[1] {
        Node::CodeHikeBlock(b) => b,
        other => panic!("expected nested CodeHikeBlock, got {:?}", other),
    };
    assert_eq!(inner.name, "rings");
    assert!(inner.list, "nested block should be a list block");
    assert_eq!(inner.title, "Elves");
    assert_eq!(inner.depth, 3);
    assert_eq!(inner.children.len(), 1);
}

#[test]
fn paragraph_metadata() {
    let tree = to_mdast("!author Tolkien", &code_hike_opts()).unwrap();
    let children = root_children(&tree);
    assert_eq!(children.len(), 1);

    match &children[0] {
        Node::CodeHikeText(CodeHikeText {
            name, list, value, ..
        }) => {
            assert_eq!(name, "author");
            assert!(!*list);
            assert_eq!(value, "Tolkien");
        }
        other => panic!("expected CodeHikeText, got {:?}", other),
    }
}

#[test]
fn image_metadata() {
    let tree = to_mdast(
        "![!cover Gandalf](/gandalf.jpg \"a wizard\")",
        &code_hike_opts(),
    )
    .unwrap();
    let children = root_children(&tree);
    assert_eq!(children.len(), 1);

    match &children[0] {
        Node::CodeHikeImage(CodeHikeImage {
            name,
            list,
            alt,
            url,
            title,
            ..
        }) => {
            assert_eq!(name, "cover");
            assert!(!*list);
            assert_eq!(alt, "Gandalf");
            assert_eq!(url, "/gandalf.jpg");
            assert_eq!(title.as_deref(), Some("a wizard"));
        }
        other => panic!("expected CodeHikeImage, got {:?}", other),
    }
}

#[test]
fn image_metadata_list_form() {
    let tree = to_mdast("![!!cover Gandalf](/gandalf.jpg)", &code_hike_opts()).unwrap();
    let children = root_children(&tree);
    assert_eq!(children.len(), 1);

    match &children[0] {
        Node::CodeHikeImage(CodeHikeImage { name, list, .. }) => {
            assert_eq!(name, "cover");
            assert!(*list);
        }
        other => panic!("expected CodeHikeImage, got {:?}", other),
    }
}

#[test]
fn code_metadata() {
    let tree = to_mdast(
        "```js !riddle mellon.js\nspeak(\"friend\")\n```",
        &code_hike_opts(),
    )
    .unwrap();
    let children = root_children(&tree);
    assert_eq!(children.len(), 1);

    match &children[0] {
        Node::CodeHikeCode(CodeHikeCode {
            name,
            list,
            lang,
            meta,
            value,
            ..
        }) => {
            assert_eq!(name, "riddle");
            assert!(!*list);
            assert_eq!(lang.as_deref(), Some("js"));
            assert_eq!(meta.as_deref(), Some("mellon.js"));
            assert_eq!(value, "speak(\"friend\")");
        }
        other => panic!("expected CodeHikeCode, got {:?}", other),
    }
}

#[test]
fn code_metadata_no_rest() {
    let tree = to_mdast("```js !riddle\nspeak()\n```", &code_hike_opts()).unwrap();
    let children = root_children(&tree);
    assert_eq!(children.len(), 1);

    match &children[0] {
        Node::CodeHikeCode(CodeHikeCode { name, meta, .. }) => {
            assert_eq!(name, "riddle");
            assert!(meta.is_none(), "meta should be None when rest is empty");
        }
        other => panic!("expected CodeHikeCode, got {:?}", other),
    }
}

#[test]
fn root_level_list_syntax() {
    let tree = to_mdast("## !!breakfasts first", &code_hike_opts()).unwrap();
    let children = root_children(&tree);
    assert_eq!(children.len(), 1);

    match &children[0] {
        Node::CodeHikeBlock(CodeHikeBlock {
            name, list, title, ..
        }) => {
            assert_eq!(name, "breakfasts");
            assert!(*list, "should be a list block");
            assert_eq!(title, "first");
        }
        other => panic!("expected CodeHikeBlock, got {:?}", other),
    }
}

#[test]
fn mdx_component_children() {
    let options = ParseOptions {
        constructs: Constructs {
            code_hike_blocks: true,
            ..Constructs::mdx()
        },
        mdx_esm_parse: Some(Box::new(test_utils::swc::parse_esm)),
        mdx_expression_parse: Some(Box::new(test_utils::swc::parse_expression)),
        ..ParseOptions::default()
    };

    let tree = to_mdast(
        "<MyComponent>\n\n## !mordor Barad-dur\nThe Dark Tower\n\n</MyComponent>",
        &options,
    )
    .unwrap();
    let children = root_children(&tree);
    assert_eq!(children.len(), 1, "should produce a single JSX element");

    let jsx = match &children[0] {
        Node::MdxJsxFlowElement(e) => e,
        other => panic!("expected MdxJsxFlowElement, got {:?}", other),
    };
    assert_eq!(jsx.name.as_deref(), Some("MyComponent"));
    assert!(
        jsx.children
            .iter()
            .any(|c| matches!(c, Node::CodeHikeBlock(_))),
        "JSX children should contain a CodeHikeBlock, got {:?}",
        jsx.children
    );
}

#[test]
fn invalid_decorations_remain_markdown() {
    let cases = ["! invalid", "!!", "## ! invalid"];

    for input in cases {
        let tree = to_mdast(input, &code_hike_opts()).unwrap();
        let children = root_children(&tree);
        for child in children {
            assert!(
                !matches!(
                    child,
                    Node::CodeHikeBlock(_)
                        | Node::CodeHikeText(_)
                        | Node::CodeHikeImage(_)
                        | Node::CodeHikeCode(_)
                ),
                "invalid decoration `{}` should not produce a CodeHike node, got {:?}",
                input,
                child
            );
        }
    }
}

#[test]
fn html_output_unaffected() {
    use markdown::{to_html_with_options, Options};

    let input = "## !mordor Barad-dur\nThe Dark Tower";
    let html_default = to_html_with_options(input, &Options::default()).unwrap();
    let html_code_hike = to_html_with_options(input, &Options::code_hike()).unwrap();
    assert_eq!(
        html_default, html_code_hike,
        "to_html should be unaffected by code_hike_blocks"
    );
}
