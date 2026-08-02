//! Turn bytes of markdown into events.

use crate::event::{Event, Point};
use crate::message;
use crate::state::{Name as StateName, State};
use crate::subtokenize::subtokenize;
use crate::tokenizer::Tokenizer;
use crate::util::location::Location;
use crate::ParseOptions;
use alloc::{string::String, vec, vec::Vec};
#[cfg(feature = "std")]
use core::{cell::RefCell, mem};

#[cfg(feature = "std")]
std::thread_local! {
    static EVENT_SCRATCH: RefCell<Vec<Event>> = const { RefCell::new(Vec::new()) };
}

/// Info needed, in all content types, when parsing markdown.
///
/// Importantly, this contains a set of known definitions.
/// It also references the input value as bytes (`u8`).
#[derive(Debug)]
pub struct ParseState<'a> {
    /// Configuration.
    pub location: Option<Location>,
    /// Configuration.
    pub options: &'a ParseOptions,
    /// List of chars.
    pub bytes: &'a [u8],
    /// Set of defined definition identifiers.
    pub definitions: Vec<String>,
    /// Set of defined GFM footnote definition identifiers.
    pub gfm_footnote_definitions: Vec<String>,
}

/// Turn a string of markdown into events.
///
/// Passes the bytes back so the compiler can access the source.
#[cfg(not(feature = "std"))]
pub fn parse<'a>(
    value: &'a str,
    options: &'a ParseOptions,
) -> Result<(Vec<Event>, ParseState<'a>), message::Message> {
    parse_with_definitions(value, options, vec![], vec![])
}

/// Parsed events returned to this worker's scratch slot on drop.
#[cfg(feature = "std")]
pub(crate) struct RecycledEvents(Vec<Event>);

#[cfg(feature = "std")]
impl core::ops::Deref for RecycledEvents {
    type Target = [Event];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(feature = "std")]
impl Drop for RecycledEvents {
    fn drop(&mut self) {
        self.0.clear();
        let events = mem::take(&mut self.0);
        EVENT_SCRATCH.with(|scratch| *scratch.borrow_mut() = events);
    }
}

/// Parse with event capacity retained by the current worker.
#[cfg(feature = "std")]
pub(crate) fn parse_recycled<'a>(
    value: &'a str,
    options: &'a ParseOptions,
) -> Result<(RecycledEvents, ParseState<'a>), message::Message> {
    let events = EVENT_SCRATCH.with(|scratch| mem::take(&mut *scratch.borrow_mut()));
    parse_with_storage(value, options, vec![], vec![], events)
        .map(|(events, state)| (RecycledEvents(events), state))
}

/// Turn a string of markdown into events with definitions inherited from a
/// preceding, independently parsed document region.
pub(crate) fn parse_with_definitions<'a>(
    value: &'a str,
    options: &'a ParseOptions,
    definitions: Vec<String>,
    gfm_footnote_definitions: Vec<String>,
) -> Result<(Vec<Event>, ParseState<'a>), message::Message> {
    parse_with_storage(
        value,
        options,
        definitions,
        gfm_footnote_definitions,
        vec![],
    )
}

fn parse_with_storage<'a>(
    value: &'a str,
    options: &'a ParseOptions,
    definitions: Vec<String>,
    gfm_footnote_definitions: Vec<String>,
    events: Vec<Event>,
) -> Result<(Vec<Event>, ParseState<'a>), message::Message> {
    let bytes = value.as_bytes();

    let mut parse_state = ParseState {
        options,
        bytes,
        location: if options.mdx_esm_parse.is_some() || options.mdx_expression_parse.is_some() {
            Some(Location::new(bytes))
        } else {
            None
        },
        definitions,
        gfm_footnote_definitions,
    };

    let start = Point {
        line: 1,
        column: 1,
        index: 0,
        vs: 0,
    };
    let mut tokenizer = Tokenizer::new_with_events(start, &parse_state, events);

    let state = tokenizer.push(
        (0, 0),
        (parse_state.bytes.len(), 0),
        State::Next(StateName::DocumentStart),
    );
    let mut result = tokenizer.flush(state, true)?;
    let mut events = tokenizer.events;

    loop {
        let fn_defs = &mut parse_state.gfm_footnote_definitions;
        let defs = &mut parse_state.definitions;
        fn_defs.append(&mut result.gfm_footnote_definitions);
        defs.append(&mut result.definitions);

        if result.done {
            return Ok((events, parse_state));
        }

        result = subtokenize(&mut events, &parse_state, None)?;
    }
}
