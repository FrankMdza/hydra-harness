//! `hydra models`: the models each harness offers, from the CLIs themselves.
//!
//! Build ticket 292. `refresh` runs `agy models`, `cursor-agent models` and
//! `codex debug models` under the launch environment's allowlist, with a timeout
//! each, and reads Claude's rows from `integrations/puppets/aliases.json`. The
//! table is `models.json` under the data root. `hydra models` prints that file
//! and launches nothing. A provider whose command fails keeps its previous rows
//! and `fetched_at`, so one broken CLI does not empty the table.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::env::ChildEnv;
use crate::paths::Layout;
use crate::provider::Provider;

/// How long `refresh` waits for one listing command.
pub const REFRESH_TIMEOUT: Duration = Duration::from_secs(60);

/// Where the table lives under a [`Layout`].
pub fn catalog_path(layout: &Layout) -> PathBuf {
    layout.root().join("models.json")
}

/// One model a harness offers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelRow {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_levels: Option<Vec<String>>,
}

/// What the last refresh recorded for one provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ProviderEntry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fetched_at: Option<String>,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default)]
    pub rows: Vec<ModelRow>,
}

/// The four providers, in the order the rest of the binary names them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Catalog {
    #[serde(default)]
    pub claude: ProviderEntry,
    #[serde(rename = "cursor-agent", default)]
    pub cursor_agent: ProviderEntry,
    #[serde(default)]
    pub agy: ProviderEntry,
    #[serde(default)]
    pub codex: ProviderEntry,
}

impl Catalog {
    fn entry(&self, provider: Provider) -> &ProviderEntry {
        match provider {
            Provider::Claude => &self.claude,
            Provider::CursorAgent => &self.cursor_agent,
            Provider::Agy => &self.agy,
            Provider::Codex => &self.codex,
        }
    }

    fn entry_mut(&mut self, provider: Provider) -> &mut ProviderEntry {
        match provider {
            Provider::Claude => &mut self.claude,
            Provider::CursorAgent => &mut self.cursor_agent,
            Provider::Agy => &mut self.agy,
            Provider::Codex => &mut self.codex,
        }
    }
}

/// A source of listing-command stdout. Production uses [`CliListing`]; tests
/// pass a closure over recorded samples.
pub trait Listing {
    fn fetch(&self, provider: Provider) -> Result<String, String>;
}

impl<F> Listing for F
where
    F: Fn(Provider) -> Result<String, String>,
{
    fn fetch(&self, provider: Provider) -> Result<String, String> {
        self(provider)
    }
}

/// Runs the three listing commands on PATH, under [`ChildEnv`].
pub struct CliListing {
    pub timeout: Duration,
    /// Directories prepended to PATH. Tests plant stub binaries here.
    pub path_prepend: Vec<PathBuf>,
}

impl Default for CliListing {
    fn default() -> Self {
        Self {
            timeout: REFRESH_TIMEOUT,
            path_prepend: Vec::new(),
        }
    }
}

impl Listing for CliListing {
    fn fetch(&self, provider: Provider) -> Result<String, String> {
        fetch_cli(provider, self.timeout, &self.path_prepend)
    }
}

fn source_of(provider: Provider) -> &'static str {
    match provider {
        Provider::Claude => "aliases",
        Provider::CursorAgent => "cursor-agent models",
        Provider::Agy => "agy models",
        Provider::Codex => "codex debug models",
    }
}

fn listing_args(provider: Provider) -> &'static [&'static str] {
    match provider {
        Provider::Agy | Provider::CursorAgent => &["models"],
        Provider::Codex => &["debug", "models"],
        Provider::Claude => &[],
    }
}

/// Strip the zero-width characters `cursor-agent` appends to some names.
fn strip_zw(text: &str) -> String {
    text.chars()
        .filter(|c| {
            !matches!(
                *c,
                '\u{200b}' | '\u{200c}' | '\u{200d}' | '\u{2060}' | '\u{feff}'
            )
        })
        .collect()
}

/// `agy models`: `id<TAB>display name` after an optional fetching line.
pub fn parse_agy(text: &str) -> Result<Vec<ModelRow>, String> {
    let mut rows = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let line = line.trim_end_matches('\r');
        if line.is_empty() || line.starts_with("Fetching available models") {
            continue;
        }
        match line.split_once('\t') {
            Some((id, name)) => {
                let id = id.trim();
                let name = name.trim();
                if id.is_empty() {
                    return Err(format!("agy models: line {} has an empty id", i + 1));
                }
                rows.push(ModelRow {
                    id: id.to_string(),
                    name: name.to_string(),
                    reasoning_levels: None,
                });
            }
            None => {
                return Err(format!("agy models: line {} is not id<TAB>name", i + 1));
            }
        }
    }
    Ok(rows)
}

