//! A strict subset of TOML, parsed with the standard library only.
//!
//! Every document this parser accepts is also valid TOML, so a full TOML
//! parser could replace it later without changing anyone's files. The subset
//! is deliberately small, because configuration is hostile input (ADV-10):
//!
//! - `[table]` headers with bare names (no dotted, quoted or array tables);
//! - `key = value` with bare keys (no dotted or quoted keys);
//! - values: basic `"..."` and literal `'...'` single-line strings, decimal
//!   integers, `true`/`false`, and arrays of strings (which may span lines);
//! - `#` comments, LF or CRLF line endings, an optional leading byte-order mark.
//!
//! Anything else (floats, dates, inline tables, multi-line strings, arrays of
//! tables) is rejected with a clear message rather than guessed at.
//! Every input is bounded: see the `MAX_*` constants.

use std::fmt;

/// Largest accepted document, in bytes.
pub const MAX_INPUT_BYTES: usize = 64 * 1024;
/// Longest accepted line, in characters.
pub const MAX_LINE_CHARS: usize = 1024;
/// Longest accepted string value, in characters (after escapes).
pub const MAX_STRING_CHARS: usize = 512;
/// Most items in one array.
pub const MAX_ARRAY_ITEMS: usize = 256;
/// Most `[table]` sections in one document.
pub const MAX_TABLES: usize = 64;
/// Most keys in one table (including the top level).
pub const MAX_KEYS_PER_TABLE: usize = 128;
/// Longest accepted key or table name, in characters.
pub const MAX_NAME_CHARS: usize = 64;

/// A position in the input: 1-based line and column (in characters).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Span {
    /// Line number, starting at 1.
    pub line: usize,
    /// Column number in characters, starting at 1.
    pub column: usize,
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}, column {}", self.line, self.column)
    }
}

/// A value in the subset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    /// A string.
    String(String),
    /// A decimal integer.
    Integer(i64),
    /// `true` or `false`.
    Boolean(bool),
    /// An array of strings, each with its position.
    Array(Vec<(String, Span)>),
}

impl Value {
    /// A short name of the value's type, for error messages.
    #[must_use]
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::String(_) => "string",
            Value::Integer(_) => "integer",
            Value::Boolean(_) => "boolean",
            Value::Array(_) => "array",
        }
    }
}

/// One `key = value` line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The key.
    pub key: String,
    /// Where the key starts.
    pub key_span: Span,
    /// The value.
    pub value: Value,
    /// Where the value starts.
    pub value_span: Span,
}

/// A table: the top level or one `[name]` section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    /// The table name; empty for the top level.
    pub name: String,
    /// Where the `[name]` header starts (line 1, column 1 for the top level).
    pub span: Span,
    /// The entries, in file order. Keys are unique within a table.
    pub entries: Vec<Entry>,
}

impl Table {
    /// The entry for `key`, if present.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Entry> {
        self.entries.iter().find(|e| e.key == key)
    }
}

/// A parsed document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    /// Keys before the first `[table]` header.
    pub root: Table,
    /// The `[table]` sections, in file order. Names are unique.
    pub tables: Vec<Table>,
}

impl Document {
    /// The table called `name`, if present.
    #[must_use]
    pub fn table(&self, name: &str) -> Option<&Table> {
        self.tables.iter().find(|t| t.name == name)
    }
}

