//! Obsidian: wikilink occurs in the [text][] content type.
//!
//! ## Grammar
//!
//! ```bnf
//! obsidian_wikilink ::= '[[' target ']]'
//! ```
//!
//! Where `target` is defined by [`obsidian_target`][crate::construct::obsidian_target].
//!
//! ## Examples
//!
//! ```markdown
//! > | [[Note]]
//!     ^^^^^^^^
//! > | [[Note#Heading]]
//!     ^^^^^^^^^^^^^^^^^
//! > | [[Note#^block-id]]
//!     ^^^^^^^^^^^^^^^^^^^
//! > | [[Note|Alias]]
//!     ^^^^^^^^^^^^^^
//! > | [[#Heading]]
//!     ^^^^^^^^^^^^
//! ```
//!
//! [text]: crate::construct::text

use crate::event::Name;
use crate::state::{Name as StateName, State};
use crate::tokenizer::Tokenizer;

/// Start of wikilink, at first `[`.
///
/// ```markdown
/// > | [[Note]]
///     ^
/// ```
pub fn start(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.parse_state.options.constructs.obsidian_wikilink && tokenizer.current == Some(b'[')
    {
        tokenizer.tokenize_state.token_1 = Name::ObsidianWikilink;
        tokenizer.enter(Name::ObsidianWikilink);
        tokenizer.enter(Name::ObsidianWikilinkMarker);
        tokenizer.consume();
        State::Next(StateName::ObsidianWikilinkOpen)
    } else {
        State::Nok
    }
}

/// After first `[`, at second `[`.
///
/// ```markdown
/// > | [[Note]]
///      ^
/// ```
pub fn open(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(b'[') {
        tokenizer.consume();
        tokenizer.exit(Name::ObsidianWikilinkMarker);
        // Delegate to shared target parser. Use Next (not Retry) so the next
        // byte is fed to the target parser.
        State::Next(StateName::ObsidianTargetStart)
    } else {
        State::Nok
    }
}

/// At second `]` of `]]`, closing the wikilink.
///
/// ```markdown
/// > | [[Note]]
///              ^
/// ```
pub fn close(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(b']') {
        tokenizer.consume();
        tokenizer.exit(Name::ObsidianWikilinkMarker);
        tokenizer.exit(Name::ObsidianWikilink);
        // Reset token_1 to default.
        tokenizer.tokenize_state.token_1 = Name::Data;
        State::Ok
    } else {
        State::Nok
    }
}
