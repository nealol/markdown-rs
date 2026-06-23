//! `CodeHike`-style decorated blocks: mdast structural transform.
//!
//! This module is a private post-processing pass run after the normal mdast
//! is built (see [`crate::to_mdast`]). It does **not** change `CommonMark`
//! tokenization, codeblock parsing, or annotation parsing. It only rewrites
//! the mdast tree when [`Constructs::code_hike_blocks`][crate::Constructs::code_hike_blocks]
//! is enabled.
//!
//! See the module-level docs in [`crate`] for the configuration flag.
//!
//! ## Decoration syntax
//!
//! * `!name` — single named block/value.
//! * `!!name` — repeatable/list block/value.
//!
//! A decoration appears at the start of a heading’s first text child, a
//! paragraph’s first text child, an image’s `alt`, or a fenced code block’s
//! `meta`. The decoration is stripped and the remaining text becomes the
//! block’s `title`/`value`/`alt`/`meta`.
//!
//! ## HTML
//!
//! This transform is mdast-only. `to_html` / `to_html_with_options` compile
//! directly from parser events and are unaffected.

use crate::mdast::{CodeHikeBlock, CodeHikeCode, CodeHikeImage, CodeHikeText, Node};
use crate::unist::Position;
use alloc::string::{String, ToString};
use alloc::{string, vec::Vec};

/// Parsed `CodeHike` decoration prefix.
#[derive(Clone, Debug, Eq, PartialEq)]
struct Decoration {
    /// Decoration name (e.g. `author`, `breakfasts`).
    name: String,
    /// Whether the `!!name` (repeatable/list) form was used.
    list: bool,
    /// The text after the name and the whitespace separating it from the
    /// name. Leading whitespace between name and rest is trimmed; meaningful
    /// content after that is preserved verbatim.
    rest: String,
    /// Number of bytes consumed by the decoration prefix (including the
    /// leading `!`/`!!` and the trimmed whitespace separator, but not
    /// including the `rest` content).
    consumed: usize,
}

/// Parse a `CodeHike` decoration prefix from `s`.
///
/// Returns `Some(Decoration)` iff `s` starts with `!` or `!!`, followed by
/// a valid name (first char ASCII letter or `_`, rest ASCII alphanumeric,
/// `_`, or `-`), followed by EOF or whitespace.
///
/// Examples:
/// * `!author Tolkien` -> `{ name: "author", list: false, rest: "Tolkien" }`
/// * `!!breakfasts first` -> `{ name: "breakfasts", list: true, rest: "first" }`
/// * `! invalid` -> no decoration (name must start with letter/`_`)
/// * `!` -> no decoration
/// * `!!` -> no decoration
fn parse_decoration(s: &str) -> Option<Decoration> {
    let bytes = s.as_bytes();
    if bytes.first() != Some(&b'!') {
        return None;
    }

    let (list, after_bangs) = if bytes.get(1) == Some(&b'!') {
        (true, &bytes[2..])
    } else {
        (false, &bytes[1..])
    };

    // Name: first char ASCII letter or `_`.
    let first = after_bangs.first()?;
    if !first.is_ascii_alphabetic() && *first != b'_' {
        return None;
    }

    // Rest of name: ASCII alphanumeric, `_`, or `-`.
    let mut i = 1;
    while i < after_bangs.len() {
        let c = after_bangs[i];
        if c.is_ascii_alphanumeric() || c == b'_' || c == b'-' {
            i += 1;
        } else {
            break;
        }
    }

    // After the name there must be EOF or whitespace. If a non-whitespace
    // character immediately follows the name, this isn’t a valid decoration
    // (e.g. `!foo-bar` is fine, but `!foo!bar` is not a decoration).
    if i < after_bangs.len() && !after_bangs[i].is_ascii_whitespace() {
        return None;
    }

    let name = string::String::from_utf8(after_bangs[..i].to_vec()).ok()?;
    let after_name = &after_bangs[i..];

    // Trim only the leading whitespace between name and rest. Preserve
    // meaningful rest content after that.
    let mut sep = 0;
    while sep < after_name.len() && after_name[sep].is_ascii_whitespace() {
        sep += 1;
    }
    let rest = string::String::from_utf8(after_name[sep..].to_vec()).ok()?;

    // Bytes consumed: leading `!`/`!!` + name + the whitespace separator we
    // trimmed. Computed as a byte offset into the original `s`.
    let consumed = s.len() - rest.len();

    Some(Decoration {
        name,
        list,
        rest,
        consumed,
    })
}

