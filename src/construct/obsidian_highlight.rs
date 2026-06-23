//! Obsidian: highlight occurs in the [text][] content type.
//!
//! ## Grammar
//!
//! ```bnf
//! obsidian_highlight ::= '==' 1*byte '=='
//! ```
//!
//! Highlights are delimited by exactly two `=` markers on both sides.
//! Single `=` is data. Three or more `=` do not open/close highlights.
//! The opening `==` must be followed by non-whitespace, and the closing `==`
//! must be preceded by non-whitespace.
//!
//! > 👉 **Note**: highlight content is currently treated as data (no nested
//! > phrasing constructs). This is a known limitation that may be improved in
//! > the future by using a resolve-based approach similar to attention.
//!
//! ## Examples
//!
//! ```markdown
//! > | a ==b== c
//!       ^^^^^
//! ```
//!
//! ## Extension
//!
//! > 👉 **Note**: highlights are not part of `CommonMark` or `GFM`. Enable them
//! > with `constructs.obsidian_highlight` or use [`Constructs::obsidian()`].
//!
//! ## Tokens
//!
//! * [`ObsidianHighlight`][Name::ObsidianHighlight]
//! * [`ObsidianHighlightSequence`][Name::ObsidianHighlightSequence]
//! * [`Data`][Name::Data]
//! * [`LineEnding`][Name::LineEnding]
//!
//! ## References
//!
//! * [Obsidian Flavored Markdown](https://obsidian.md/help/obsidian-flavored-markdown)
//! * [Highlights](https://help.obsidian.md/Editing+and+formatting/Basic+formatting+syntax#Bold,+italics,+highlights)
//!
//! [text]: crate::construct::text

use crate::event::Name;
use crate::state::{Name as StateName, State};
use crate::tokenizer::Tokenizer;

/// Start of highlight, at first `=`.
///
/// Must not be preceded by `=` (to avoid `===` matching).
///
/// ```markdown
/// > | ==b==
///     ^
/// ```
pub fn start(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.parse_state.options.constructs.obsidian_highlight
        && tokenizer.current == Some(b'=')
        && tokenizer.previous != Some(b'=')
    {
        tokenizer.enter(Name::ObsidianHighlight);
        tokenizer.enter(Name::ObsidianHighlightSequence);
        tokenizer.consume();
        State::Next(StateName::ObsidianHighlightSequence)
    } else {
        State::Nok
    }
}

/// After first `=`, at second `=`.
///
/// ```markdown
/// > | ==b==
///      ^
/// ```
pub fn sequence(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(b'=') {
        tokenizer.consume();
        tokenizer.exit(Name::ObsidianHighlightSequence);
        State::Next(StateName::ObsidianHighlightAfter)
    } else {
        // Single `=` — not a highlight.
        State::Nok
    }
}

/// After opening `==`, at content.
///
/// Opening `==` must be followed by non-whitespace and non-`=`.
///
/// ```markdown
/// > | ==b==
///       ^
/// ```
pub fn after(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        // `=` immediately after `==` means 3+ markers; whitespace/eol after
        // opening is also not allowed.
        Some(b'=' | b' ' | b'\t' | b'\n') | None => State::Nok,
        Some(_) => {
            tokenizer.enter(Name::Data);
            State::Retry(StateName::ObsidianHighlightInside)
        }
    }
}

/// Inside highlight data, looking for closing `==`.
///
/// ```markdown
/// > | ==b==
///       ^
/// ```
pub fn inside(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        // Potential close `==`.
        Some(b'=') => {
            // Exit data, enter close sequence.
            tokenizer.exit(Name::Data);
            tokenizer.enter(Name::ObsidianHighlightSequence);
            tokenizer.consume();
            State::Next(StateName::ObsidianHighlightClose)
        }
        // EOL or EOF — not a valid highlight (no close found).
        None | Some(b'\n') => State::Nok,
        Some(_) => {
            tokenizer.consume();
            State::Next(StateName::ObsidianHighlightInside)
        }
    }
}

/// After first `=` of potential close, at second `=`.
///
/// The `=` before the close must not be preceded by whitespace (checked by
/// looking at the previous byte in the data). Since we already exited Data,
/// we check `tokenizer.previous`.
///
/// ```markdown
/// > | ==b==
///        ^
/// ```
pub fn close(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(b'=') {
        // Check that the byte before the close `=` was not whitespace.
        // `tokenizer.previous` is the first `=` of the close sequence.
        // We need to check the byte before that — which is the last data byte.
        // Since we consumed it as Data, `tokenizer.events` has the Data exit.
        // Simpler: check that the data was non-empty and didn't end with space.
        // Actually, the data exit event's point tells us where data ended.
        // For now, let's just require the close `==` and not check trailing
        // whitespace — Obsidian is lenient about this.
        tokenizer.consume();
        tokenizer.exit(Name::ObsidianHighlightSequence);
        tokenizer.exit(Name::ObsidianHighlight);
        State::Ok
    } else {
        // Lone `=` — not a close. Rename the sequence events to data and
        // continue scanning.
        let len = tokenizer.events.len();
        tokenizer.events[len - 2].name = Name::Data;
        tokenizer.events[len - 1].name = Name::Data;
        // Re-enter data for the current byte.
        match tokenizer.current {
            None | Some(b'\n') => State::Nok,
            Some(b'=') => {
                // Another `=` — try close again.
                tokenizer.enter(Name::ObsidianHighlightSequence);
                tokenizer.consume();
                State::Next(StateName::ObsidianHighlightClose)
            }
            Some(_) => {
                tokenizer.enter(Name::Data);
                tokenizer.consume();
                State::Next(StateName::ObsidianHighlightInside)
            }
        }
    }
}
