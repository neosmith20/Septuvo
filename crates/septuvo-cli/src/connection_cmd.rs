//! `septuvo connection …` and `septuvo connect NAME` (N4-03): the saved
//! connections the service keeps.
//!
//! The tool only asks the service; the service validates every
//! configuration (with `septuvo-config`) and owns the store. Connections
//! are named by their ID (`c0001`) or their name (case does not matter).

use crate::service::{Reply, call, reply};
use septuvo_ipc::{ClientError, Command, ConnectionId, Outcome, Request, Response};
use std::fmt::Write as _;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Largest file `add` and `replace` read (the protocol's body limit).
pub const MAX_FILE_BYTES: u64 = septuvo_ipc::MAX_BODY_BYTES as u64;

/// Usage text for the `connection` commands.
pub const USAGE: &str = "\
  septuvo connection list                    Saved connections (* = selected)
  septuvo connection show CONN               A connection's configuration
  septuvo connection export CONN FILE        Save it to a new FILE (re-importable)
  septuvo connection add FILE [--name NAME]  Save a complete configuration
  septuvo connection replace CONN FILE       Replace a connection's configuration
  septuvo connection rename CONN NAME        Rename (no network change)
  septuvo connection delete CONN             Delete (disconnect it first)
  septuvo connection select CONN             Use it; if the kill switch is on,
                                             switch to it without a gap
  septuvo connect CONN                       Select it and connect
                                             (each also takes --runtime-dir DIR)
  CONN is an ID such as c0001, or a name.";

/// A parsed `connection` command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// `list`.
    List,
    /// `show CONN`.
    Show(String),
    /// `export CONN FILE`.
    Export(String, PathBuf),
    /// `add FILE [--name NAME]`.
    Add(PathBuf, Option<String>),
    /// `replace CONN FILE`.
    Replace(String, PathBuf),
    /// `rename CONN NAME`.
    Rename(String, String),
    /// `delete CONN`.
    Delete(String),
    /// `select CONN`.
    Select(String),
    /// `connect CONN`.
    Connect(String),
}

/// Splits off `--runtime-dir DIR` (once, absolute) from `args`.
fn take_runtime_dir(args: &[String]) -> Option<(Vec<String>, PathBuf)> {
    let mut rest = Vec::new();
    let mut dir = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == "--runtime-dir" {
            let d = it.next()?;
            if dir.is_some() || !Path::new(d).is_absolute() {
                return None;
            }
            dir = Some(PathBuf::from(d));
        } else {
            rest.push(a.clone());
        }
    }
    Some((
        rest,
        dir.unwrap_or_else(|| PathBuf::from(septuvo_ipc::DEFAULT_RUNTIME_DIR)),
    ))
}

/// Parses the arguments after `septuvo connection` (or, with `connect`,
/// after `septuvo connect`).
#[must_use]
pub fn parse(args: &[String], connect: bool) -> Option<(Action, PathBuf)> {
    let (rest, dir) = take_runtime_dir(args)?;
    let s: Vec<&str> = rest.iter().map(String::as_str).collect();
    let action = if connect {
        match s.as_slice() {
            [conn] => Action::Connect((*conn).to_string()),
            _ => return None,
        }
    } else {
        match s.as_slice() {
            ["list"] => Action::List,
            ["show", c] => Action::Show((*c).to_string()),
            ["export", c, f] => Action::Export((*c).to_string(), PathBuf::from(f)),
            ["add", f] => Action::Add(PathBuf::from(f), None),
            ["add", f, "--name", n] | ["add", "--name", n, f] => {
                Action::Add(PathBuf::from(f), Some((*n).to_string()))
            }
            ["replace", c, f] => Action::Replace((*c).to_string(), PathBuf::from(f)),
            ["rename", c, n] => Action::Rename((*c).to_string(), (*n).to_string()),
            ["delete", c] => Action::Delete((*c).to_string()),
            ["select", c] => Action::Select((*c).to_string()),
            _ => return None,
        }
    };
    Some((action, dir))
}

