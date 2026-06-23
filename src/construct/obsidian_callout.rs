//! Obsidian: callout detection helper.
//!
//! Callouts are syntactically blockquotes with a `[!type]` marker on the first
//! line. This module provides a shared helper that detects the callout shape
//! from the blockquote's text content. It is used by both [`to_mdast`][crate::to_mdast]
//! and [`to_html`][crate::to_html] compilers.
//!
//! ## Grammar
//!
//! ```bnf
//! obsidian_callout_marker ::= '[' '!' type ']' [+-]? [title]?
//! type ::= 1*ascii_alphabetic
//! ```
//!
//! ## Examples
//!
//! ```markdown
//! > [!note] Title
//! > Body text.
//! ```
//!
//! ## References
//!
//! * [Obsidian Flavored Markdown](https://obsidian.md/help/obsidian-flavored-markdown)
//! * [Callouts](https://obsidian.md/help/callouts)

use alloc::string::{String, ToString};

/// Info extracted from a callout marker.
#[derive(Debug, Eq, PartialEq)]
pub struct ObsidianCalloutInfo {
    /// Callout type identifier (e.g. `note`, `tip`, `warning`).
    pub callout_type: String,
    /// Foldable state: `Some(true)` for `+`, `Some(false)` for `-`, `None` if
    /// not foldable.
    pub foldable: Option<bool>,
    /// Optional title text (may be empty string if no title).
    pub title: Option<String>,
}

/// Detect a callout marker in the first line of a blockquote.
///
/// `first_line` is the text content of the first line of the blockquote,
/// after stripping the `>` prefix and optional space. Returns `Some(info)` if
/// the line matches the callout marker pattern, `None` otherwise.
///
/// The expected format is: `[!type][+-]?[ title]?`
pub fn detect(first_line: &str) -> Option<ObsidianCalloutInfo> {
    let trimmed = first_line.trim_start();
    let bytes = trimmed.as_bytes();

    // Must start with `[!`.
    if bytes.len() < 3 || bytes[0] != b'[' || bytes[1] != b'!' {
        return None;
    }

    // Find closing `]`.
    let mut i = 2;
    while i < bytes.len() && bytes[i] != b']' {
        if !bytes[i].is_ascii_alphabetic() {
            return None;
        }
        i += 1;
    }

    // Must have at least one alpha char for type, and a closing `]`.
    if i == 2 || i >= bytes.len() {
        return None;
    }

    let callout_type = String::from_utf8(bytes[2..i].to_vec()).ok()?;
    i += 1; // Skip `]`

    // Optional `+` or `-` for foldable.
    let mut foldable = None;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        foldable = Some(bytes[i] == b'+');
        i += 1;
    }

    // Optional title: everything after optional whitespace, on the same line.
    let rest = &trimmed[i..];
    // Only consider the first line for the title (stop at newline).
    let rest = rest.split('\n').next().unwrap_or("");
    let title = if rest.is_empty() || rest.trim().is_empty() {
        None
    } else {
        Some(rest.trim().to_string())
    };

    Some(ObsidianCalloutInfo {
        callout_type,
        foldable,
        title,
    })
}
