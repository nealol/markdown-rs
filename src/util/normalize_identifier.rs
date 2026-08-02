//! Normalize identifiers.

use alloc::string::String;

/// Normalize an identifier, as found in references and definitions.
///
/// ASCII Markdown whitespace is collapsed and trimmed. Each non-whitespace
/// scalar is lowercased and then uppercased, preserving the Unicode folding
/// behavior required by CommonMark.
pub fn normalize_identifier(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    normalize_identifier_into(value, &mut result);
    result
}

/// Normalize into reusable storage.
pub fn normalize_identifier_into(value: &str, result: &mut String) {
    result.clear();
    result.reserve(value.len().saturating_sub(result.capacity()));
    let mut pending_whitespace = false;

    for character in value.chars() {
        if matches!(character, '\t' | '\n' | '\r' | ' ') {
            if !result.is_empty() {
                pending_whitespace = true;
            }
            continue;
        }

        if pending_whitespace {
            result.push(' ');
            pending_whitespace = false;
        }
        for lowercase in character.to_lowercase() {
            for uppercase in lowercase.to_uppercase() {
                result.push(uppercase);
            }
        }
    }
}
