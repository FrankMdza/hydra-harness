//! How a tool call's input is shortened for the cockpit detail, build ticket 206.

use serde_json::Value;

/// How long a full input stays on screen after `i`, build ticket 206.
pub const REVEAL_SECONDS: u64 = 10;

const INPUT_KEYS: &[&str] = &[
    "command",
    "cmd",
    "file_path",
    "path",
    "notebook_path",
    "pattern",
    "query",
    "searchTerm",
    "url",
    "description",
    "prompt",
];

const SHELL_TOOLS: &[&str] = &["Bash", "Shell", "shell", "run_terminal_cmd"];

/// A call that carries one of these is a shell call whatever its tool is named.
const COMMAND_KEYS: &[&str] = &["command", "cmd"];

/// The arguments a row may show as they are: where a tool reads or what it looks for.
const SHOWN_KEYS: &[&str] = &["file_path", "path", "notebook_path", "pattern"];

/// Programs whose second word is a subcommand. For any other program the second word is an
/// argument, and an argument can be a secret (`echo hunter2`), so it stays behind the key.
const SUBCOMMAND_PROGRAMS: &[&str] = &[
    "git",
    "cargo",
    "docker",
    "npm",
    "pnpm",
    "yarn",
    "kubectl",
    "aws",
    "gh",
    "hydra",
    "go",
    "brew",
    "pip",
    "pip3",
    "uv",
    "rustup",
    "systemctl",
    "apt",
    "apt-get",
    "terraform",
    "gcloud",
    "az",
    "herdr",
];

/// The text a call row shows and the full text the reveal key may open.
pub fn call_inputs(name: &str, arguments: &Value) -> (String, String) {
    let full = full_input(arguments);
    if let Some(command) = COMMAND_KEYS
        .iter()
        .find_map(|key| argument_text(arguments, key))
    {
        return (shell_head(&command), full);
    }
    if SHELL_TOOLS.contains(&name) {
        return (shell_head(&full), full);
    }
    // A URL, a query, a prompt or a description can carry anything, so only the tool's
    // name shows for those, and for a tool this does not know.
    match SHOWN_KEYS
        .iter()
        .find_map(|key| argument_text(arguments, key))
    {
        Some(arg) => (format!("{name} {arg}"), full),
        None if full.is_empty() => (name.to_string(), full),
        None => (format!("{name} ..."), full),
    }
}

/// The program and at most one bare subcommand, with `...` when the rest is left out.
pub fn shell_head(command: &str) -> String {
    let command = command.split_whitespace().collect::<Vec<_>>().join(" ");
    if command.is_empty() {
        return String::new();
    }
    let piped = command_has_list(&command);
    let segment = shell_segments(&command)
        .into_iter()
        .next()
        .unwrap_or_default();
    let mut skipped = false;
    let mut i = 0usize;
    while i < segment.len() {
        let tok = &segment[i];
        if is_assignment(tok) || matches!(tok.as_str(), "sudo" | "env" | "time") {
            skipped = true;
            i += 1;
            continue;
        }
        break;
    }
    let mut parts: Vec<String> = Vec::new();
    if skipped {
        parts.push("env".into());
        parts.push("...".into());
    }
    if i >= segment.len() {
        let mut head = parts.join(" ");
        if piped {
            head.push_str(" ...");
        }
        return head;
    }
    let program = program_base(&segment[i]);
    let takes_subcommand = SUBCOMMAND_PROGRAMS.contains(&program.as_str());
    parts.push(program);
    i += 1;
    let mut truncated = piped;
    if i < segment.len() {
        let tok = &segment[i];
        let shell = matches!(
            parts.last().map(String::as_str),
            Some("bash" | "sh" | "zsh" | "dash" | "fish")
        );
        if (takes_subcommand && is_bare_subcommand(tok)) || (shell && is_script_flag(tok)) {
            parts.push(tok.clone());
            i += 1;
        } else {
            truncated = true;
        }
    }
    if i < segment.len() {
        truncated = true;
    }
    let mut head = parts.join(" ");
    if truncated {
        head.push_str(" ...");
    }
    head
}

fn full_input(arguments: &Value) -> String {
    let named = INPUT_KEYS
        .iter()
        .find_map(|key| argument_text(arguments, key));
    named.unwrap_or_else(|| match arguments {
        Value::Null => String::new(),
        other => other.to_string(),
    })
}

fn argument_text(arguments: &Value, key: &str) -> Option<String> {
    match &arguments[key] {
        Value::String(text) => Some(text.clone()),
        Value::Array(parts) if !parts.is_empty() => Some(
            parts
                .iter()
                .map(|part| {
                    part.as_str()
                        .map_or_else(|| part.to_string(), str::to_string)
                })
                .collect::<Vec<_>>()
                .join(" "),
        ),
        _ => None,
    }
}

