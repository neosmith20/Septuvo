//! A small, strict text format for policies, treated as hostile input.
//!
//! ```text
//! # Untrusted Wi-Fi with one approved printer on the local network
//! mode full-tunnel
//! direct 192.168.1.20/32
//! block 10.0.0.0/8
//! ```
//!
//! One statement per line: `mode full-tunnel|split` exactly once, then any
//! number of `tunnel|direct|block <prefix>` lines. `#` starts a comment line.
//! Input is limited to [`MAX_INPUT_BYTES`] bytes and [`MAX_LINE_BYTES`] bytes
//! per line, printable ASCII only.

use crate::policy::{Action, Mode, Policy, PolicyError, Rule};
use crate::prefix::{Prefix, PrefixError};
use std::fmt;

/// Maximum size of a policy text.
pub const MAX_INPUT_BYTES: usize = 64 * 1024;
/// Maximum length of one line, excluding the line break.
pub const MAX_LINE_BYTES: usize = 256;

/// Why a policy text was rejected. `line` is 1-based; 0 means the whole input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseError {
    /// Line number (1-based), or 0 for whole-input problems.
    pub line: usize,
    /// What went wrong.
    pub kind: ParseErrorKind,
}

/// The kinds of [`ParseError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseErrorKind {
    /// Input larger than [`MAX_INPUT_BYTES`].
    InputTooLarge,
    /// Line longer than [`MAX_LINE_BYTES`].
    LineTooLong,
    /// A byte outside printable ASCII, space and tab.
    InvalidCharacter,
    /// The first word is not `mode`, `tunnel`, `direct` or `block`.
    UnknownStatement,
    /// Wrong number of words on the line.
    WrongArgumentCount,
    /// `mode` is not `full-tunnel` or `split`.
    UnknownMode,
    /// More than one `mode` line.
    DuplicateMode,
    /// No `mode` line.
    MissingMode,
    /// The prefix is invalid.
    Prefix(PrefixError),
    /// The rules are individually valid but the policy is not.
    Policy(PolicyError),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line > 0 {
            write!(f, "line {}: ", self.line)?;
        }
        match self.kind {
            ParseErrorKind::InputTooLarge => write!(f, "input exceeds {MAX_INPUT_BYTES} bytes"),
            ParseErrorKind::LineTooLong => write!(f, "line exceeds {MAX_LINE_BYTES} bytes"),
            ParseErrorKind::InvalidCharacter => f.write_str("invalid character"),
            ParseErrorKind::UnknownStatement => {
                f.write_str("expected 'mode', 'tunnel', 'direct' or 'block'")
            }
            ParseErrorKind::WrongArgumentCount => f.write_str("expected exactly one argument"),
            ParseErrorKind::UnknownMode => f.write_str("mode must be 'full-tunnel' or 'split'"),
            ParseErrorKind::DuplicateMode => f.write_str("mode is set more than once"),
            ParseErrorKind::MissingMode => f.write_str("no 'mode' line"),
            ParseErrorKind::Prefix(e) => write!(f, "invalid prefix: {e}"),
            ParseErrorKind::Policy(e) => write!(f, "invalid policy: {e}"),
        }
    }
}

impl std::error::Error for ParseError {}

fn err(line: usize, kind: ParseErrorKind) -> ParseError {
    ParseError { line, kind }
}

/// Parses and validates a policy text.
///
/// # Errors
/// The first problem found, with its line number.
pub fn parse_policy(input: &str) -> Result<Policy, ParseError> {
    if input.len() > MAX_INPUT_BYTES {
        return Err(err(0, ParseErrorKind::InputTooLarge));
    }
    let mut mode = None;
    let mut rules = Vec::new();
    for (index, raw) in input.split('\n').enumerate() {
        let number = index + 1;
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        if line.len() > MAX_LINE_BYTES {
            return Err(err(number, ParseErrorKind::LineTooLong));
        }
        if !line
            .bytes()
            .all(|b| b == b'\t' || (b' '..=b'~').contains(&b))
        {
            return Err(err(number, ParseErrorKind::InvalidCharacter));
        }
        let line = line.trim_matches([' ', '\t']);
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let words: Vec<&str> = line.split_ascii_whitespace().collect();
        let (keyword, argument) = match words.as_slice() {
            [keyword, argument] => (*keyword, *argument),
            [keyword, ..] if is_keyword(keyword) => {
                return Err(err(number, ParseErrorKind::WrongArgumentCount));
            }
            _ => return Err(err(number, ParseErrorKind::UnknownStatement)),
        };
        match keyword {
            "mode" => {
                if mode.is_some() {
                    return Err(err(number, ParseErrorKind::DuplicateMode));
                }
                mode = Some(match argument {
                    "full-tunnel" => Mode::FullTunnel,
                    "split" => Mode::Split,
                    _ => return Err(err(number, ParseErrorKind::UnknownMode)),
                });
            }
            "tunnel" | "direct" | "block" => {
                let action = match keyword {
                    "tunnel" => Action::Tunnel,
                    "direct" => Action::Direct,
                    _ => Action::Block,
                };
                let prefix: Prefix = argument
                    .parse()
                    .map_err(|e| err(number, ParseErrorKind::Prefix(e)))?;
                rules.push(Rule { prefix, action });
            }
            _ => return Err(err(number, ParseErrorKind::UnknownStatement)),
        }
    }
    let mode = mode.ok_or(err(0, ParseErrorKind::MissingMode))?;
    Policy::new(mode, rules).map_err(|e| err(0, ParseErrorKind::Policy(e)))
}

fn is_keyword(word: &str) -> bool {
    matches!(word, "mode" | "tunnel" | "direct" | "block")
}
