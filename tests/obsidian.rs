use markdown::{
    mdast::Node, message, to_html, to_html_with_options, to_mdast, Options, ParseOptions,
};
use pretty_assertions::assert_eq;

fn obsidian_opts() -> Options {
    Options {
        parse: ParseOptions::obsidian(),
        ..Options::default()
    }
}

// ---------------------------------------------------------------------------
// HTML tests
// ---------------------------------------------------------------------------

#[test]
fn obsidian_wikilink_basic() -> Result<(), message::Message> {
    let html = to_html_with_options("[[Note]]", &obsidian_opts())?;
    assert_eq!(html, "<p><a href=\"Note\">Note</a></p>");
    Ok(())
}

#[test]
fn obsidian_wikilink_heading() -> Result<(), message::Message> {
    let html = to_html_with_options("[[Note#Heading]]", &obsidian_opts())?;
    assert_eq!(html, "<p><a href=\"Note#Heading\">Heading</a></p>");
    Ok(())
}

#[test]
fn obsidian_wikilink_block_ref() -> Result<(), message::Message> {
    let html = to_html_with_options("[[Note#^block-id]]", &obsidian_opts())?;
    assert_eq!(html, "<p><a href=\"Note#^block-id\">Note</a></p>");
    Ok(())
}

#[test]
fn obsidian_wikilink_alias() -> Result<(), message::Message> {
    let html = to_html_with_options("[[Note|Alias]]", &obsidian_opts())?;
    assert_eq!(html, "<p><a href=\"Note\">Alias</a></p>");
    Ok(())
}

#[test]
fn obsidian_wikilink_same_file_heading() -> Result<(), message::Message> {
    let html = to_html_with_options("[[#Heading]]", &obsidian_opts())?;
    assert_eq!(html, "<p><a href=\"#Heading\">Heading</a></p>");
    Ok(())
}

#[test]
fn obsidian_embed_basic() -> Result<(), message::Message> {
    let html = to_html_with_options("![[Note]]", &obsidian_opts())?;
    assert_eq!(html, "<p><img src=\"Note\" alt=\"Note\" /></p>");
    Ok(())
}

#[test]
fn obsidian_embed_with_alias() -> Result<(), message::Message> {
    let html = to_html_with_options("![[Note|Display]]", &obsidian_opts())?;
    assert_eq!(html, "<p><img src=\"Note\" alt=\"Display\" /></p>");
    Ok(())
}

#[test]
fn obsidian_comment_basic() -> Result<(), message::Message> {
    let html = to_html_with_options("a %%comment%% b", &obsidian_opts())?;
    assert_eq!(html, "<p>a  b</p>");
    Ok(())
}

#[test]
fn obsidian_highlight_basic() -> Result<(), message::Message> {
    let html = to_html_with_options("a ==b== c", &obsidian_opts())?;
    assert_eq!(html, "<p>a <mark>b</mark> c</p>");
    Ok(())
}

#[test]
fn obsidian_disabled_by_default() {
    // Wikilinks, embeds, comments, and highlights are not CommonMark.
    assert_eq!(to_html("[[Note]]"), "<p>[[Note]]</p>");
    assert_eq!(to_html("![[Note]]"), "<p>![[Note]]</p>");
    assert_eq!(to_html("a %%b%% c"), "<p>a %%b%% c</p>");
    assert_eq!(to_html("a ==b== c"), "<p>a ==b== c</p>");
}

// ---------------------------------------------------------------------------
// mdast tests
// ---------------------------------------------------------------------------

#[test]
fn obsidian_wikilink_mdast() -> Result<(), message::Message> {
    let tree = to_mdast("[[Note]]", &ParseOptions::obsidian())?;
    let node = find_first(&tree, &|n| matches!(n, Node::ObsidianWikilink(_)))
        .expect("expected ObsidianWikilink node");
    if let Node::ObsidianWikilink(w) = node {
        assert_eq!(w.path.as_deref(), Some("Note"));
        assert_eq!(w.heading, None);
        assert_eq!(w.block_id, None);
        assert_eq!(w.alias, None);
    }
    Ok(())
}