/// Recursively transform `node` and its descendants.
pub fn transform(node: &mut Node) {
    // Recurse first into existing children so that nested structures (e.g.
    // block quotes, lists, MDX JSX elements) get their children transformed.
    // Then transform this node’s own children vector, which may rewrite
    // headings into grouping `CodeHikeBlock`s.
    if let Some(children) = node.children_mut() {
        // Recurse into each child first so deeper content is normalized
        // before we group at this level.
        let mut borrowed: Vec<Node> = children.clone();
        for child in &mut borrowed {
            transform(child);
        }
        // Now run the grouping/leaf transform over this level.
        transform_children(&mut borrowed);
        *children = borrowed;
    }
}

/// Transform a flat vector of sibling nodes.
///
/// Decorated headings group following siblings; decorated leaves are
/// one-for-one replacements.
fn transform_children(children: &mut Vec<Node>) {
    let source = children.clone();
    let mut out: Vec<Node> = Vec::with_capacity(source.len());
    let mut i = 0;

    while i < source.len() {
        let current = &source[i];

        if let Some((block, consumed)) = try_heading_block(current, &source[i + 1..]) {
            out.push(block);
            i += consumed + 1;
            continue;
        }

        if let Some(leaf) = try_image_leaf(current) {
            out.push(leaf);
            i += 1;
            continue;
        }

        if let Some(leaf) = try_code_leaf(current) {
            out.push(leaf);
            i += 1;
            continue;
        }

        if let Some(leaf) = try_paragraph_leaf(current) {
            out.push(leaf);
            i += 1;
            continue;
        }

        out.push(current.clone());
        i += 1;
    }

    *children = out;
}

/// If `node` is a decorated heading, build a `CodeHikeBlock` and collect
/// following siblings until a heading with `depth <= current_depth`.
///
/// Returns `(block, number_of_following_siblings_consumed)`.
fn try_heading_block(node: &Node, rest: &[Node]) -> Option<(Node, usize)> {
    let heading = match node {
        Node::Heading(h) => h,
        _ => return None,
    };

    let first_text = heading.children.first().and_then(|c| match c {
        Node::Text(t) => Some(t),
        _ => None,
    })?;

    let deco = parse_decoration(&first_text.value)?;
    let title = heading_title(heading, deco.consumed);

    let depth = heading.depth;

    // Collect following siblings until a heading with depth <= current depth.
    let mut consumed = 0;
    let mut collected: Vec<Node> = Vec::new();
    for sibling in rest {
        if let Node::Heading(h) = sibling {
            if h.depth <= depth {
                break;
            }
        }
        collected.push(sibling.clone());
        consumed += 1;
    }

    // Recursively transform collected children so deeper decorated headings
    // become nested `CodeHikeBlock`s.
    transform_children(&mut collected);

    // Compute position: start from heading position start; end at last
    // collected child position end if any, else heading position end.
    let position = match (heading.position.as_ref(), collected.last()) {
        (Some(start_pos), Some(last)) => last.position().map(|end_pos| Position {
            start: start_pos.start.clone(),
            end: end_pos.end.clone(),
        }),
        (Some(start_pos), None) => Some(Position {
            start: start_pos.start.clone(),
            end: start_pos.end.clone(),
        }),
        _ => None,
    };

    let block = CodeHikeBlock {
        children: collected,
        position,
        name: deco.name,
        title,
        list: deco.list,
        depth,
    };

    Some((Node::CodeHikeBlock(block), consumed))
}

/// Build a decorated heading title from the full heading text, stripping the
/// decoration prefix from the first text child.
fn heading_title(heading: &crate::mdast::Heading, consumed: usize) -> String {
    let mut title = String::new();

    for (idx, child) in heading.children.iter().enumerate() {
        if idx == 0 {
            if let Node::Text(t) = child {
                title.push_str(&t.value[consumed..]);
            } else {
                title.push_str(&child.to_string());
            }
        } else {
            title.push_str(&child.to_string());
        }
    }

    title
}

