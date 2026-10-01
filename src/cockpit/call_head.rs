//! How a tool call's input is shortened for the cockpit detail, build ticket 206.

use serde_json::Value;

/// How long a full input stays on screen after `i`, build ticket 206.
pub const REVEAL_SECONDS: u64 = 10;

const INPUT_KEYS: &[&str] = &[
    "command",
    "cmd",
    "CommandLine",
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
const COMMAND_KEYS: &[&str] = &["command", "cmd", "CommandLine"];

/// The arguments a row may show as they are: where a tool reads or what it looks for.
const SHOWN_KEYS: &[&str] = &["file_path", "path", "notebook_path"];

/// Programs whose second word is a subcommand. For any other program the second word is an
/// argument, and an argument can be a secret (`echo hunter2`), so it stays behind the key.
// Deliberately short lists, not aliases or user-defined commands. Unknown words stay hidden.
fn subcommands(program: &str) -> &'static [&'static str] {
    match program {
        "git" => &[
            "add", "commit", "push", "pull", "clone", "status", "diff", "log", "checkout",
            "switch", "branch", "merge", "rebase", "fetch", "stash", "apply", "worktree", "show",
            "reset", "restore", "tag", "remote", "init",
        ],
        "cargo" => &[
            "build", "check", "test", "run", "fmt", "clippy", "doc", "clean", "new", "init", "add",
            "remove", "update", "fetch", "metadata", "tree", "bench", "publish", "install",
        ],
        "docker" => &[
            "run",
            "exec",
            "ps",
            "build",
            "pull",
            "push",
            "images",
            "compose",
            "container",
            "image",
            "network",
            "volume",
            "inspect",
            "logs",
            "start",
            "stop",
            "rm",
        ],
        "npm" => &[
            "install",
            "ci",
            "run",
            "test",
            "build",
            "init",
            "audit",
            "publish",
            "uninstall",
            "update",
            "exec",
            "outdated",
            "list",
            "ls",
            "cache",
            "config",
        ],
        "pnpm" => &[
            "add", "install", "remove", "update", "run", "test", "exec", "dlx", "publish", "init",
            "list", "audit",
        ],
        "yarn" => &[
            "add", "install", "remove", "run", "test", "init", "publish", "upgrade", "info",
            "list", "cache", "config",
        ],
        "kubectl" => &[
            "get",
            "describe",
            "apply",
            "delete",
            "create",
            "edit",
            "logs",
            "exec",
            "config",
            "rollout",
            "scale",
            "port-forward",
            "diff",
            "patch",
            "version",
        ],
        "aws" => &[
            "configure",
            "s3",
            "s3api",
            "ec2",
            "ecs",
            "eks",
            "iam",
            "sts",
            "lambda",
            "cloudformation",
            "logs",
            "help",
        ],
        "gh" => &[
            "auth",
            "pr",
            "issue",
            "repo",
            "run",
            "workflow",
            "release",
            "api",
            "gist",
            "search",
            "status",
            "project",
            "extension",
        ],
        "hydra" => &["run", "show", "status", "logs", "doctor", "reap", "cockpit"],
        "go" => &[
            "build", "test", "run", "fmt", "vet", "mod", "get", "install", "clean", "env",
            "version", "list", "work", "generate",
        ],
        "brew" => &[
            "install",
            "uninstall",
            "update",
            "upgrade",
            "list",
            "info",
            "search",
            "doctor",
            "config",
            "help",
            "create",
            "edit",
        ],
        "pip" | "pip3" => &[
            "install",
            "uninstall",
            "download",
            "freeze",
            "list",
            "show",
            "check",
            "config",
            "cache",
            "wheel",
            "help",
        ],
        "uv" => &[
            "run", "init", "add", "remove", "sync", "lock", "tree", "tool", "python", "pip",
            "venv", "build", "publish", "cache", "help",
        ],
        "rustup" => &[
            "install",
            "uninstall",
            "toolchain",
            "default",
            "show",
            "update",
            "check",
            "target",
            "component",
            "override",
            "run",
            "which",
            "doc",
            "self",
            "set",
            "help",
        ],
        "systemctl" => &[
            "start",
            "stop",
            "restart",
            "reload",
            "status",
            "enable",
            "disable",
            "list-units",
            "daemon-reload",
            "is-active",
            "is-enabled",
        ],
        "apt" | "apt-get" => &[
            "update",
            "upgrade",
            "install",
            "remove",
            "purge",
            "autoremove",
            "clean",
            "autoclean",
            "check",
            "download",
        ],
        "terraform" => &[
            "init",
            "plan",
            "apply",
            "destroy",
            "fmt",
            "validate",
            "show",
            "output",
            "state",
            "import",
            "workspace",
            "version",
        ],
        "gcloud" => &[
            "auth",
            "config",
            "compute",
            "container",
            "run",
            "storage",
            "projects",
            "iam",
            "functions",
            "version",
            "help",
        ],
        "az" => &[
            "login",
            "logout",
            "account",
            "group",
            "vm",
            "aks",
            "storage",
            "network",
            "keyvault",
            "deployment",
            "version",
        ],
        "herdr" => &[
            "session",
            "completion",
            "update",
            "channel",
            "server",
            "api",
            "config",
            "workspace",
            "worktree",
            "tab",
            "notification",
            "agent",
            "pane",
            "integration",
            "status",
        ],
        _ => &[],
    }
}

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
    let token = &segment[i];
    if skipped && token.starts_with('-') {
        return "env ...".into();
    }
    if token.chars().any(|c| {
        c.is_whitespace()
            || matches!(
                c,
                '\'' | '"' | '$' | '`' | '(' | ')' | '<' | '>' | '{' | '}' | '=' | '!'
            )
    }) {
        return if skipped { "env ..." } else { "..." }.into();
    }
    let program = program_base(token);
    let allowed = subcommands(&program);
    parts.push(program);
    i += 1;
    let mut truncated = piped;
    if i < segment.len() {
        let tok = &segment[i];
        let shell = matches!(
            parts.last().map(String::as_str),
            Some("bash" | "sh" | "zsh" | "dash" | "fish")
        );
        if (allowed.contains(&tok.as_str()) && is_bare_subcommand(tok))
            || (shell && is_script_flag(tok))
        {
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
        // One `&` ends a command as `&&` does, and `&>` is a redirection whose target is
        // left out: either way something after the head is not shown.
        if ch == '&' {
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
            }
            word.push(ch);
            i += 1;
            continue;
        }
        if in_double {
            if ch == '"' {
                in_double = false;
            }
            word.push(ch);
            i += 1;
            continue;
        }
        if ch == '\'' {
            word.push(ch);
            in_single = true;
            i += 1;
            continue;
        }
        if ch == '"' {
            word.push(ch);
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

/// A raw record line exposes metadata only until the explicit reveal key.
pub fn tail_line_shown(line: &str, max_chars: usize) -> String {
    if line.starts_with("[line hidden:") {
        return line.into();
    }
    let Ok(value) = serde_json::from_str::<Value>(line) else {
        return format!("not JSON, {} bytes", line.len());
    };
    if value["secret_flag"].is_array() {
        return "[line hidden: secret_flag]".into();
    }
    let mut metadata = serde_json::Map::new();
    if let Some(seq) = value["seq"].as_u64() {
        metadata.insert("seq".into(), seq.into());
    }
    if let Some(ts) = value["ts"].as_str()
        && chrono::DateTime::parse_from_rfc3339(ts).is_ok()
    {
        metadata.insert("ts".into(), ts.into());
    }
    let frame = value.get("raw").unwrap_or(&value);
    let kind = value
        .get("hydra")
        .and_then(|v| v.get("type"))
        .or_else(|| frame.get("type"))
        .or_else(|| frame.get("event"));
    if let Some(kind) = kind.and_then(Value::as_str) {
        metadata.insert("type".into(), kind.into());
    }
    if let Some((name, arguments)) = frame_call(frame) {
        if !name.is_empty() {
            metadata.insert("tool".into(), name.into());
        }
        metadata.insert("head".into(), call_inputs(name, &arguments).0.into());
    }
    let shown = format!("{}, {} bytes", Value::Object(metadata), line.len());
    let mut cut: String = shown.chars().take(max_chars).collect();
    if cut.len() < shown.len() {
        cut.push_str(" [cut]");
    }
    cut
}

// Walk only the adapters' structural call positions, never tool result payloads.
fn frame_call(frame: &Value) -> Option<(&str, Value)> {
    if let Some(command) = COMMAND_KEYS.iter().find_map(|key| frame.get(key)) {
        return Some(("", serde_json::json!({"command": command})));
    }
    if let Some(blocks) = frame["message"]["content"].as_array() {
        for block in blocks {
            if block["type"] == "tool_use" {
                return Some((block["name"].as_str().unwrap_or(""), block["input"].clone()));
            }
        }
    }
    let step = frame.get("step_update").unwrap_or(frame);
    if step["step_type"] == "tool" || step.get("tool_info").is_some() {
        let info = &step["tool_info"];
        let name = step["tool_name"]
            .as_str()
            .or_else(|| info["name"].as_str())
            .unwrap_or("");
        return Some((name, info["parameters"].clone()));
    }
    let item = &frame["item"];
    match item["type"].as_str() {
        Some("command_execution") => {
            return Some(("Bash", serde_json::json!({"command": item["command"]})));
        }
        Some("mcp_tool_call") => {
            return Some((
                item["tool"].as_str().unwrap_or(""),
                item["arguments"].clone(),
            ));
        }
        _ => {}
    }
    if frame["type"] == "tool_call"
        && let Some(calls) = frame["tool_call"].as_object()
    {
        for (name, call) in calls {
            if call.get("args").is_some() {
                return Some((name.as_str(), call["args"].clone()));
            }
        }
    }
    None
}