/// `cursor-agent models`: `id - display name`, after `Available models`.
///
/// Lines that are not that shape are skipped: the listing on this machine
/// ends with a footer the shape in the ticket did not name. An output with
/// no rows at all and leftover lines is still an error, so a CLI failure
/// message does not become an empty table marked ok.
pub fn parse_cursor(text: &str) -> Result<Vec<ModelRow>, String> {
    let mut rows = Vec::new();
    let mut skipped = 0;
    for (i, line) in text.lines().enumerate() {
        let line = strip_zw(line);
        let line = line.trim_end_matches('\r').trim();
        if line.is_empty() || line.eq_ignore_ascii_case("available models") {
            continue;
        }
        match line.split_once(" - ") {
            Some((id, name)) => {
                let id = id.trim();
                let name = name.trim();
                if id.is_empty() {
                    return Err(format!(
                        "cursor-agent models: line {} has an empty id",
                        i + 1
                    ));
                }
                rows.push(ModelRow {
                    id: id.to_string(),
                    name: name.to_string(),
                    reasoning_levels: None,
                });
            }
            None => skipped += 1,
        }
    }
    if rows.is_empty() && skipped > 0 {
        return Err("cursor-agent models: no id - name lines".to_string());
    }
    Ok(rows)
}

#[derive(Deserialize)]
struct CodexListing {
    models: Vec<CodexModel>,
}

#[derive(Deserialize)]
struct CodexModel {
    slug: String,
    display_name: String,
    #[serde(default)]
    supported_reasoning_levels: Vec<CodexLevel>,
}

#[derive(Deserialize)]
struct CodexLevel {
    effort: String,
}

/// `codex debug models`: a JSON object with a `models` array.
pub fn parse_codex(text: &str) -> Result<Vec<ModelRow>, String> {
    let trimmed = text.trim();
    let json = trimmed.find('{').map(|i| &trimmed[i..]).unwrap_or(trimmed);
    let listing: CodexListing =
        serde_json::from_str(json).map_err(|err| format!("codex debug models: {err}"))?;
    let mut rows = Vec::with_capacity(listing.models.len());
    for model in listing.models {
        if model.slug.is_empty() {
            return Err("codex debug models: a model has an empty slug".to_string());
        }
        let levels: Vec<String> = model
            .supported_reasoning_levels
            .into_iter()
            .map(|level| level.effort)
            .filter(|effort| !effort.is_empty())
            .collect();
        rows.push(ModelRow {
            id: model.slug,
            name: model.display_name,
            reasoning_levels: if levels.is_empty() {
                None
            } else {
                Some(levels)
            },
        });
    }
    Ok(rows)
}

#[derive(Deserialize)]
struct AliasRow {
    canonical: AliasCanonical,
}

#[derive(Deserialize)]
struct AliasCanonical {
    provider: String,
    model: Option<String>,
}

/// Unique Claude canonical models from `aliases.json`. Names are the canonical
/// ids; nothing is invented.
pub fn parse_claude_aliases(text: &str) -> Result<Vec<ModelRow>, String> {
    let rows: Vec<AliasRow> =
        serde_json::from_str(text).map_err(|err| format!("aliases.json: {err}"))?;
    let mut seen = Vec::new();
    let mut out = Vec::new();
    for row in rows {
        if row.canonical.provider != "claude" {
            continue;
        }
        let Some(id) = row.canonical.model.filter(|model| !model.is_empty()) else {
            continue;
        };
        if seen.iter().any(|have: &String| have == &id) {
            continue;
        }
        seen.push(id.clone());
        out.push(ModelRow {
            id: id.clone(),
            name: id,
            reasoning_levels: None,
        });
    }
    Ok(out)
}

fn parse_listing(provider: Provider, text: &str) -> Result<Vec<ModelRow>, String> {
    match provider {
        Provider::Agy => parse_agy(text),
        Provider::CursorAgent => parse_cursor(text),
        Provider::Codex => parse_codex(text),
        Provider::Claude => Err("claude has no listing command".to_string()),
    }
}

