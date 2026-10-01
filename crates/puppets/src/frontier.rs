//! `.scratch/hydra/frontier.py` as a subcommand: which tickets are takeable and what blocks
//! the rest.

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

struct Ticket {
    title: String,
    status: String,
    kind: String,
    blocked_by: Vec<String>,
}

/// Print the frontier for the map (`issues/`) or `build/` under `.scratch/hydra`, or for
/// `--dir`.
pub fn run(tracker: Option<&str>, dir: Option<&Path>, out: &mut dyn Write) -> anyhow::Result<i32> {
    let issues = issues_dir(tracker, dir)?;
    let tickets = load(&issues)?;
    if tickets.is_empty() {
        anyhow::bail!("no tickets found in {}", issues.display());
    }
    print_frontier(&tickets, out)?;
    Ok(0)
}

fn issues_dir(tracker: Option<&str>, dir: Option<&Path>) -> anyhow::Result<PathBuf> {
    if let Some(dir) = dir {
        return Ok(dir.to_path_buf());
    }
    let here = find_scratch_hydra()?;
    let name = tracker.unwrap_or("issues");
    Ok(here.join(name))
}

fn find_scratch_hydra() -> anyhow::Result<PathBuf> {
    let mut dir = std::env::current_dir()?;
    loop {
        let candidate = dir.join(".scratch/hydra");
        if candidate.is_dir() {
            return Ok(candidate);
        }
        if !dir.pop() {
            break;
        }
    }
    anyhow::bail!("could not find .scratch/hydra from the working directory")
}

fn load(issues: &Path) -> anyhow::Result<BTreeMap<String, Ticket>> {
    let mut tickets = BTreeMap::new();
    let mut paths: Vec<PathBuf> = fs::read_dir(issues)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| ticket_name(path).is_some())
        .collect();
    paths.sort_by_key(|path| {
        let num = ticket_name(path).unwrap();
        num.parse::<u64>().unwrap_or(0)
    });

    for path in paths {
        let num = ticket_name(&path).unwrap();
        let text = fs::read_to_string(&path)?;
        let first = text.lines().next().unwrap_or("");
        let title = title_line(first);
        tickets.insert(
            num,
            Ticket {
                title,
                status: field_word(&text, "Status: ").unwrap_or_else(|| "open".into()),
                kind: field_word(&text, "Type: ").unwrap_or_else(|| "?".into()),
                blocked_by: blocked_by(&text),
            },
        );
    }
    Ok(tickets)
}

fn ticket_name(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_str()?;
    if !name.ends_with(".md") {
        return None;
    }
    let stem = name.strip_suffix(".md")?;
    let (num, rest) = stem.split_once('-')?;
    if num.is_empty() || !num.chars().all(|c| c.is_ascii_digit()) || rest.is_empty() {
        return None;
    }
    Some(num.to_string())
}

fn title_line(line: &str) -> String {
    let trimmed = line.trim_start_matches(['#', ' ']);
    trimmed.trim().to_string()
}

fn field_word(text: &str, prefix: &str) -> Option<String> {
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix(prefix) {
            return take_word(rest);
        }
    }
    None
}

fn blocked_by(text: &str) -> Vec<String> {
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("Blocked by: ") {
            if rest.is_empty() {
                continue;
            }
            return rest
                .split(',')
                .map(|part| part.trim().to_string())
                .collect();
        }
    }
    Vec::new()
}

/// One run of Python 3 `\w+` at the start of `text`.
fn take_word(text: &str) -> Option<String> {
    let mut end = 0;
    for (i, ch) in text.char_indices() {
        if ch.is_alphanumeric() || ch == '_' {
            end = i + ch.len_utf8();
        } else {
            break;
        }
    }
    if end == 0 {
        None
    } else {
        Some(text[..end].to_string())
    }
}

fn ticket_order(tickets: &BTreeMap<String, Ticket>) -> Vec<String> {
    let mut nums: Vec<String> = tickets.keys().cloned().collect();
    nums.sort_by_key(|num| num.parse::<u64>().unwrap_or(0));
    nums
}

fn print_frontier(tickets: &BTreeMap<String, Ticket>, out: &mut dyn Write) -> anyhow::Result<()> {
    let order = ticket_order(tickets);
    let frontier: Vec<&str> = order
        .iter()
        .filter(|num| {
            let t = &tickets[*num];
            t.status == "open" && unblocked(num, tickets)
        })
        .map(String::as_str)
        .collect();
    let blocked: Vec<&str> = order
        .iter()
        .filter(|num| {
            let t = &tickets[*num];
            t.status == "open" && !unblocked(num, tickets)
        })
        .map(String::as_str)
        .collect();
    let claimed: Vec<&str> = order
        .iter()
        .filter(|num| tickets[*num].status == "claimed")
        .map(String::as_str)
        .collect();
    let done: Vec<&str> = order
        .iter()
        .filter(|num| tickets[*num].status == "resolved")
        .map(String::as_str)
        .collect();

    writeln!(out, "FRONTIER ({} takeable now)", frontier.len())?;
    for num in frontier {
        let t = &tickets[num];
        writeln!(out, "  {num}  {:<9} {}", t.kind, t.title)?;
    }

    if !claimed.is_empty() {
        writeln!(out)?;
        writeln!(out, "CLAIMED ({} in flight, do not take)", claimed.len())?;
        for num in claimed {
            let t = &tickets[num];
            writeln!(out, "  {num}  {:<9} {}", t.kind, t.title)?;
        }
    }

    writeln!(out)?;
    writeln!(out, "BLOCKED ({})", blocked.len())?;
    for num in blocked {
        let t = &tickets[num];
        let waiting: Vec<&str> = t
            .blocked_by
            .iter()
            .filter(|b| tickets.get(*b).map(|t| t.status.as_str()) != Some("resolved"))
            .map(String::as_str)
            .collect();
        writeln!(out, "  {num}  {}", t.title)?;
        writeln!(out, "      waiting on {}", waiting.join(", "))?;
    }

    writeln!(out)?;
    writeln!(out, "{} of {} resolved", done.len(), tickets.len())?;
    Ok(())
}

fn unblocked(num: &str, tickets: &BTreeMap<String, Ticket>) -> bool {
    tickets[num]
        .blocked_by
        .iter()
        .all(|b| tickets.get(b).map(|t| t.status.as_str()) == Some("resolved"))
}
