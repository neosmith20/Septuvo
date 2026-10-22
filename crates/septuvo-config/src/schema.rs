//! Typed, checked access to a parsed document (shared by the schemas).

use crate::error::{ConfigError, Problem};
use crate::toml::{Document, Entry, MAX_INPUT_BYTES, Span, Table, Value};
use septuvo_policy::Prefix;
use std::fs::File;
use std::io::Read;
use std::net::IpAddr;
use std::path::Path;

/// Reads a configuration file, refusing anything that is not a regular file
/// or is larger than the parser accepts.
pub(crate) fn read_bounded(path: &Path) -> Result<Vec<u8>, ConfigError> {
    let io = |e: std::io::Error| ConfigError::new(None, "", Problem::Io(e.to_string()));
    let file = File::open(path).map_err(io)?;
    if !file.metadata().map_err(io)?.is_file() {
        return Err(ConfigError::new(
            None,
            "",
            Problem::Io("not a regular file".into()),
        ));
    }
    let mut bytes = Vec::new();
    let limit = u64::try_from(MAX_INPUT_BYTES).unwrap_or(u64::MAX) + 1;
    file.take(limit).read_to_end(&mut bytes).map_err(io)?;
    Ok(bytes)
}

/// A table of the document, with its dotted path prefix for messages.
#[derive(Clone, Copy)]
pub(crate) struct Section<'a> {
    pub table: &'a Table,
}

impl<'a> Section<'a> {
    pub fn path(self, key: &str) -> String {
        if self.table.name.is_empty() {
            key.to_string()
        } else {
            format!("{}.{key}", self.table.name)
        }
    }

    /// Rejects keys that are not in `allowed`.
    pub fn only(self, allowed: &[&str]) -> Result<(), ConfigError> {
        match self
            .table
            .entries
            .iter()
            .find(|e| !allowed.contains(&e.key.as_str()))
        {
            Some(e) => Err(ConfigError::new(
                Some(e.key_span),
                self.path(&e.key),
                Problem::UnknownKey,
            )),
            None => Ok(()),
        }
    }

    fn entry(self, key: &str) -> Option<&'a Entry> {
        self.table.get(key)
    }

    fn wrong(self, e: &Entry, expected: &'static str) -> ConfigError {
        ConfigError::new(
            Some(e.value_span),
            self.path(&e.key),
            Problem::WrongType {
                expected,
                found: e.value.type_name(),
            },
        )
    }

    pub fn missing(self, key: &str) -> ConfigError {
        ConfigError::new(Some(self.table.span), self.path(key), Problem::Missing)
    }

    /// An optional integer within `min..=max`.
    pub fn int(self, key: &str, min: i64, max: i64) -> Result<Option<(i64, Span)>, ConfigError> {
        let Some(e) = self.entry(key) else {
            return Ok(None);
        };
        let Value::Integer(n) = e.value else {
            return Err(self.wrong(e, "an integer"));
        };
        if n < min || n > max {
            return Err(ConfigError::new(
                Some(e.value_span),
                self.path(key),
                Problem::OutOfRange { min, max },
            ));
        }
        Ok(Some((n, e.value_span)))
    }

    /// An optional string.
    pub fn string(self, key: &str) -> Result<Option<(&'a str, Span)>, ConfigError> {
        let Some(e) = self.entry(key) else {
            return Ok(None);
        };
        match &e.value {
            Value::String(s) => Ok(Some((s.as_str(), e.value_span))),
            _ => Err(self.wrong(e, "a string")),
        }
    }

    /// An optional array of strings (with each item's position).
    pub fn strings(self, key: &str) -> Result<Option<&'a [(String, Span)]>, ConfigError> {
        let Some(e) = self.entry(key) else {
            return Ok(None);
        };
        match &e.value {
            Value::Array(items) => Ok(Some(items.as_slice())),
            _ => Err(self.wrong(e, "an array of strings")),
        }
    }

    /// An invalid-value error for `key` at `span`.
    pub fn invalid(self, key: &str, span: Span, why: impl Into<String>) -> ConfigError {
        ConfigError::new(Some(span), self.path(key), Problem::Invalid(why.into()))
    }

    /// A conflict error for `key` at `span`.
    pub fn conflict(self, key: &str, span: Span, why: impl Into<String>) -> ConfigError {
        ConfigError::new(Some(span), self.path(key), Problem::Conflict(why.into()))
    }
}