fn load_catalog(path: &Path) -> Catalog {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Catalog::default();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

fn stamp(now: DateTime<Utc>) -> String {
    now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn succeed(source: &str, rows: Vec<ModelRow>, now: DateTime<Utc>) -> ProviderEntry {
    ProviderEntry {
        fetched_at: Some(stamp(now)),
        source: source.to_string(),
        ok: true,
        error: None,
        rows,
    }
}

fn fail(previous: &ProviderEntry, source: &str, error: String) -> ProviderEntry {
    ProviderEntry {
        fetched_at: previous.fetched_at.clone(),
        source: source.to_string(),
        ok: false,
        error: Some(error),
        rows: previous.rows.clone(),
    }
}

fn write_catalog(path: &Path, catalog: &Catalog) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let bytes = serde_json::to_vec_pretty(catalog)?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, bytes).with_context(|| format!("writing {}", tmp.display()))?;
    std::fs::rename(&tmp, path).with_context(|| format!("publishing {}", path.display()))?;
    Ok(())
}

/// Run the three listing commands (or the test stand-in), read Claude from
/// `aliases`, and write the table atomically.
pub fn refresh(
    layout: &Layout,
    aliases: &Path,
    listing: &impl Listing,
    now: DateTime<Utc>,
    out: &mut dyn Write,
) -> Result<Catalog> {
    let path = catalog_path(layout);
    let mut catalog = load_catalog(&path);

    match std::fs::read_to_string(aliases) {
        Ok(text) => match parse_claude_aliases(&text) {
            Ok(rows) => {
                let n = rows.len();
                *catalog.entry_mut(Provider::Claude) =
                    succeed(source_of(Provider::Claude), rows, now);
                writeln!(out, "claude  aliases  {n} models")?;
            }
            Err(err) => {
                let next = fail(
                    catalog.entry(Provider::Claude),
                    source_of(Provider::Claude),
                    err.clone(),
                );
                *catalog.entry_mut(Provider::Claude) = next;
                writeln!(out, "claude  aliases  error: {err}")?;
            }
        },
        Err(err) => {
            let err = format!("reading {}: {err}", aliases.display());
            let next = fail(
                catalog.entry(Provider::Claude),
                source_of(Provider::Claude),
                err.clone(),
            );
            *catalog.entry_mut(Provider::Claude) = next;
            writeln!(out, "claude  aliases  error: {err}")?;
        }
    }

    for provider in [Provider::CursorAgent, Provider::Agy, Provider::Codex] {
        let source = source_of(provider);
        let next = match listing.fetch(provider) {
            Ok(text) => match parse_listing(provider, &text) {
                Ok(rows) => {
                    let n = rows.len();
                    writeln!(out, "{}  {source}  {n} models", provider.binary())?;
                    succeed(source, rows, now)
                }
                Err(err) => {
                    let kept = catalog.entry(provider).rows.len();
                    writeln!(
                        out,
                        "{}  {source}  error: {err} (kept {kept})",
                        provider.binary()
                    )?;
                    fail(catalog.entry(provider), source, err)
                }
            },
            Err(err) => {
                let kept = catalog.entry(provider).rows.len();
                writeln!(
                    out,
                    "{}  {source}  error: {err} (kept {kept})",
                    provider.binary()
                )?;
                fail(catalog.entry(provider), source, err)
            }
        };
        *catalog.entry_mut(provider) = next;
    }

    write_catalog(&path, &catalog)?;
    writeln!(out, "wrote {}", path.display())?;
    Ok(catalog)
}

/// Print the table from the file. Launches nothing.
pub fn show(
    layout: &Layout,
    provider: Option<Provider>,
    json: bool,
    now: DateTime<Utc>,
    out: &mut dyn Write,
) -> Result<()> {
    let path = catalog_path(layout);
    if !path.is_file() {
        bail!("no models table yet; run hydra models refresh");
    }
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let catalog: Catalog =
        serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;

    if json {
        match provider {
            Some(provider) => {
                let value = serde_json::json!({
                    provider.binary(): catalog.entry(provider)
                });
                writeln!(out, "{}", serde_json::to_string_pretty(&value)?)?;
            }
            None => writeln!(out, "{}", serde_json::to_string_pretty(&catalog)?)?,
        }
        return Ok(());
    }

    let providers = match provider {
        Some(provider) => vec![provider],
        None => vec![
            Provider::Claude,
            Provider::CursorAgent,
            Provider::Agy,
            Provider::Codex,
        ],
    };
    for (i, provider) in providers.iter().enumerate() {
        if i > 0 {
            writeln!(out)?;
        }
        write_provider(out, provider.binary(), catalog.entry(*provider), now)?;
    }
    Ok(())
}