/// What is wrong with the input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntaxError {
    /// Larger than [`MAX_INPUT_BYTES`].
    InputTooLarge,
    /// Not valid UTF-8.
    NotUtf8,
    /// A line longer than [`MAX_LINE_CHARS`].
    LineTooLong,
    /// A carriage return not followed by a line feed.
    BareCarriageReturn,
    /// A control character outside a string escape.
    ControlCharacter,
    /// Expected a key, a `[table]` header, a comment or a blank line.
    ExpectedKey,
    /// Expected `=` after the key.
    ExpectedEquals,
    /// Expected a value after `=`.
    ExpectedValue,
    /// Something other than a comment follows a complete line.
    TrailingCharacters,
    /// A key or table name is longer than [`MAX_NAME_CHARS`].
    NameTooLong,
    /// A key appears twice in one table.
    DuplicateKey,
    /// A `[table]` appears twice, or reuses a top-level key's name.
    DuplicateTable,
    /// More than [`MAX_TABLES`] tables.
    TooManyTables,
    /// More than [`MAX_KEYS_PER_TABLE`] keys in one table.
    TooManyKeys,
    /// A string longer than [`MAX_STRING_CHARS`].
    StringTooLong,
    /// A string that is not closed on the same line.
    UnterminatedString,
    /// An unknown or malformed escape sequence.
    InvalidEscape,
    /// An array with more than [`MAX_ARRAY_ITEMS`] items.
    ArrayTooLong,
    /// An array that is not closed.
    UnterminatedArray,
    /// An array item that is not a string, or a missing comma.
    InvalidArrayItem,
    /// An integer that is malformed or outside the 64-bit range.
    InvalidInteger,
    /// Valid TOML that this subset deliberately does not support.
    Unsupported(Unsupported),
}

/// TOML features outside the supported subset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unsupported {
    /// `a.b = ...` or `[a.b]`.
    DottedName,
    /// `"key" = ...` or `["table"]`.
    QuotedName,
    /// `[[table]]`.
    ArrayOfTables,
    /// `{ ... }`.
    InlineTable,
    /// `"""..."""` or `'''...'''`.
    MultiLineString,
    /// Floating-point numbers, infinities and NaN.
    Float,
    /// Dates and times.
    DateTime,
    /// Hexadecimal, octal or binary integers, or `_` separators.
    IntegerFormat,
}

impl fmt::Display for Unsupported {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Unsupported::DottedName => "dotted keys and table names",
            Unsupported::QuotedName => "quoted keys and table names",
            Unsupported::ArrayOfTables => "arrays of tables ([[...]])",
            Unsupported::InlineTable => "inline tables ({ ... })",
            Unsupported::MultiLineString => "multi-line strings",
            Unsupported::Float => "floating-point numbers",
            Unsupported::DateTime => "dates and times",
            Unsupported::IntegerFormat => {
                "hexadecimal, octal, binary or underscore-separated integers"
            }
        })
    }
}

impl fmt::Display for SyntaxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SyntaxError::InputTooLarge => write!(f, "file is larger than {MAX_INPUT_BYTES} bytes"),
            SyntaxError::NotUtf8 => f.write_str("file is not valid UTF-8 text"),
            SyntaxError::LineTooLong => {
                write!(f, "line is longer than {MAX_LINE_CHARS} characters")
            }
            SyntaxError::BareCarriageReturn => {
                f.write_str("carriage return without a following line feed")
            }
            SyntaxError::ControlCharacter => f.write_str("control character not allowed here"),
            SyntaxError::ExpectedKey => {
                f.write_str("expected a key, a [table] header, a comment or a blank line")
            }
            SyntaxError::ExpectedEquals => f.write_str("expected '=' after the key"),
            SyntaxError::ExpectedValue => f.write_str("expected a value"),
            SyntaxError::TrailingCharacters => {
                f.write_str("unexpected characters after the value (only a comment may follow)")
            }
            SyntaxError::NameTooLong => {
                write!(f, "name is longer than {MAX_NAME_CHARS} characters")
            }
            SyntaxError::DuplicateKey => f.write_str("key is defined more than once"),
            SyntaxError::DuplicateTable => f.write_str("table is defined more than once"),
            SyntaxError::TooManyTables => write!(f, "more than {MAX_TABLES} tables"),
            SyntaxError::TooManyKeys => {
                write!(f, "more than {MAX_KEYS_PER_TABLE} keys in one table")
            }
            SyntaxError::StringTooLong => {
                write!(f, "string is longer than {MAX_STRING_CHARS} characters")
            }
            SyntaxError::UnterminatedString => f.write_str("string is not closed on this line"),
            SyntaxError::InvalidEscape => f.write_str("invalid escape sequence"),
            SyntaxError::ArrayTooLong => write!(f, "array has more than {MAX_ARRAY_ITEMS} items"),
            SyntaxError::UnterminatedArray => f.write_str("array is not closed"),
            SyntaxError::InvalidArrayItem => {
                f.write_str("expected a string, ',' or ']' in the array")
            }
            SyntaxError::InvalidInteger => {
                f.write_str("invalid integer (decimal, 64-bit, no leading zeros)")
            }
            SyntaxError::Unsupported(what) => {
                write!(f, "{what} are valid TOML but not supported by Septuvo")
            }
        }
    }
}

