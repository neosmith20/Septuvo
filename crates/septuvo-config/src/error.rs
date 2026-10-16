//! Configuration errors with their location.

use crate::toml::{ParseError, Span, SyntaxError};
use std::fmt;

/// What is wrong with a configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    /// The file could not be read.
    Io(String),
    /// The file is not valid in the supported TOML subset.
    Syntax(SyntaxError),
    /// `version` names a format this program does not know.
    UnsupportedVersion(i64),
    /// A `[table]` this schema does not define.
    UnknownTable,
    /// A key this table does not define.
    UnknownKey,
    /// A required key or table is missing.
    Missing,
    /// The value has the wrong type.
    WrongType {
        /// The type the schema requires.
        expected: &'static str,
        /// The type found in the file.
        found: &'static str,
    },
    /// An integer outside its allowed range.
    OutOfRange {
        /// Smallest allowed value.
        min: i64,
        /// Largest allowed value.
        max: i64,
    },
    /// The value is malformed or not allowed; the text says why.
    Invalid(String),
    /// Two settings contradict each other, or a setting would weaken
    /// protection; the text says why.
    Conflict(String),
    /// The setting needs reviewed enrollment or protocol work that does not
    /// exist before gate G1.
    NotAvailableBeforeG1,
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Problem::Io(why) => write!(f, "cannot read the file: {why}"),
            Problem::Syntax(e) => write!(f, "{e}"),
            Problem::UnsupportedVersion(v) => {
                write!(
                    f,
                    "unsupported format version {v} (this program reads version 1)"
                )
            }
            Problem::UnknownTable => f.write_str("unknown table"),
            Problem::UnknownKey => f.write_str("unknown key"),
            Problem::Missing => f.write_str("required setting is missing"),
            Problem::WrongType { expected, found } => {
                write!(f, "expected {expected}, found {found}")
            }
            Problem::OutOfRange { min, max } => {
                write!(f, "value must be between {min} and {max}")
            }
            Problem::Invalid(why) | Problem::Conflict(why) => f.write_str(why),
            Problem::NotAvailableBeforeG1 => f.write_str(
                "not available: enrollment and the Septuvo Protocol are not implemented \
                 before the G1 security review",
            ),
        }
    }
}

/// A configuration error: what, where, and for which setting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigError {
    /// Where in the file, if known.
    pub span: Option<Span>,
    /// The setting, such as `dns.resolvers`; empty if the whole file.
    pub path: String,
    /// What is wrong.
    pub problem: Problem,
}

impl ConfigError {
    pub(crate) fn new(span: Option<Span>, path: impl Into<String>, problem: Problem) -> Self {
        ConfigError {
            span,
            path: path.into(),
            problem,
        }
    }
}

impl From<ParseError> for ConfigError {
    fn from(e: ParseError) -> Self {
        ConfigError::new(Some(e.span), "", Problem::Syntax(e.error))
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(span) = self.span {
            write!(f, "{span}: ")?;
        }
        if !self.path.is_empty() {
            write!(f, "{}: ", self.path)?;
        }
        write!(f, "{}", self.problem)
    }
}

impl std::error::Error for ConfigError {}
