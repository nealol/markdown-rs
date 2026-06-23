//! Obsidian: shared target parser for [wikilinks][crate::construct::obsidian_wikilink] and [embeds][crate::construct::obsidian_embed].
//!
//! This construct provides the inner state machine that parses the content
//! between `[[` and `]]` (or `![[` and `]]`). It is invoked by the wikilink
//! and embed constructs after they consume their opening markers.
//!
//! The caller sets `tokenize_state.token_1` to `Name::ObsidianWikilink` or
//! `Name::ObsidianEmbed` before delegating to this parser, so the close
//! transition knows which construct to finish.
//!
//! ## Grammar
//!
//! ```bnf
//! target   ::= path? rest? alias?
//! path     ::= 1*byte - ( '#' | '|' | ']' )
//! rest     ::= '#' ( '^' block_id | heading )
//! heading  ::= *byte - ( '|' | ']' )
//! block_id ::= 1*( ascii_alphanumeric | '-' | '_' )
//! alias    ::= '|' 1*byte - ']'
//! ```
//!
//! ## Tokens
//!
//! * [`ObsidianTargetPath`][Name::ObsidianTargetPath]
//! * [`ObsidianTargetHash`][Name::ObsidianTargetHash]
//! * [`ObsidianTargetHeading`][Name::ObsidianTargetHeading]
//! * [`ObsidianTargetBlockIdMarker`][Name::ObsidianTargetBlockIdMarker]
//! * [`ObsidianTargetBlockId`][Name::ObsidianTargetBlockId]
//! * [`ObsidianTargetAliasMarker`][Name::ObsidianTargetAliasMarker]
//! * [`ObsidianTargetAlias`][Name::ObsidianTargetAlias]

use crate::event::Name;
use crate::state::{Name as StateName, State};
use crate::tokenizer::Tokenizer;

/// Start of target content (first byte after `[[` or `![[`).
///
/// ```markdown
/// > | [[Note]]
///       ^
/// > | [[#Heading]]
///       ^
/// ```
pub fn start(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        // Empty target, alias with no target, or EOF are invalid.
        Some(b']' | b'|') | None => State::Nok,
        // `#` — same-file heading or block reference.
        Some(b'#') => {
            tokenizer.enter(Name::ObsidianTargetHash);
            tokenizer.consume();
            tokenizer.exit(Name::ObsidianTargetHash);
            State::Next(StateName::ObsidianTargetAfterHash)
        }
        // Path starts here.
        Some(_) => {
            tokenizer.enter(Name::ObsidianTargetPath);
            State::Retry(StateName::ObsidianTargetPath)
        }
    }
}

/// In path.
///
/// ```markdown
/// > | [[Daily notes/2026]]
///       ^^^^^^^^^^^^^^^^^
/// ```
pub fn path(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        // Close `]]`.
        Some(b']') => {
            tokenizer.exit(Name::ObsidianTargetPath);
            State::Retry(StateName::ObsidianTargetClose)
        }
        // `#` — heading or block reference.
        Some(b'#') => {
            tokenizer.exit(Name::ObsidianTargetPath);
            tokenizer.enter(Name::ObsidianTargetHash);
            tokenizer.consume();
            tokenizer.exit(Name::ObsidianTargetHash);
            State::Next(StateName::ObsidianTargetAfterHash)
        }
        // `|` — alias.
        Some(b'|') => {
            tokenizer.exit(Name::ObsidianTargetPath);
            tokenizer.enter(Name::ObsidianTargetAliasMarker);
            tokenizer.consume();
            tokenizer.exit(Name::ObsidianTargetAliasMarker);
            State::Next(StateName::ObsidianTargetAfterPipe)
        }
        // Line ending or EOF is invalid inside target.
        Some(b'\n') | None => State::Nok,
        Some(_) => {
            tokenizer.consume();
            State::Next(StateName::ObsidianTargetPath)
        }
    }
}

