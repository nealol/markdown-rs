//! Obsidian: block id occurs at the end of a block in the [text][] content
//! type.
//!
//! ## Grammar
//!
//! ```bnf
//! obsidian_block_id ::= '^' block_id
//! block_id ::= 1*( ascii_alphanumeric | '-' | '_' )
//! ```
//!
//! A block id is a `^` followed by an alphanumeric identifier (with `-` and
//! `_` allowed). It appears at the end of a block (paragraph, heading, list
//! item, etc.) after optional whitespace, and makes that block referenceable
//! via `[[note#^id]]`.
//!
//! ## Examples
//!
//! ```markdown
//! > | This is a paragraph.
//! > | ^abc-def
//!     ^^^^^^^^
//! ```
//!
//! ## Extension
//!
//! > 👉 **Note**: block ids are not part of `CommonMark` or `GFM`. Enable them
//! > with `constructs.obsidian_block_id` or use [`Constructs::obsidian()`].
//!
//! ## Tokens
//!
//! * [`ObsidianBlockId`][Name::ObsidianBlockId]
//! * [`ObsidianBlockIdMarker`][Name::ObsidianBlockIdMarker]
//! * [`ObsidianBlockIdValue`][Name::ObsidianBlockIdValue]
//!
//! ## References
//!
//! * [Obsidian Flavored Markdown](https://obsidian.md/help/obsidian-flavored-markdown)
//! * [Block references](https://help.obsidian.md/Linking+notes+and+files/Internal+links#Link+to+a+block+in+a+note)
//!
//! [text]: crate::construct::text

use crate::event::Name;
use crate::state::{Name as StateName, State};
use crate::tokenizer::Tokenizer;

/// Start of block id, at `^`.
///
/// ```markdown
/// > | ^abc
///     ^
/// ```
pub fn start(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.parse_state.options.constructs.obsidian_block_id && tokenizer.current == Some(b'^')
    {
        tokenizer.enter(Name::ObsidianBlockId);
        tokenizer.enter(Name::ObsidianBlockIdMarker);
        tokenizer.consume();
        tokenizer.exit(Name::ObsidianBlockIdMarker);
        State::Next(StateName::ObsidianBlockIdValue)
    } else {
        State::Nok
    }
}

/// In block id value.
///
/// ```markdown
/// > | ^abc-def
///      ^^^^^^^
/// ```
pub fn value(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(c) if c.is_ascii_alphanumeric() || c == b'-' || c == b'_' => {
            tokenizer.enter(Name::ObsidianBlockIdValue);
            tokenizer.consume();
            State::Next(StateName::ObsidianBlockIdValueInside)
        }
        // Empty block ids and non-id characters are invalid.
        _ => State::Nok,
    }
}

/// Inside block id value, consuming characters.
///
/// ```markdown
/// > | ^abc-def
///       ^^^^^^
/// ```
pub fn value_inside(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(c) if c.is_ascii_alphanumeric() || c == b'-' || c == b'_' => {
            tokenizer.consume();
            State::Next(StateName::ObsidianBlockIdValueInside)
        }
        // EOL or EOF — block id ends.
        None | Some(b'\n') => {
            tokenizer.exit(Name::ObsidianBlockIdValue);
            tokenizer.exit(Name::ObsidianBlockId);
            State::Ok
        }
        // Any other character — not a valid block id.
        _ => State::Nok,
    }
}