/// A syntax error and where it was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseError {
    /// Where the problem is.
    pub span: Span,
    /// What the problem is.
    pub error: SyntaxError,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.span, self.error)
    }
}

impl std::error::Error for ParseError {}

/// Parses raw bytes (for example a file's contents).
///
/// # Errors
/// [`SyntaxError::InputTooLarge`] or [`SyntaxError::NotUtf8`], or any error
/// from [`parse`].
pub fn parse_bytes(input: &[u8]) -> Result<Document, ParseError> {
    let start = Span { line: 1, column: 1 };
    if input.len() > MAX_INPUT_BYTES {
        return Err(ParseError {
            span: start,
            error: SyntaxError::InputTooLarge,
        });
    }
    match std::str::from_utf8(input) {
        Ok(text) => parse(text),
        Err(e) => {
            // Report the line of the first invalid byte.
            let valid = &input[..e.valid_up_to()];
            let line = valid.split(|&b| b == b'\n').count();
            Err(ParseError {
                span: Span { line, column: 1 },
                error: SyntaxError::NotUtf8,
            })
        }
    }
}

/// Parses a document.
///
/// # Errors
/// The first problem found, with its position.
pub fn parse(input: &str) -> Result<Document, ParseError> {
    if input.len() > MAX_INPUT_BYTES {
        return Err(ParseError {
            span: Span { line: 1, column: 1 },
            error: SyntaxError::InputTooLarge,
        });
    }
    let input = input.strip_prefix('\u{feff}').unwrap_or(input);
    check_lines(input)?;
    Parser::new(input).document()
}

/// Line-level checks done before parsing: lengths and line endings.
fn check_lines(input: &str) -> Result<(), ParseError> {
    for (index, line) in input.split('\n').enumerate() {
        let span = Span {
            line: index + 1,
            column: 1,
        };
        let body = line.strip_suffix('\r').unwrap_or(line);
        if let Some(pos) = body.chars().position(|c| c == '\r') {
            return Err(ParseError {
                span: Span {
                    column: pos + 1,
                    ..span
                },
                error: SyntaxError::BareCarriageReturn,
            });
        }
        if body.chars().count() > MAX_LINE_CHARS {
            return Err(ParseError {
                span,
                error: SyntaxError::LineTooLong,
            });
        }
    }
    Ok(())
}

fn is_bare_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

/// Control characters TOML forbids outside escapes (tab is allowed).
fn is_forbidden_control(c: char) -> bool {
    (c.is_control() && c != '\t') || c == '\u{7f}'
}

struct Parser<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
    line: usize,
    column: usize,
}