#[test]
fn obsidian_wikilink_heading_mdast() -> Result<(), message::Message> {
    let tree = to_mdast("[[Note#Heading]]", &ParseOptions::obsidian())?;
    let node = find_first(&tree, &|n| matches!(n, Node::ObsidianWikilink(_)))
        .expect("expected ObsidianWikilink node");
    if let Node::ObsidianWikilink(w) = node {
        assert_eq!(w.path.as_deref(), Some("Note"));
        assert_eq!(w.heading.as_deref(), Some("Heading"));
        assert_eq!(w.block_id, None);
        assert_eq!(w.alias, None);
    }
    Ok(())
}

#[test]
fn obsidian_wikilink_block_ref_mdast() -> Result<(), message::Message> {
    let tree = to_mdast("[[Note#^block-id]]", &ParseOptions::obsidian())?;
    let node = find_first(&tree, &|n| matches!(n, Node::ObsidianWikilink(_)))
        .expect("expected ObsidianWikilink node");
    if let Node::ObsidianWikilink(w) = node {
        assert_eq!(w.path.as_deref(), Some("Note"));
        assert_eq!(w.heading, None);
        assert_eq!(w.block_id.as_deref(), Some("block-id"));
        assert_eq!(w.alias, None);
    }
    Ok(())
}

#[test]
fn obsidian_wikilink_alias_mdast() -> Result<(), message::Message> {
    let tree = to_mdast("[[Note|Alias]]", &ParseOptions::obsidian())?;
    let node = find_first(&tree, &|n| matches!(n, Node::ObsidianWikilink(_)))
        .expect("expected ObsidianWikilink node");
    if let Node::ObsidianWikilink(w) = node {
        assert_eq!(w.path.as_deref(), Some("Note"));
        assert_eq!(w.heading, None);
        assert_eq!(w.block_id, None);
        assert_eq!(w.alias.as_deref(), Some("Alias"));
    }
    Ok(())
}

#[test]
fn obsidian_embed_mdast() -> Result<(), message::Message> {
    let tree = to_mdast("![[Note]]", &ParseOptions::obsidian())?;
    let node = find_first(&tree, &|n| matches!(n, Node::ObsidianEmbed(_)))
        .expect("expected ObsidianEmbed node");
    if let Node::ObsidianEmbed(e) = node {
        assert_eq!(e.path.as_deref(), Some("Note"));
        assert_eq!(e.heading, None);
        assert_eq!(e.block_id, None);
        assert_eq!(e.alias, None);
    }
    Ok(())
}

#[test]
fn obsidian_comment_mdast() -> Result<(), message::Message> {
    let tree = to_mdast("a %%comment%% b", &ParseOptions::obsidian())?;
    let node = find_first(&tree, &|n| matches!(n, Node::ObsidianComment(_)))
        .expect("expected ObsidianComment node");
    if let Node::ObsidianComment(c) = node {
        assert_eq!(c.value, "comment");
    }
    Ok(())
}

