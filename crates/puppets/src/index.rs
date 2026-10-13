//! Regenerate the build tracker's Markdown index from its ticket files.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

/// Print one entry per numbered Markdown ticket, in numeric order.
pub fn build(dir: Option<&Path>, out: &mut dyn Write) -> anyhow::Result<i32> {
    let dir = crate::frontier::issues_dir(Some("build"), dir)?;
    let mut tickets = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        let Some((number, slug)) = name.strip_suffix(".md").and_then(|s| s.split_once('-')) else {
            continue;
        };
        if number.is_empty() || !number.bytes().all(|b| b.is_ascii_digit()) || slug.is_empty() {
            continue;
        }
        let order = number.parse::<u64>()?;
        let mut first = String::new();
        BufReader::new(fs::File::open(entry.path())?).read_line(&mut first)?;
        let heading = first.trim_start_matches(['#', ' ']).trim();
        let prefix = format!("{number} - ");
        let title = heading.strip_prefix(&prefix).unwrap_or(heading);
        tickets.push((
            order,
            name.to_string(),
            format!("- [{number} - {title}]({name})"),
        ));
    }
    tickets.sort_by(|a, b| (a.0, &a.1).cmp(&(b.0, &b.1)));
    for (_, _, line) in tickets {
        writeln!(out, "{line}")?;
    }
    Ok(0)
}
