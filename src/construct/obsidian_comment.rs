//! Obsidian: comment occurs in the [text][] content type.
//!
//! ## Grammar
//!
//! ```bnf
//! obsidian_comment ::= '%%' *byte '%%'
//! ```
//!
//! Comments are delimited by `%%` on both sides. The content between them is
//! arbitrary text (no nesting). Comments do not render to HTML.
//!
//! ## Examples
//!
//! ```markdown
//! > | a %%comment%% b
//!       ^^^^^^^^^^
//! ```
//!
//! ## Extension
//!
//! > 👉 **Note**: comments are not part of `CommonMark` or `GFM`. Enable them
//! > with `constructs.obsidian_comment` or use [`Constructs::obsidian()`].
//!
//! ## Tokens
//!
//! * [`ObsidianComment`][Name::ObsidianComment]
//! * [`ObsidianCommentMarker`][Name::ObsidianCommentMarker]
//! * [`ObsidianCommentValue`][Name::ObsidianCommentValue]
//!
//! ## References
//!
//! * [Obsidian Flavored Markdown](https://obsidian.md/help/obsidian-flavored-markdown)
//! * [Comments](https://help.obsidian.md/Editing+and+formatting/Basic+formatting+syntax#Comments)
//!
//! [text]: crate::construct::text

use crate::event::Name;
use crate::state::{Name as StateName, State};
use crate::tokenizer::Tokenizer;

/// Start of comment, at first `%`.
///
/// ```markdown
/// > | %%comment%%
///     ^
/// ```
pub fn start(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.parse_state.options.constructs.obsidian_comment && tokenizer.current == Some(b'%')
    {
        tokenizer.enter(Name::ObsidianComment);
        tokenizer.enter(Name::ObsidianCommentMarker);
        tokenizer.consume();
        State::Next(StateName::ObsidianCommentOpen)
    } else {
        State::Nok
    }
}

/// After first `%`, at second `%`.
///
/// ```markdown
/// > | %%comment%%
///      ^
/// ```
pub fn open(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(b'%') {
        tokenizer.consume();
        tokenizer.exit(Name::ObsidianCommentMarker);
        tokenizer.enter(Name::ObsidianCommentValue);
        State::Next(StateName::ObsidianCommentInside)
    } else {
        State::Nok
    }
}

/// In comment value, looking for closing `%%`.
///
/// ```markdown
/// > | %%comment%%
///       ^^^^^^
/// ```
pub fn inside(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        // Potential close `%%`.
        Some(b'%') => {
            // Exit value so the close marker is clean.
            tokenizer.exit(Name::ObsidianCommentValue);
            tokenizer.enter(Name::ObsidianCommentMarker);
            tokenizer.consume();
            State::Next(StateName::ObsidianCommentAfter)
        }
        None | Some(b'\n') => State::Nok,
        Some(_) => {
            tokenizer.consume();
            State::Next(StateName::ObsidianCommentInside)
        }
    }
}

/// After first `%` of potential close, at second `%`.
///
/// ```markdown
/// > | %%comment%%
///                 ^
/// ```
pub fn after(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(b'%') {
        tokenizer.consume();
        tokenizer.exit(Name::ObsidianCommentMarker);
        tokenizer.exit(Name::ObsidianComment);
        State::Ok
    } else {
        // Not a close — it was a lone `%`. Re-enter value and continue.
        tokenizer.exit(Name::ObsidianCommentMarker);
        tokenizer.enter(Name::ObsidianCommentValue);
        match tokenizer.current {
            None | Some(b'\n') => State::Nok,
            Some(_) => {
                tokenizer.consume();
                State::Next(StateName::ObsidianCommentInside)
            }
        }
    }
}