fn program_base(token: &str) -> String {
    if token.contains("://") {
        return token.split("://").next().unwrap_or(token).to_string();
    }
    let path = token.rsplit(['/', '\\']).next().unwrap_or(token);
    if path.contains('@') || path.contains(':') {
        return path.split(['@', ':']).next().unwrap_or(path).to_string();
    }
    path.to_string()
}

fn is_assignment(token: &str) -> bool {
    let Some((name, _)) = token.split_once('=') else {
        return false;
    };
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.'))
}

fn is_bare_subcommand(token: &str) -> bool {
    !token.is_empty()
        && !token.starts_with('-')
        && token
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !token
            .chars()
            .any(|c| matches!(c, '=' | '/' | '.' | ':' | '@' | '\'' | '"' | '$'))
}

fn is_script_flag(token: &str) -> bool {
    token == "-c" || token == "-lc"
}

fn command_has_list(command: &str) -> bool {
    let mut quote = None;
    let bytes = command.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let ch = bytes[i] as char;
        if quote == Some('\'') {
            if ch == '\'' {
                quote = None;
            }
            i += 1;
            continue;
        }
        if quote == Some('"') {
            if ch == '"' {
                quote = None;
            }
            i += 1;
            continue;
        }
        if ch == '\'' {
            quote = Some('\'');
            i += 1;
            continue;
        }
        if ch == '"' {
            quote = Some('"');
            i += 1;
            continue;
        }
        if ch == '|' {
            return true;
        }
        if ch == ';' {
            return true;
        }
        if ch == '&' && i + 1 < bytes.len() && bytes[i + 1] == b'&' {
            return true;
        }
        if ch == '|' && i + 1 < bytes.len() && bytes[i + 1] == b'|' {
            return true;
        }
        i += 1;
    }
    false
}

fn shell_segments(command: &str) -> Vec<Vec<String>> {
    let mut segments: Vec<Vec<String>> = Vec::new();
    let mut current: Vec<String> = Vec::new();
    let mut word = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut i = 0;
    let bytes = command.as_bytes();
    while i < bytes.len() {
        let ch = bytes[i] as char;
        if in_single {
            if ch == '\'' {
                in_single = false;
            } else {
                word.push(ch);
            }
            i += 1;
            continue;
        }
        if in_double {
            if ch == '"' {
                in_double = false;
            } else {
                word.push(ch);
            }
            i += 1;
            continue;
        }
        if ch == '\'' {
            in_single = true;
            i += 1;
            continue;
        }
        if ch == '"' {
            in_double = true;
            i += 1;
            continue;
        }
        if ch.is_whitespace() {
            if !word.is_empty() {
                current.push(word.clone());
                word.clear();
            }
            i += 1;
            continue;
        }
        if matches!(ch, ';' | '|' | '&' | '\n') {
            let doubled =
                matches!(ch, '|' | '&') && i + 1 < bytes.len() && bytes[i + 1] as char == ch;
            i += if doubled { 2 } else { 1 };
            if !word.is_empty() {
                current.push(word.clone());
                word.clear();
            }
            if !current.is_empty() {
                segments.push(current);
                current = Vec::new();
            }
            continue;
        }
        word.push(ch);
        i += 1;
    }
    if !word.is_empty() {
        current.push(word);
    }
    if !current.is_empty() {
        segments.push(current);
    }
    segments
}

/// A raw record line with tool inputs shortened the same way as the call rows.
pub fn tail_line_shown(line: &str, max_chars: usize) -> String {
    if line.contains("\"secret_flag\":[") {
        return "[line hidden: secret_flag]".to_string();
    }
    let truncated = line.chars().nth(max_chars).is_some();
    let shown = abbreviate_line(line);
    let mut cut: String = shown.chars().take(max_chars).collect();
    if cut.len() < shown.len() || truncated {
        cut.push_str(" [cut]");
    }
    cut
}

fn abbreviate_line(line: &str) -> String {
    let Ok(mut value) = serde_json::from_str::<Value>(line) else {
        return line.to_string();
    };
    abbreviate_value(&mut value);
    value.to_string()
}

fn abbreviate_value(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for (key, child) in map.iter_mut() {
                if INPUT_KEYS.contains(&key.as_str()) {
                    if let Value::String(text) = child {
                        *text = shell_head(text);
                    }
                } else {
                    abbreviate_value(child);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                abbreviate_value(item);
            }
        }
        _ => {}
    }
}