#[test]
fn obsidian_highlight_mdast() -> Result<(), message::Message> {
    let tree = to_mdast("a ==b== c", &ParseOptions::obsidian())?;
    let node = find_first(&tree, &|n| matches!(n, Node::ObsidianHighlight(_)))
        .expect("expected ObsidianHighlight node");
    if let Node::ObsidianHighlight(h) = node {
        assert_eq!(h.children.len(), 1);
        if let Node::Text(t) = &h.children[0] {
            assert_eq!(t.value, "b");
        } else {
            panic!("expected Text child inside highlight");
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Block id tests
// ---------------------------------------------------------------------------

#[test]
fn obsidian_block_id_html() -> Result<(), message::Message> {
    // Block ids are metadata — they don't render to HTML.
    let html = to_html_with_options("Hello world ^abc-def\n", &obsidian_opts())?;
    assert_eq!(html, "<p>Hello world </p>\n");
    Ok(())
}

#[test]
fn obsidian_block_id_mdast() -> Result<(), message::Message> {
    let tree = to_mdast("Hello world ^abc-def\n", &ParseOptions::obsidian())?;
    let node = find_first(&tree, &|n| matches!(n, Node::ObsidianBlockId(_)))
        .expect("expected ObsidianBlockId node");
    if let Node::ObsidianBlockId(block_id) = node {
        assert_eq!(block_id.id, "abc-def");
    }
    Ok(())
}

#[test]
fn obsidian_block_id_heading_mdast() -> Result<(), message::Message> {
    let tree = to_mdast("# Heading ^my-id\n", &ParseOptions::obsidian())?;
    let node = find_first(&tree, &|n| matches!(n, Node::ObsidianBlockId(_)))
        .expect("expected ObsidianBlockId node");
    if let Node::ObsidianBlockId(block_id) = node {
        assert_eq!(block_id.id, "my-id");
    }
    Ok(())
}

#[test]
fn obsidian_block_id_disabled_by_default() {
    // Without obsidian enabled, ^abc is just text.
    let html = to_html("Hello ^abc\n");
    assert_eq!(html, "<p>Hello ^abc</p>\n");
}

// ---------------------------------------------------------------------------
// Callout tests
// ---------------------------------------------------------------------------

#[test]
fn obsidian_callout_html() -> Result<(), message::Message> {
    let html = to_html_with_options("> [!note] My Title\n> Body text here\n", &obsidian_opts())?;
    assert_eq!(
        html,
        "<div class=\"callout\" data-callout=\"note\">\
<div class=\"callout-title\"><div class=\"callout-title-inner\">My Title</div></div>\
<p>\nBody text here</p>\n</div>\n"
    );
    Ok(())
}

#[test]
fn obsidian_callout_no_title_html() -> Result<(), message::Message> {
    let html = to_html_with_options("> [!warning]\n> Body text\n", &obsidian_opts())?;
    assert_eq!(
        html,
        "<div class=\"callout\" data-callout=\"warning\">\
<div class=\"callout-title\"><div class=\"callout-title-inner\">warning</div></div>\
<p>\nBody text</p>\n</div>\n"
    );
    Ok(())
}

#[test]
fn obsidian_callout_foldable_html() -> Result<(), message::Message> {
    let html = to_html_with_options("> [!note]+ Title\n> Body\n", &obsidian_opts())?;
    assert!(html.contains("is-collapsible is-expanded"));
    Ok(())
}

#[test]
fn obsidian_callout_collapsed_html() -> Result<(), message::Message> {
    let html = to_html_with_options("> [!note]- Title\n> Body\n", &obsidian_opts())?;
    assert!(html.contains("is-collapsible is-collapsed"));
    Ok(())
}

#[test]
fn obsidian_callout_mdast() -> Result<(), message::Message> {
    let tree = to_mdast(
        "> [!note] My Title\n> Body text\n",
        &ParseOptions::obsidian(),
    )?;
    if let Node::Root(root) = &tree {
        if let Some(Node::ObsidianCallout(callout)) = root.children.first() {
            assert_eq!(callout.callout_type, "note");
            assert_eq!(callout.title.as_deref(), Some("My Title"));
            assert_eq!(callout.foldable, None);
            // Children should contain the body text paragraph.
            assert!(!callout.children.is_empty());
            return Ok(());
        }
    }
    panic!("expected ObsidianCallout node");
}

#[test]
fn obsidian_callout_no_title_mdast() -> Result<(), message::Message> {
    let tree = to_mdast("> [!tip]\n> Tip body\n", &ParseOptions::obsidian())?;
    if let Node::Root(root) = &tree {
        if let Some(Node::ObsidianCallout(callout)) = root.children.first() {
            assert_eq!(callout.callout_type, "tip");
            assert_eq!(callout.title, None);
            return Ok(());
        }
    }
    panic!("expected ObsidianCallout node");
}

#[test]
fn obsidian_callout_foldable_mdast() -> Result<(), message::Message> {
    let tree = to_mdast("> [!note]+ Title\n> Body\n", &ParseOptions::obsidian())?;
    if let Node::Root(root) = &tree {
        if let Some(Node::ObsidianCallout(callout)) = root.children.first() {
            assert_eq!(callout.foldable, Some(true));
            return Ok(());
        }
    }
    panic!("expected ObsidianCallout node");
}

#[test]
fn obsidian_callout_not_a_callout() -> Result<(), message::Message> {
    // Regular blockquote should not be converted to a callout.
    let tree = to_mdast("> Just a regular quote\n", &ParseOptions::obsidian())?;
    if let Node::Root(root) = &tree {
        if let Some(Node::Blockquote(_)) = root.children.first() {
            return Ok(());
        }
    }
    panic!("expected Blockquote, not ObsidianCallout");
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Depth-first search for the first node matching a predicate.
fn find_first<'a, F: Fn(&Node) -> bool>(node: &'a Node, pred: &'a F) -> Option<&'a Node> {
    if pred(node) {
        return Some(node);
    }
    if let Some(children) = node.children() {
        for child in children {
            if let Some(found) = find_first(child, pred) {
                return Some(found);
            }
        }
    }
    None
}