/// Checks the top-level `version` and the set of tables.
///
/// `forbidden` lists table names that are rejected with a specific message
/// (for example settings that would weaken protection).
pub(crate) fn check_document(
    doc: &Document,
    tables: &[&str],
    forbidden: &[(&str, &str)],
) -> Result<(), ConfigError> {
    let root = Section { table: &doc.root };
    root.only(&["version"])?;
    match root.int("version", i64::MIN, i64::MAX)? {
        None => return Err(root.missing("version")),
        Some((1, _)) => {}
        Some((v, span)) => {
            return Err(ConfigError::new(
                Some(span),
                "version",
                Problem::UnsupportedVersion(v),
            ));
        }
    }
    for table in &doc.tables {
        if let Some((_, why)) = forbidden.iter().find(|(name, _)| *name == table.name) {
            return Err(ConfigError::new(
                Some(table.span),
                table.name.clone(),
                Problem::Conflict((*why).to_string()),
            ));
        }
        if !tables.contains(&table.name.as_str()) {
            return Err(ConfigError::new(
                Some(table.span),
                table.name.clone(),
                Problem::UnknownTable,
            ));
        }
    }
    Ok(())
}

/// The table `name`, or an error naming it if it is missing.
pub(crate) fn required_table<'a>(
    doc: &'a Document,
    name: &str,
) -> Result<Section<'a>, ConfigError> {
    doc.table(name)
        .map(|table| Section { table })
        .ok_or_else(|| ConfigError::new(Some(Span { line: 1, column: 1 }), name, Problem::Missing))
}

/// Parses an IP address that can be used as a peer or resolver: not
/// unspecified, multicast or loopback.
pub(crate) fn unicast_address(text: &str) -> Result<IpAddr, String> {
    let addr: IpAddr = text
        .parse()
        .map_err(|_| format!("'{text}' is not an IP address (host names are not accepted)"))?;
    if addr.is_unspecified() || addr.is_multicast() || addr.is_loopback() {
        return Err(format!(
            "'{text}' is not usable here (unspecified, multicast or loopback)"
        ));
    }
    Ok(addr)
}

/// Parses a canonical prefix such as `10.77.0.0/24`.
pub(crate) fn prefix(text: &str) -> Result<Prefix, String> {
    text.parse::<Prefix>()
        .map_err(|e| format!("'{text}' is not a valid prefix: {e}"))
}

/// True if `inner` lies entirely inside `outer`.
pub(crate) fn within(inner: Prefix, outer: Prefix) -> bool {
    inner.family() == outer.family()
        && inner.prefix_len() >= outer.prefix_len()
        && outer.contains(inner.addr())
}

/// The retention setting shared by both schemas (Q10: 7 days by default).
pub(crate) fn retention_days(doc: &Document) -> Result<u16, ConfigError> {
    let Some(table) = doc.table("logging") else {
        return Ok(DEFAULT_RETENTION_DAYS);
    };
    let section = Section { table };
    section.only(&["retention_days"])?;
    Ok(section
        .int("retention_days", 0, MAX_RETENTION_DAYS)?
        .and_then(|(n, _)| u16::try_from(n).ok())
        .unwrap_or(DEFAULT_RETENTION_DAYS))
}

/// Default log retention in days (owner decision Q10, D-032).
pub const DEFAULT_RETENTION_DAYS: u16 = 7;
/// Longest log retention accepted, in days.
pub const MAX_RETENTION_DAYS: i64 = 365;