/// `text` as a TOML basic string, quotes included.
#[must_use]
pub fn toml_string(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if c.is_control() => {
                let _ = write!(out, "\\u{:04X}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// True if the file already has a `[connection]` table.
fn has_name_table(text: &str) -> bool {
    text.lines().any(|l| {
        let l = l.trim();
        l == "[connection]"
            || l.starts_with("[connection]") && l[12..].trim_start().starts_with('#')
    })
}

/// The `ID<TAB>name` lines of a `LIST` body.
#[must_use]
pub fn parse_list(body: &str) -> Vec<(ConnectionId, String)> {
    body.lines()
        .filter_map(|l| {
            let (id, name) = l.split_once('\t')?;
            Some((ConnectionId::parse(id)?, name.to_string()))
        })
        .collect()
}

/// Finds a connection by ID or name (case-insensitive) in a list.
#[must_use]
pub fn find(list: &[(ConnectionId, String)], conn: &str) -> Option<ConnectionId> {
    if let Some(id) = ConnectionId::parse(conn) {
        return list.iter().any(|(i, _)| *i == id).then_some(id);
    }
    let wanted = conn.trim().to_lowercase();
    list.iter()
        .find(|(_, n)| n.to_lowercase() == wanted)
        .map(|(i, _)| *i)
}

/// Text from the service, made safe for a terminal: no control
/// characters other than tab and line feed.
fn safe(text: &str) -> String {
    text.chars()
        .filter(|&c| !c.is_control() || c == '\n' || c == '\t')
        .collect()
}

fn read_file(path: &Path) -> Result<String, String> {
    let meta = fs::metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if !meta.is_file() || meta.len() > MAX_FILE_BYTES {
        return Err(format!(
            "{}: must be a regular file of at most {MAX_FILE_BYTES} bytes",
            path.display()
        ));
    }
    let mut text = String::new();
    fs::File::open(path)
        .and_then(|f| f.take(MAX_FILE_BYTES + 1).read_to_string(&mut text))
        .map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(text)
}

/// Runs a connection command. Returns `(success, text to print)`.
#[must_use]
pub fn run(action: &Action, runtime_dir: &Path) -> (bool, String) {
    match execute(action, runtime_dir) {
        Ok(text) => (true, text),
        Err(text) => (false, text),
    }
}

fn service_error(dir: &Path, e: ClientError) -> String {
    let Reply { text, .. } = reply(dir, Err(e));
    text
}

/// The response, or its error as text.
fn ok(dir: &Path, answer: Result<Response, ClientError>) -> Result<Response, String> {
    let r = answer.map_err(|e| service_error(dir, e))?;
    match &r.outcome {
        Outcome::Ok => Ok(r),
        Outcome::Err { code, message } => Err(format!("Error ({code}): {message}\n")),
    }
}

fn resolve(dir: &Path, conn: &str) -> Result<ConnectionId, String> {
    let list = ok(dir, call(dir, &Request::new(Command::List)))?;
    find(&parse_list(list.body.as_deref().unwrap_or_default()), conn).ok_or_else(|| {
        format!(
            "No saved connection called '{}'. See: septuvo connection list\n",
            safe(conn).chars().take(64).collect::<String>()
        )
    })
}

fn status_text(dir: &Path, r: Response) -> String {
    let reply = reply(dir, Ok(r));
    format!("{}{}", reply.status.report(None), reply.text)
}

fn execute(action: &Action, dir: &Path) -> Result<String, String> {
    match action {
        Action::List => {
            let r = ok(dir, call(dir, &Request::new(Command::List)))?;
            let selected = r.field("selected").and_then(ConnectionId::parse);
            let list = parse_list(r.body.as_deref().unwrap_or_default());
            if list.is_empty() {
                return Ok(
                    "No saved connections. Add one with: septuvo connection add FILE --name NAME\n"
                        .into(),
                );
            }
            let mut s = String::new();
            for (id, name) in list {
                let mark = if Some(id) == selected { '*' } else { ' ' };
                let _ = writeln!(s, "{mark} {id}  {}", safe(&name));
            }
            Ok(s)
        }
        Action::Show(c) => {
            let id = resolve(dir, c)?;
            let r = ok(dir, call(dir, &Request::new(Command::Get(id))))?;
            Ok(safe(r.body.as_deref().unwrap_or_default()))
        }
        Action::Export(c, file) => {
            let id = resolve(dir, c)?;
            let r = ok(dir, call(dir, &Request::new(Command::Get(id))))?;
            // Never overwrites: a new file only.
            let mut f = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(file)
                .map_err(|e| format!("{}: {e}\n", file.display()))?;
            f.write_all(r.body.as_deref().unwrap_or_default().as_bytes())
                .map_err(|e| format!("{}: {e}\n", file.display()))?;
            Ok(format!("Exported {id} to {}\n", file.display()))
        }
        Action::Add(file, name) => {
            let mut text = read_file(file)?;
            if let Some(name) = name {
                if has_name_table(&text) {
                    return Err("The file already names the connection ([connection] table); leave out --name.\n".into());
                }
                let _ = write!(text, "\n[connection]\nname = {}\n", toml_string(name));
            }
            let r = ok(dir, call(dir, &Request::with_body(Command::Add, text)))?;
            Ok(format!(
                "Saved as {} (not selected).\n",
                r.field("id").unwrap_or("?")
            ))
        }
        Action::Replace(c, file) => {
            let id = resolve(dir, c)?;
            let text = read_file(file)?;
            let r = ok(
                dir,
                call(dir, &Request::with_body(Command::Replace(id), text)),
            )?;
            Ok(status_text(dir, r))
        }
        Action::Rename(c, name) => {
            let id = resolve(dir, c)?;
            ok(
                dir,
                call(dir, &Request::with_body(Command::Rename(id), name.as_str())),
            )?;
            Ok(format!("Renamed {id}.\n"))
        }
        Action::Delete(c) => {
            let id = resolve(dir, c)?;
            let r = ok(dir, call(dir, &Request::new(Command::Delete(id))))?;
            Ok(status_text(dir, r))
        }
        Action::Select(c) => {
            let id = resolve(dir, c)?;
            let r = ok(dir, call(dir, &Request::new(Command::Select(id))))?;
            Ok(status_text(dir, r))
        }
        Action::Connect(c) => {
            let id = resolve(dir, c)?;
            let r = ok(dir, call(dir, &Request::new(Command::Select(id))))?;
            // Already connected: selecting switched. Otherwise connect now.
            if matches!(r.field("state"), Some("blocking" | "failed")) {
                return Ok(status_text(dir, r));
            }
            let r = ok(dir, call(dir, &Request::new(Command::Connect)))?;
            Ok(status_text(dir, r))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(s: &[&str]) -> Vec<String> {
        s.iter().map(|a| (*a).to_string()).collect()
    }

    #[test]
    fn arguments() {
        let default = PathBuf::from(septuvo_ipc::DEFAULT_RUNTIME_DIR);
        assert_eq!(
            parse(&args(&["list"]), false),
            Some((Action::List, default.clone()))
        );
        assert_eq!(
            parse(
                &args(&[
                    "add",
                    "f.toml",
                    "--name",
                    "Home VPN",
                    "--runtime-dir",
                    "/tmp/r"
                ]),
                false
            ),
            Some((
                Action::Add("f.toml".into(), Some("Home VPN".into())),
                PathBuf::from("/tmp/r")
            ))
        );
        assert_eq!(
            parse(&args(&["Home"]), true),
            Some((Action::Connect("Home".into()), default))
        );
        for bad in [
            &["list", "x"][..],
            &["add"],
            &["rename", "c0001"],
            &["select", "a", "b"],
            &["list", "--runtime-dir", "relative"],
            &["list", "--runtime-dir"],
            &["list", "--runtime-dir", "/a", "--runtime-dir", "/b"],
            &["frobnicate"],
        ] {
            assert_eq!(parse(&args(bad), false), None, "{bad:?}");
        }
        assert_eq!(parse(&args(&[]), true), None);
    }

    #[test]
    fn names_are_quoted_for_toml() {
        assert_eq!(toml_string("Home VPN"), "\"Home VPN\"");
        assert_eq!(toml_string("a\"b\\c"), "\"a\\\"b\\\\c\"");
        assert_eq!(toml_string("é\u{7}"), "\"é\\u0007\"");
        assert!(has_name_table("x = 1\n[connection]\nname = \"a\"\n"));
        assert!(has_name_table("  [connection]  # named\n"));
        assert!(!has_name_table("[connections]\n# [connection]\n"));
    }

    #[test]
    fn lists_and_lookups() {
        let list = parse_list("c0001\tHome VPN\nc0002\tBüro\nbad line\nc0000\tx\n");
        assert_eq!(list.len(), 2);
        assert_eq!(
            find(&list, "c0002").map(|i| i.to_string()).as_deref(),
            Some("c0002")
        );
        assert_eq!(find(&list, "home vpn").map(ConnectionId::number), Some(1));
        assert_eq!(find(&list, "BÜRO").map(ConnectionId::number), Some(2));
        assert_eq!(find(&list, "c0009"), None, "an ID must exist");
        assert_eq!(find(&list, "Office"), None);
    }

    #[test]
    fn service_text_is_safe_for_a_terminal() {
        assert_eq!(safe("ok\u{1b}[2J\tname\n"), "ok[2J\tname\n");
    }
}