fn write_provider(
    out: &mut dyn Write,
    name: &str,
    entry: &ProviderEntry,
    now: DateTime<Utc>,
) -> Result<()> {
    let status = if entry.ok {
        format!("{} models", entry.rows.len())
    } else {
        match &entry.error {
            Some(err) => format!("FAILED {err}"),
            None => "FAILED".to_string(),
        }
    };
    let age = match &entry.fetched_at {
        Some(fetched) => format!("  fetched {fetched} ({})", age_label(fetched, now)),
        None => String::new(),
    };
    writeln!(out, "{name}  {}  {status}{age}", entry.source)?;
    let width = entry.rows.iter().map(|row| row.id.len()).max().unwrap_or(0);
    for row in &entry.rows {
        write!(out, "  {:width$}  {}", row.id, row.name, width = width)?;
        if let Some(levels) = &row.reasoning_levels {
            write!(out, "  [{}]", levels.join(", "))?;
        }
        writeln!(out)?;
    }
    Ok(())
}

fn age_label(fetched: &str, now: DateTime<Utc>) -> String {
    let Ok(then) = DateTime::parse_from_rfc3339(fetched) else {
        return "unknown age".to_string();
    };
    let then = then.with_timezone(&Utc);
    let secs = now.signed_duration_since(then).num_seconds();
    if secs < 0 {
        return "in the future".to_string();
    }
    if secs < 60 {
        return "just now".to_string();
    }
    if secs < 3600 {
        return format!("{}m ago", secs / 60);
    }
    if secs < 86400 {
        return format!("{}h ago", secs / 3600);
    }
    format!("{}d ago", secs / 86400)
}

fn fetch_cli(
    provider: Provider,
    timeout: Duration,
    path_prepend: &[PathBuf],
) -> Result<String, String> {
    let mut env = ChildEnv::build(provider).map_err(|err| err.to_string())?;
    if !path_prepend.is_empty() {
        let rest = env
            .vars()
            .find(|(name, _)| *name == "PATH")
            .map(|(_, value)| value.to_string())
            .unwrap_or_default();
        let prefix = path_prepend
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(":");
        env.set("PATH", format!("{prefix}:{rest}"));
    }
    let path_value = env
        .vars()
        .find(|(name, _)| *name == "PATH")
        .map(|(_, value)| value.to_string())
        .unwrap_or_default();
    let binary = which(provider.binary(), &path_value);

    let mut child = Command::new(&binary)
        .args(listing_args(provider))
        .env_clear()
        .envs(env.vars().map(|(k, v)| (k.to_string(), v.to_string())))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| format!("{} is not on PATH: {err}", provider.binary()))?;

    let mut stdout = child.stdout.take().ok_or_else(|| "no stdout".to_string())?;
    let mut stderr = child.stderr.take().ok_or_else(|| "no stderr".to_string())?;
    let out_h = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = stdout.read_to_end(&mut bytes);
        bytes
    });
    let err_h = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = stderr.read_to_end(&mut bytes);
        bytes
    });

    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                break Err(format!("timed out after {}s", timeout.as_secs()));
            }
            Err(err) => {
                let _ = child.kill();
                break Err(err.to_string());
            }
        }
    };

    let stdout = out_h.join().unwrap_or_default();
    let stderr = err_h.join().unwrap_or_default();
    let source = source_of(provider);
    let status = status?;
    if !status.success() {
        let err = String::from_utf8_lossy(&stderr);
        let err = err.trim();
        let code = status
            .code()
            .map(|code| format!("exited {code}"))
            .unwrap_or_else(|| "died on a signal".to_string());
        if err.is_empty() {
            return Err(format!("{source} {code}"));
        }
        return Err(format!("{source} {code}: {err}"));
    }
    Ok(String::from_utf8_lossy(&stdout).into_owned())
}

/// The first executable named `name` on `path`. `Command::new` looks up the
/// parent process PATH, not the child's, so a test that prepends stubs has to
/// resolve the binary itself.
fn which(name: &str, path: &str) -> PathBuf {
    for dir in path.split(':') {
        if dir.is_empty() {
            continue;
        }
        let candidate = Path::new(dir).join(name);
        if candidate.is_file() {
            return candidate;
        }
    }
    PathBuf::from(name)
}