/// After `#`, at heading text or `^` (block id).
///
/// ```markdown
/// > | [[Note#Heading]]
///             ^
/// > | [[Note#^id]]
///             ^
/// ```
pub fn after_hash(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        // `^` — block reference.
        Some(b'^') => {
            tokenizer.enter(Name::ObsidianTargetBlockIdMarker);
            tokenizer.consume();
            tokenizer.exit(Name::ObsidianTargetBlockIdMarker);
            State::Next(StateName::ObsidianTargetAfterCaret)
        }
        // Empty heading or EOF is invalid.
        Some(b']' | b'|') | None => State::Nok,
        // Heading text.
        Some(_) => {
            tokenizer.enter(Name::ObsidianTargetHeading);
            State::Retry(StateName::ObsidianTargetHeading)
        }
    }
}

/// In heading text.
///
/// ```markdown
/// > | [[Note#Heading]]
///              ^^^^^^^
/// ```
pub fn heading(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b']') => {
            tokenizer.exit(Name::ObsidianTargetHeading);
            State::Retry(StateName::ObsidianTargetClose)
        }
        Some(b'|') => {
            tokenizer.exit(Name::ObsidianTargetHeading);
            tokenizer.enter(Name::ObsidianTargetAliasMarker);
            tokenizer.consume();
            tokenizer.exit(Name::ObsidianTargetAliasMarker);
            State::Next(StateName::ObsidianTargetAfterPipe)
        }
        Some(b'\n') | None => State::Nok,
        Some(_) => {
            tokenizer.consume();
            State::Next(StateName::ObsidianTargetHeading)
        }
    }
}

/// After `^`, at block id value.
///
/// ```markdown
/// > | [[Note#^id]]
///              ^
/// ```
pub fn after_caret(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(c) if c.is_ascii_alphanumeric() || c == b'-' || c == b'_' => {
            tokenizer.enter(Name::ObsidianTargetBlockId);
            State::Retry(StateName::ObsidianTargetBlockId)
        }
        _ => State::Nok,
    }
}

/// In block id value.
///
/// ```markdown
/// > | [[Note#^abc-id]]
///               ^^^^^^
/// ```
pub fn block_id(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b']') => {
            tokenizer.exit(Name::ObsidianTargetBlockId);
            State::Retry(StateName::ObsidianTargetClose)
        }
        Some(b'|') => {
            tokenizer.exit(Name::ObsidianTargetBlockId);
            tokenizer.enter(Name::ObsidianTargetAliasMarker);
            tokenizer.consume();
            tokenizer.exit(Name::ObsidianTargetAliasMarker);
            State::Next(StateName::ObsidianTargetAfterPipe)
        }
        Some(c) if c.is_ascii_alphanumeric() || c == b'-' || c == b'_' => {
            tokenizer.consume();
            State::Next(StateName::ObsidianTargetBlockId)
        }
        _ => State::Nok,
    }
}

/// After `|`, at alias text.
///
/// ```markdown
/// > | [[Note|Alias]]
///             ^
/// ```
pub fn after_pipe(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        // Empty alias, invalid.
        Some(b']' | b'\n') | None => State::Nok,
        Some(_) => {
            tokenizer.enter(Name::ObsidianTargetAlias);
            State::Retry(StateName::ObsidianTargetAlias)
        }
    }
}

/// In alias text.
///
/// ```markdown
/// > | [[Note|Alias]]
///              ^^^^^
/// ```
pub fn alias(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b']') => {
            tokenizer.exit(Name::ObsidianTargetAlias);
            State::Retry(StateName::ObsidianTargetClose)
        }
        Some(b'\n') | None => State::Nok,
        Some(_) => {
            tokenizer.consume();
            State::Next(StateName::ObsidianTargetAlias)
        }
    }
}

/// At `]]`, closing the target. Enter the appropriate marker (wikilink or
/// embed based on `token_1`), consume the first `]`, and transition to the
/// construct's close state.
///
/// ```markdown
/// > | [[Note]]
///             ^
/// ```
pub fn close(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(b']') {
        let marker = if tokenizer.tokenize_state.token_1 == Name::ObsidianEmbed {
            Name::ObsidianEmbedMarker
        } else {
            Name::ObsidianWikilinkMarker
        };
        let close_state = if tokenizer.tokenize_state.token_1 == Name::ObsidianEmbed {
            StateName::ObsidianEmbedClose
        } else {
            StateName::ObsidianWikilinkClose
        };
        tokenizer.enter(marker);
        tokenizer.consume();
        State::Next(close_state)
    } else {
        State::Nok
    }
}