/// If `node` is a paragraph containing exactly one `Image` whose `alt`
/// starts with a decoration, build a `CodeHikeImage`.
fn try_image_leaf(node: &Node) -> Option<Node> {
    let paragraph = match node {
        Node::Paragraph(p) => p,
        _ => return None,
    };

    if paragraph.children.len() != 1 {
        return None;
    }

    let image = match &paragraph.children[0] {
        Node::Image(img) => img,
        _ => return None,
    };

    let deco = parse_decoration(&image.alt)?;

    let position = paragraph
        .position
        .clone()
        .or_else(|| image.position.clone());

    Some(Node::CodeHikeImage(CodeHikeImage {
        position,
        name: deco.name,
        list: deco.list,
        alt: deco.rest,
        url: image.url.clone(),
        title: image.title.clone(),
    }))
}

/// If `node` is a `Code` whose `meta` starts with a decoration, build a
/// `CodeHikeCode`.
fn try_code_leaf(node: &Node) -> Option<Node> {
    let code = match node {
        Node::Code(c) => c,
        _ => return None,
    };

    let meta = code.meta.as_ref()?;
    let deco = parse_decoration(meta)?;

    let new_meta = if deco.rest.is_empty() {
        None
    } else {
        Some(deco.rest)
    };

    Some(Node::CodeHikeCode(CodeHikeCode {
        value: code.value.clone(),
        position: code.position.clone(),
        name: deco.name,
        list: deco.list,
        lang: code.lang.clone(),
        meta: new_meta,
    }))
}

/// If `node` is a paragraph whose first child is `Text` starting with a
/// decoration, build a `CodeHikeText`.
fn try_paragraph_leaf(node: &Node) -> Option<Node> {
    let paragraph = match node {
        Node::Paragraph(p) => p,
        _ => return None,
    };

    // Avoid transforming a paragraph that is actually a decorated image
    // paragraph; the image rule runs first, but be defensive.
    if paragraph.children.len() == 1 && matches!(paragraph.children[0], Node::Image(_)) {
        return None;
    }

    let first_text = paragraph.children.first().and_then(|c| match c {
        Node::Text(t) => Some(t),
        _ => None,
    })?;

    let deco = parse_decoration(&first_text.value)?;

    // Build the value from the paragraph’s full text content, with the
    // decoration prefix stripped from the first text node.
    let mut value = String::new();
    for (idx, child) in paragraph.children.iter().enumerate() {
        if idx == 0 {
            if let Node::Text(t) = child {
                value.push_str(&t.value[deco.consumed..]);
            } else {
                value.push_str(&child.to_string());
            }
        } else {
            value.push_str(&child.to_string());
        }
    }

    Some(Node::CodeHikeText(CodeHikeText {
        value,
        position: paragraph.position.clone(),
        name: deco.name,
        list: deco.list,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_decoration_basic() {
        let d = parse_decoration("!author Tolkien").unwrap();
        assert_eq!(d.name, "author");
        assert!(!d.list);
        assert_eq!(d.rest, "Tolkien");
    }

    #[test]
    fn parse_decoration_list() {
        let d = parse_decoration("!!breakfasts first").unwrap();
        assert_eq!(d.name, "breakfasts");
        assert!(d.list);
        assert_eq!(d.rest, "first");
    }

    #[test]
    fn parse_decoration_no_rest() {
        let d = parse_decoration("!author").unwrap();
        assert_eq!(d.name, "author");
        assert_eq!(d.rest, "");
    }

    #[test]
    fn parse_decoration_invalid() {
        assert!(parse_decoration("! invalid").is_none());
        assert!(parse_decoration("!").is_none());
        assert!(parse_decoration("!!").is_none());
        assert!(parse_decoration("author").is_none());
        assert!(parse_decoration("!1abc").is_none());
        assert!(parse_decoration("!foo!bar").is_none());
    }

    #[test]
    fn parse_decoration_name_chars() {
        let d = parse_decoration("!_under_score-1").unwrap();
        assert_eq!(d.name, "_under_score-1");
    }
}