type PResult<T> = Result<T, ParseError>;

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Parser {
            chars: input.chars().peekable(),
            line: 1,
            column: 1,
        }
    }

    fn span(&self) -> Span {
        Span {
            line: self.line,
            column: self.column,
        }
    }

    fn fail<T>(&self, error: SyntaxError) -> PResult<T> {
        Err(ParseError {
            span: self.span(),
            error,
        })
    }

    fn peek(&mut self) -> Option<char> {
        self.chars.peek().copied()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.chars.next()?;
        if c == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(c)
    }

    fn skip_spaces(&mut self) {
        while matches!(self.peek(), Some(' ' | '\t')) {
            self.bump();
        }
    }

    /// Skips a comment (if any) up to, not including, the line feed.
    fn skip_comment(&mut self) -> PResult<()> {
        if self.peek() == Some('#') {
            while let Some(c) = self.peek() {
                if c == '\n' || c == '\r' {
                    break;
                }
                if is_forbidden_control(c) {
                    return self.fail(SyntaxError::ControlCharacter);
                }
                self.bump();
            }
        }
        Ok(())
    }

    /// After a complete header or key/value: spaces, an optional comment,
    /// then a line ending or the end of input.
    fn end_of_line(&mut self) -> PResult<()> {
        self.skip_spaces();
        self.skip_comment()?;
        match self.peek() {
            None => Ok(()),
            Some('\r') => {
                self.bump();
                self.bump(); // check_lines guarantees the '\n'
                Ok(())
            }
            Some('\n') => {
                self.bump();
                Ok(())
            }
            Some(_) => self.fail(SyntaxError::TrailingCharacters),
        }
    }

    fn document(mut self) -> PResult<Document> {
        let mut root = Table {
            name: String::new(),
            span: Span { line: 1, column: 1 },
            entries: Vec::new(),
        };
        let mut tables: Vec<Table> = Vec::new();
        loop {
            self.skip_spaces();
            match self.peek() {
                None => break,
                Some('\r' | '\n' | '#') => self.end_of_line()?,
                Some('[') => {
                    let table = self.header()?;
                    let clash = tables.iter().any(|t| t.name == table.name)
                        || root.get(&table.name).is_some();
                    if clash {
                        return Err(ParseError {
                            span: table.span,
                            error: SyntaxError::DuplicateTable,
                        });
                    }
                    if tables.len() == MAX_TABLES {
                        return Err(ParseError {
                            span: table.span,
                            error: SyntaxError::TooManyTables,
                        });
                    }
                    tables.push(table);
                    self.end_of_line()?;
                }
                Some(_) => {
                    let entry = self.key_value()?;
                    let table = tables.last_mut().unwrap_or(&mut root);
                    if table.get(&entry.key).is_some() {
                        return Err(ParseError {
                            span: entry.key_span,
                            error: SyntaxError::DuplicateKey,
                        });
                    }
                    if table.entries.len() == MAX_KEYS_PER_TABLE {
                        return Err(ParseError {
                            span: entry.key_span,
                            error: SyntaxError::TooManyKeys,
                        });
                    }
                    table.entries.push(entry);
                    self.end_of_line()?;
                }
            }
        }
        Ok(Document { root, tables })
    }

    fn header(&mut self) -> PResult<Table> {
        let span = self.span();
        self.bump(); // '['
        if self.peek() == Some('[') {
            return self.fail(SyntaxError::Unsupported(Unsupported::ArrayOfTables));
        }
        self.skip_spaces();
        let name = self.name()?;
        self.skip_spaces();
        match self.peek() {
            Some(']') => {
                self.bump();
            }
            Some('.') => return self.fail(SyntaxError::Unsupported(Unsupported::DottedName)),
            _ => return self.fail(SyntaxError::ExpectedKey),
        }
        Ok(Table {
            name,
            span,
            entries: Vec::new(),
        })
    }

    /// A bare key or table name.
    fn name(&mut self) -> PResult<String> {
        if matches!(self.peek(), Some('"' | '\'')) {
            return self.fail(SyntaxError::Unsupported(Unsupported::QuotedName));
        }
        let mut name = String::new();
        while let Some(c) = self.peek() {
            if !is_bare_char(c) {
                break;
            }
            if name.chars().count() == MAX_NAME_CHARS {
                return self.fail(SyntaxError::NameTooLong);
            }
            name.push(c);
            self.bump();
        }
        if name.is_empty() {
            return self.fail(SyntaxError::ExpectedKey);
        }
        Ok(name)
    }

    fn key_value(&mut self) -> PResult<Entry> {
        let key_span = self.span();
        let key = self.name()?;
        self.skip_spaces();
        match self.peek() {
            Some('=') => {
                self.bump();
            }
            Some('.') => return self.fail(SyntaxError::Unsupported(Unsupported::DottedName)),
            _ => return self.fail(SyntaxError::ExpectedEquals),
        }
        self.skip_spaces();
        let value_span = self.span();
        let value = self.value()?;
        Ok(Entry {
            key,
            key_span,
            value,
            value_span,
        })
    }

    fn value(&mut self) -> PResult<Value> {
        match self.peek() {
            Some('"' | '\'') => Ok(Value::String(self.string()?)),
            Some('[') => self.array(),
            Some('{') => self.fail(SyntaxError::Unsupported(Unsupported::InlineTable)),
            Some('t' | 'f') => self.boolean(),
            Some(c) if c.is_ascii_digit() || c == '+' || c == '-' => self.integer(),
            Some('i' | 'n') => self.fail(SyntaxError::Unsupported(Unsupported::Float)),
            _ => self.fail(SyntaxError::ExpectedValue),
        }
    }

    fn boolean(&mut self) -> PResult<Value> {
        let word = self.word();
        match word.as_str() {
            "true" => Ok(Value::Boolean(true)),
            "false" => Ok(Value::Boolean(false)),
            _ => self.fail(SyntaxError::ExpectedValue),
        }
    }

    /// Characters up to the next delimiter (space, comment, comma, bracket,
    /// line ending).
    fn word(&mut self) -> String {
        let mut word = String::new();
        while let Some(c) = self.peek() {
            if matches!(c, ' ' | '\t' | '#' | ',' | ']' | '\r' | '\n') {
                break;
            }
            word.push(c);
            self.bump();
            if word.len() > 64 {
                break; // bounded; the caller rejects it
            }
        }
        word
    }

    fn integer(&mut self) -> PResult<Value> {
        let span = self.span();
        let word = self.word();
        let fail = |error| Err(ParseError { span, error });
        let digits = word.strip_prefix(['+', '-']).unwrap_or(&word);
        if digits.contains(['.', 'e', 'E']) || digits == "inf" || digits == "nan" {
            return fail(SyntaxError::Unsupported(Unsupported::Float));
        }
        if digits.contains([':', 'T', 'Z']) || (digits.len() > 4 && digits.as_bytes()[4] == b'-') {
            return fail(SyntaxError::Unsupported(Unsupported::DateTime));
        }
        if digits.starts_with("0x")
            || digits.starts_with("0o")
            || digits.starts_with("0b")
            || digits.contains('_')
        {
            return fail(SyntaxError::Unsupported(Unsupported::IntegerFormat));
        }
        let well_formed = !digits.is_empty()
            && digits.bytes().all(|b| b.is_ascii_digit())
            && (digits == "0" || !digits.starts_with('0'));
        if !well_formed {
            return fail(SyntaxError::InvalidInteger);
        }
        match word.parse::<i64>() {
            Ok(n) => Ok(Value::Integer(n)),
            Err(_) => fail(SyntaxError::InvalidInteger),
        }
    }

    fn string(&mut self) -> PResult<String> {
        let quote = self.bump().unwrap_or('"');
        if self.peek() == Some(quote) {
            self.bump();
            if self.peek() == Some(quote) {
                return self.fail(SyntaxError::Unsupported(Unsupported::MultiLineString));
            }
            return Ok(String::new());
        }
        let mut out = String::new();
        loop {
            let Some(c) = self.peek() else {
                return self.fail(SyntaxError::UnterminatedString);
            };
            if c == '\n' || c == '\r' {
                return self.fail(SyntaxError::UnterminatedString);
            }
            if c == quote {
                self.bump();
                return Ok(out);
            }
            if is_forbidden_control(c) {
                return self.fail(SyntaxError::ControlCharacter);
            }
            if out.chars().count() == MAX_STRING_CHARS {
                return self.fail(SyntaxError::StringTooLong);
            }
            if c == '\\' && quote == '"' {
                out.push(self.escape()?);
            } else {
                out.push(c);
                self.bump();
            }
        }
    }

    fn escape(&mut self) -> PResult<char> {
        self.bump(); // '\\'
        let Some(c) = self.bump() else {
            return self.fail(SyntaxError::UnterminatedString);
        };
        let simple = match c {
            'b' => Some('\u{8}'),
            't' => Some('\t'),
            'n' => Some('\n'),
            'f' => Some('\u{c}'),
            'r' => Some('\r'),
            '"' => Some('"'),
            '\\' => Some('\\'),
            _ => None,
        };
        if let Some(ch) = simple {
            return Ok(ch);
        }
        let digits = match c {
            'u' => 4,
            'U' => 8,
            _ => return self.fail(SyntaxError::InvalidEscape),
        };
        let mut code: u32 = 0;
        for _ in 0..digits {
            let Some(d) = self.peek().and_then(|h| h.to_digit(16)) else {
                return self.fail(SyntaxError::InvalidEscape);
            };
            self.bump();
            code = code * 16 + d;
        }
        match char::from_u32(code) {
            Some(ch) => Ok(ch),
            None => self.fail(SyntaxError::InvalidEscape),
        }
    }

    /// Spaces, line endings and comments inside an array.
    fn skip_array_space(&mut self) -> PResult<()> {
        loop {
            match self.peek() {
                Some(' ' | '\t' | '\n' | '\r') => {
                    self.bump();
                }
                Some('#') => self.skip_comment()?,
                _ => return Ok(()),
            }
        }
    }

    fn array(&mut self) -> PResult<Value> {
        let start = self.span();
        self.bump(); // '['
        let mut items = Vec::new();
        loop {
            self.skip_array_space()?;
            match self.peek() {
                None => {
                    return Err(ParseError {
                        span: start,
                        error: SyntaxError::UnterminatedArray,
                    });
                }
                Some(']') => {
                    self.bump();
                    return Ok(Value::Array(items));
                }
                Some('"' | '\'') => {
                    if items.len() == MAX_ARRAY_ITEMS {
                        return self.fail(SyntaxError::ArrayTooLong);
                    }
                    let span = self.span();
                    items.push((self.string()?, span));
                }
                Some(_) => return self.fail(SyntaxError::InvalidArrayItem),
            }
            self.skip_array_space()?;
            match self.peek() {
                Some(',') => {
                    self.bump();
                }
                Some(']') => {}
                None => {
                    return Err(ParseError {
                        span: start,
                        error: SyntaxError::UnterminatedArray,
                    });
                }
                Some(_) => return self.fail(SyntaxError::InvalidArrayItem),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn err(input: &str) -> SyntaxError {
        parse(input).expect_err("input should be rejected").error
    }

    fn at(input: &str) -> (usize, usize, SyntaxError) {
        let e = parse(input).expect_err("input should be rejected");
        (e.span.line, e.span.column, e.error)
    }

    #[test]
    fn parses_the_whole_subset() {
        let doc = parse(
            "\u{feff}# comment\r\nversion = 1\n\n[server] # trailing\n\
             name = \"a \\\"b\\\" \\u00e9\"\nraw = 'C:\\path'\nflag = false\n\
             neg = -42\nlist = [\n  \"x\", # one\n  'y',\n]\nempty = []\n",
        )
        .expect("valid document");
        assert_eq!(
            doc.root.get("version").map(|e| &e.value),
            Some(&Value::Integer(1))
        );
        let server = doc.table("server").expect("server table");
        assert_eq!(server.span, Span { line: 4, column: 1 });
        assert_eq!(
            server.get("name").map(|e| &e.value),
            Some(&Value::String("a \"b\" \u{e9}".into()))
        );
        assert_eq!(
            server.get("raw").map(|e| &e.value),
            Some(&Value::String("C:\\path".into()))
        );
        assert_eq!(
            server.get("flag").map(|e| &e.value),
            Some(&Value::Boolean(false))
        );
        assert_eq!(
            server.get("neg").map(|e| &e.value),
            Some(&Value::Integer(-42))
        );
        let Some(Value::Array(items)) = server.get("list").map(|e| &e.value) else {
            panic!("list should be an array");
        };
        let texts: Vec<&str> = items.iter().map(|(s, _)| s.as_str()).collect();
        assert_eq!(texts, ["x", "y"]);
        assert_eq!(
            items[1].1,
            Span {
                line: 11,
                column: 3
            }
        );
        assert_eq!(
            server.get("empty").map(|e| &e.value),
            Some(&Value::Array(vec![]))
        );
    }

    #[test]
    fn empty_and_comment_only_documents_are_valid() {
        assert_eq!(parse("").expect("empty").tables.len(), 0);
        assert_eq!(
            parse("# only\n\n   \n")
                .expect("comments")
                .root
                .entries
                .len(),
            0
        );
    }

    #[test]
    fn reports_positions() {
        assert_eq!(at("a = 1\nb = @\n"), (2, 5, SyntaxError::ExpectedValue));
        assert_eq!(at("a = 1 2\n"), (1, 7, SyntaxError::TrailingCharacters));
        assert_eq!(at("[t]\nx = 1\nx = 2\n"), (3, 1, SyntaxError::DuplicateKey));
        assert_eq!(at("a b = 1"), (1, 3, SyntaxError::ExpectedEquals));
    }

    #[test]
    fn rejects_duplicates_and_clashes() {
        assert_eq!(err("[a]\n[a]\n"), SyntaxError::DuplicateTable);
        assert_eq!(err("a = 1\n[a]\n"), SyntaxError::DuplicateTable);
        assert_eq!(err("a = 1\na = 1\n"), SyntaxError::DuplicateKey);
    }

    #[test]
    fn rejects_unsupported_toml_clearly() {
        let cases = [
            ("a.b = 1", Unsupported::DottedName),
            ("[a.b]", Unsupported::DottedName),
            ("\"a\" = 1", Unsupported::QuotedName),
            ("[\"a\"]", Unsupported::QuotedName),
            ("[[a]]", Unsupported::ArrayOfTables),
            ("a = { b = 1 }", Unsupported::InlineTable),
            ("a = \"\"\"x\"\"\"", Unsupported::MultiLineString),
            ("a = '''x'''", Unsupported::MultiLineString),
            ("a = 1.5", Unsupported::Float),
            ("a = 1e3", Unsupported::Float),
            ("a = inf", Unsupported::Float),
            ("a = nan", Unsupported::Float),
            ("a = 2026-10-16", Unsupported::DateTime),
            ("a = 07:30:00", Unsupported::DateTime),
            ("a = 0x1F", Unsupported::IntegerFormat),
            ("a = 1_000", Unsupported::IntegerFormat),
        ];
        for (input, what) in cases {
            assert_eq!(err(input), SyntaxError::Unsupported(what), "{input}");
        }
    }

    #[test]
    fn rejects_malformed_values() {
        let cases = [
            ("a = ", SyntaxError::ExpectedValue),
            ("a = yes", SyntaxError::ExpectedValue),
            ("a = truex", SyntaxError::ExpectedValue),
            ("a = 012", SyntaxError::InvalidInteger),
            ("a = 99999999999999999999", SyntaxError::InvalidInteger),
            ("a = -", SyntaxError::InvalidInteger),
            ("a = \"open", SyntaxError::UnterminatedString),
            ("a = \"line\nbreak\"", SyntaxError::UnterminatedString),
            ("a = \"bad \\q\"", SyntaxError::InvalidEscape),
            ("a = \"bad \\u12\"", SyntaxError::InvalidEscape),
            ("a = \"surrogate \\uD800\"", SyntaxError::InvalidEscape),
            ("a = [1, 2]", SyntaxError::InvalidArrayItem),
            ("a = [\"x\" \"y\"]", SyntaxError::InvalidArrayItem),
            ("a = [\"x\",", SyntaxError::UnterminatedArray),
            ("a = [", SyntaxError::UnterminatedArray),
            ("= 1", SyntaxError::ExpectedKey),
            ("[", SyntaxError::ExpectedKey),
            ("[a", SyntaxError::ExpectedKey),
            ("[a] x", SyntaxError::TrailingCharacters),
        ];
        for (input, expected) in cases {
            assert_eq!(err(input), expected, "{input:?}");
        }
    }

    #[test]
    fn rejects_control_characters_and_bad_line_endings() {
        assert_eq!(err("a = \"x\u{1}\""), SyntaxError::ControlCharacter);
        assert_eq!(err("a = 'x\u{7f}'"), SyntaxError::ControlCharacter);
        assert_eq!(err("# bell \u{7}\n"), SyntaxError::ControlCharacter);
        assert_eq!(err("a = 1\rb = 2"), SyntaxError::BareCarriageReturn);
        assert_eq!(err("\u{0}"), SyntaxError::ExpectedKey);
    }

    #[test]
    fn enforces_limits() {
        assert_eq!(
            err(&"#".repeat(MAX_LINE_CHARS + 1)),
            SyntaxError::LineTooLong
        );
        let big = "a = 1\n".repeat(MAX_INPUT_BYTES / 6 + 1);
        assert_eq!(err(&big), SyntaxError::InputTooLarge);
        let long = format!("a = \"{}\"", "x".repeat(MAX_STRING_CHARS + 1));
        assert_eq!(err(&long), SyntaxError::StringTooLong);
        let name = format!("{} = 1", "k".repeat(MAX_NAME_CHARS + 1));
        assert_eq!(err(&name), SyntaxError::NameTooLong);
        let items = vec!["\"x\""; MAX_ARRAY_ITEMS + 1].join(",\n");
        assert_eq!(
            err(&format!("a = [\n{items}\n]")),
            SyntaxError::ArrayTooLong
        );
        let tables = (0..=MAX_TABLES)
            .map(|i| format!("[t{i}]\n"))
            .collect::<Vec<_>>()
            .concat();
        assert_eq!(err(&tables), SyntaxError::TooManyTables);
        let keys = (0..=MAX_KEYS_PER_TABLE)
            .map(|i| format!("k{i} = 1\n"))
            .collect::<Vec<_>>()
            .concat();
        assert_eq!(err(&keys), SyntaxError::TooManyKeys);
    }

    #[test]
    fn rejects_invalid_utf8_with_line() {
        let e = parse_bytes(b"a = 1\nb = \"\xff\"\n").expect_err("not UTF-8");
        assert_eq!((e.span.line, e.error), (2, SyntaxError::NotUtf8));
        let big = vec![b'#'; MAX_INPUT_BYTES + 1];
        assert_eq!(
            parse_bytes(&big).map_err(|e| e.error),
            Err(SyntaxError::InputTooLarge)
        );
    }

    /// Deterministic mutations of a valid document must never panic:
    /// every result is either a document or an error.
    #[test]
    fn mutated_inputs_never_panic() {
        let base = "version = 1\n[a]\nb = \"text \\u00e9\"\nc = [\n 'x', # c\n \"y\",\n]\nd = -7\ne = true\n";
        let bytes = base.as_bytes();
        let replacements: &[u8] = b"\0\r\n\t \"'[]{}=#,.\\-+0e_\xff\xc3";
        for cut in 0..=bytes.len() {
            let _ = parse_bytes(&bytes[..cut]);
        }
        for i in 0..bytes.len() {
            for &r in replacements {
                let mut m = bytes.to_vec();
                m[i] = r;
                let _ = parse_bytes(&m);
                m.insert(i, r);
                let _ = parse_bytes(&m);
            }
        }
    }
}
