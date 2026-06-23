//! Obsidian: embed occurs in the [text][] content type.
//!
//! ## Grammar
//!
//! ```bnf
//! obsidian_embed ::= '![[' target ']]'
//! ```
//!
//! Where `target` is defined by [`obsidian_target`][crate::construct::obsidian_target].
//!
//! ## Examples
//!
//! ```markdown
//! > | ![[Note]]
//!     ^^^^^^^^^
//! > | ![[image.png]]
//!     ^^^^^^^^^^^^^^
//! > | ![[Note#^block-id]]
//!     ^^^^^^^^^^^^^^^^^^^^
//! ```
//!
//! [text]: crate::construct::text

use crate::event::Name;
use crate::state::{Name as StateName, State};
use crate::tokenizer::Tokenizer;

/// Start of embed, at `!`.
///
/// ```markdown
/// > | ![[Note]]
///     ^
/// ```
pub fn start(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.parse_state.options.constructs.obsidian_embed && tokenizer.current == Some(b'!') {
        tokenizer.tokenize_state.token_1 = Name::ObsidianEmbed;
        tokenizer.enter(Name::ObsidianEmbed);
        tokenizer.enter(Name::ObsidianEmbedMarker);
        tokenizer.consume();
        State::Next(StateName::ObsidianEmbedOpen)
    } else {
        State::Nok
    }
}

/// After `!`, at first `[`.
///
/// ```markdown
/// > | ![[Note]]
///      ^
/// ```
pub fn open(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(b'[') {
        tokenizer.consume();
        State::Next(StateName::ObsidianEmbedOpen2)
    } else {
        State::Nok
    }
}

/// After `![`, at second `[`.
///
/// ```markdown
/// > | ![[Note]]
///       ^
/// ```
pub fn open_2(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(b'[') {
        tokenizer.consume();
        tokenizer.exit(Name::ObsidianEmbedMarker);
        // Delegate to shared target parser. Use Next (not Retry) so the next
        // byte is fed to the target parser.
        State::Next(StateName::ObsidianTargetStart)
    } else {
        State::Nok
    }
}

/// At second `]` of `]]`, closing the embed.
///
/// ```markdown
/// > | ![[Note]]
///               ^
/// ```
pub fn close(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(b']') {
        tokenizer.consume();
        tokenizer.exit(Name::ObsidianEmbedMarker);
        tokenizer.exit(Name::ObsidianEmbed);
        // Reset token_1 to default.
        tokenizer.tokenize_state.token_1 = Name::Data;
        State::Ok
    } else {
        State::Nok
    }
}
