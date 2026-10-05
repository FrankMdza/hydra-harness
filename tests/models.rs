//! `hydra models` and `hydra models refresh`, build ticket 292.
//!
//! Recorded samples of the three listing outputs, a stub command that fails, and
//! the launchd template's two times. Nothing here launches a real provider CLI.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use chrono::{TimeZone, Utc};
use clap::Parser;
use hydra::cli::{Cli, Command, ModelsCommand};
use hydra::models::{
    self, CliListing, Listing, ModelRow, catalog_path, parse_agy, parse_claude_aliases,
    parse_codex, parse_cursor,
};
use hydra::paths::Layout;
use hydra::provider::Provider;

struct Scratch {
    root: PathBuf,
}

impl Scratch {
    fn new(name: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "hydra-models-{}-{}-{}",
            std::process::id(),
            name,
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    fn layout(&self) -> Layout {
        Layout::at(&self.root)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

const NOW: &str = "2026-10-04T21:00:00Z";

fn now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 4, 21, 0, 0).unwrap()
}

const AGY_SAMPLE: &str = "\
Fetching available models...
gemini-3.1-pro\tGemini 3.1 Pro
gemini-3.8-flash\tGemini 3.8 Flash
";

const CURSOR_SAMPLE: &str = "\
Available models

composer-2.5 - Composer 2.5
claude-4-sonnet - Claude 4 Sonnet\u{200b}
";

const CODEX_SAMPLE: &str = r#"{
  "models": [
    {
      "slug": "gpt-5",
      "display_name": "GPT-5",
      "description": "flagship",
      "default_reasoning_level": "medium",
      "supported_reasoning_levels": [
        {"effort": "low"},
        {"effort": "medium", "note": "default"},
        {"effort": "high"}
      ]
    }
  ]
}"#;

const ALIASES: &str = r#"[
  {"provider": "claude", "recorded_model": null, "canonical": {"provider": "claude", "model": null}},
  {"provider": "claude", "recorded_model": "opus", "canonical": {"provider": "claude", "model": "opus"}},
  {"provider": "claude", "recorded_model": "Opus 5", "canonical": {"provider": "claude", "model": "opus"}},
  {"provider": "claude", "recorded_model": "sonnet", "canonical": {"provider": "claude", "model": "sonnet"}},
  {"provider": "agy", "recorded_model": "gemini-x", "canonical": {"provider": "agy", "model": "gemini-x"}}
]"#;

fn aliases_file(scratch: &Scratch) -> PathBuf {
    let path = scratch.root.join("aliases.json");
    std::fs::write(&path, ALIASES).unwrap();
    path
}

fn write_stub(dir: &Path, name: &str, body: &str) {
    let path = dir.join(name);
    std::fs::write(&path, body).unwrap();
    let mut perms = std::fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&path, perms).unwrap();
}

fn samples() -> impl Listing {
    |provider: Provider| match provider {
        Provider::Agy => Ok(AGY_SAMPLE.to_string()),
        Provider::CursorAgent => Ok(CURSOR_SAMPLE.to_string()),
        Provider::Codex => Ok(CODEX_SAMPLE.to_string()),
        Provider::Claude => Err("claude has no listing command".to_string()),
    }
}

#[test]
fn parse_agy_skips_the_fetching_line_and_splits_on_tab() {
    let rows = parse_agy(AGY_SAMPLE).unwrap();
    assert_eq!(
        rows,
        [
            ModelRow {
                id: "gemini-3.1-pro".into(),
                name: "Gemini 3.1 Pro".into(),
                reasoning_levels: None,
            },
            ModelRow {
                id: "gemini-3.8-flash".into(),
                name: "Gemini 3.8 Flash".into(),
                reasoning_levels: None,
            },
        ]
    );
}

#[test]
fn parse_cursor_splits_on_dash_and_strips_zero_width() {
    let rows = parse_cursor(CURSOR_SAMPLE).unwrap();
    assert_eq!(rows[0].id, "composer-2.5");
    assert_eq!(rows[0].name, "Composer 2.5");
    assert_eq!(rows[1].id, "claude-4-sonnet");
    assert_eq!(rows[1].name, "Claude 4 Sonnet");
    assert!(!rows[1].name.chars().any(|c| c == '\u{200b}'));
}

#[test]
fn parse_cursor_skips_a_footer_line_that_is_not_id_dash_name() {
    let text = "\
Available models

composer-2.5 - Composer 2.5
Type cursor-agent models --help for more
";
    let rows = parse_cursor(text).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, "composer-2.5");
}

#[test]
fn parse_cursor_refuses_output_with_no_rows() {
    let err = parse_cursor("Available models\n\nnot a listing\n").unwrap_err();
    assert!(err.contains("no id - name"), "{err}");
}

#[test]
fn parse_codex_reads_slug_name_and_reasoning_levels() {
    let rows = parse_codex(CODEX_SAMPLE).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, "gpt-5");
    assert_eq!(rows[0].name, "GPT-5");
    assert_eq!(
        rows[0].reasoning_levels,
        Some(vec!["low".into(), "medium".into(), "high".into()])
    );
}

#[test]
fn parse_aliases_keeps_unique_claude_canonicals_and_invents_none() {
    let rows = parse_claude_aliases(ALIASES).unwrap();
    assert_eq!(
        rows,
        [
            ModelRow {
                id: "opus".into(),
                name: "opus".into(),
                reasoning_levels: None,
            },
            ModelRow {
                id: "sonnet".into(),
                name: "sonnet".into(),
                reasoning_levels: None,
            },
        ]
    );
}

#[test]
fn refresh_writes_the_four_providers_from_samples_and_aliases() {
    let scratch = Scratch::new("refresh-ok");
    let layout = scratch.layout();
    let aliases = aliases_file(&scratch);
    let mut out = Vec::new();
    let catalog = models::refresh(&layout, &aliases, &samples(), now(), &mut out).unwrap();
    assert!(catalog.agy.ok);
    assert_eq!(catalog.agy.source, "agy models");
    assert_eq!(catalog.agy.fetched_at.as_deref(), Some(NOW));
    assert_eq!(catalog.agy.rows.len(), 2);
    assert!(catalog.cursor_agent.ok);
    assert_eq!(catalog.cursor_agent.rows[1].name, "Claude 4 Sonnet");
    assert!(catalog.codex.ok);
    assert_eq!(
        catalog.codex.rows[0].reasoning_levels,
        Some(vec!["low".into(), "medium".into(), "high".into()])
    );
    assert!(catalog.claude.ok);
    assert_eq!(catalog.claude.source, "aliases");
    assert_eq!(
        catalog
            .claude
            .rows
            .iter()
            .map(|r| r.id.as_str())
            .collect::<Vec<_>>(),
        ["opus", "sonnet"]
    );
    let path = catalog_path(&layout);
    assert!(path.is_file(), "{}", path.display());
    let on_disk: models::Catalog =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(on_disk.agy.rows, catalog.agy.rows);
}

#[test]
fn a_stub_command_that_fails_keeps_the_old_rows_and_fetched_at() {
    let scratch = Scratch::new("keep-old");
    let layout = scratch.layout();
    let aliases = aliases_file(&scratch);
    let previous = models::Catalog {
        claude: models::ProviderEntry {
            fetched_at: Some("2026-01-01T00:00:00Z".into()),
            source: "aliases".into(),
            ok: true,
            error: None,
            rows: vec![ModelRow {
                id: "old-claude".into(),
                name: "old".into(),
                reasoning_levels: None,
            }],
        },
        cursor_agent: models::ProviderEntry {
            fetched_at: Some("2026-01-01T00:00:00Z".into()),
            source: "cursor-agent models".into(),
            ok: true,
            error: None,
            rows: vec![ModelRow {
                id: "old-cursor".into(),
                name: "Old Cursor".into(),
                reasoning_levels: None,
            }],
        },
        agy: models::ProviderEntry {
            fetched_at: Some("2026-01-01T00:00:00Z".into()),
            source: "agy models".into(),
            ok: true,
            error: None,
            rows: vec![ModelRow {
                id: "old-agy".into(),
                name: "Old Agy".into(),
                reasoning_levels: None,
            }],
        },
        codex: models::ProviderEntry {
            fetched_at: Some("2026-01-01T00:00:00Z".into()),
            source: "codex debug models".into(),
            ok: true,
            error: None,
            rows: vec![ModelRow {
                id: "old-codex".into(),
                name: "Old Codex".into(),
                reasoning_levels: Some(vec!["low".into()]),
            }],
        },
    };
    std::fs::write(
        catalog_path(&layout),
        serde_json::to_vec_pretty(&previous).unwrap(),
    )
    .unwrap();

    let bin = scratch.root.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    let fail = "#!/bin/sh\necho stub-failed >&2\nexit 1\n";
    write_stub(&bin, "agy", fail);
    write_stub(&bin, "cursor-agent", fail);
    write_stub(&bin, "codex", fail);

    let listing = CliListing {
        timeout: Duration::from_secs(5),
        path_prepend: vec![bin],
    };
    let mut out = Vec::new();
    let catalog = models::refresh(&layout, &aliases, &listing, now(), &mut out).unwrap();

    assert!(!catalog.agy.ok);
    assert_eq!(
        catalog.agy.fetched_at.as_deref(),
        Some("2026-01-01T00:00:00Z")
    );
    assert_eq!(catalog.agy.rows[0].id, "old-agy");
    assert!(
        catalog
            .agy
            .error
            .as_deref()
            .is_some_and(|e| e.contains("exited 1") && e.contains("stub-failed")),
        "{:?}",
        catalog.agy.error
    );
    assert!(!catalog.cursor_agent.ok);
    assert_eq!(catalog.cursor_agent.rows[0].id, "old-cursor");
    assert_eq!(
        catalog.cursor_agent.fetched_at.as_deref(),
        Some("2026-01-01T00:00:00Z")
    );
    assert!(!catalog.codex.ok);
    assert_eq!(catalog.codex.rows[0].id, "old-codex");
    assert_eq!(
        catalog.codex.rows[0].reasoning_levels,
        Some(vec!["low".into()])
    );
    assert!(catalog.claude.ok);
    assert_eq!(catalog.claude.fetched_at.as_deref(), Some(NOW));
    assert_eq!(catalog.claude.rows[0].id, "opus");
}

#[test]
fn show_prints_the_table_and_its_age_and_launches_nothing() {
    let scratch = Scratch::new("show");
    let layout = scratch.layout();
    let aliases = aliases_file(&scratch);
    let mut sink = Vec::new();
    models::refresh(&layout, &aliases, &samples(), now(), &mut sink).unwrap();

    let mut out = Vec::new();
    models::show(&layout, None, false, now(), &mut out).unwrap();
    let text = String::from_utf8(out).unwrap();
    assert!(
        text.contains("just now") || text.contains("fetched"),
        "{text}"
    );
    assert!(text.contains("gemini-3.1-pro"), "{text}");
    assert!(text.contains("composer-2.5"), "{text}");
    assert!(text.contains("gpt-5"), "{text}");
    assert!(text.contains("opus"), "{text}");
    assert!(
        text.contains("low, medium, high") || text.contains("reasoning"),
        "{text}"
    );
}

#[test]
fn show_without_a_file_says_to_run_refresh() {
    let scratch = Scratch::new("missing");
    let layout = scratch.layout();
    let err = models::show(&layout, None, false, now(), &mut Vec::new()).unwrap_err();
    let text = err.to_string();
    assert!(text.contains("refresh"), "{text}");
}

#[test]
fn show_json_and_provider_filter_read_the_file() {
    let scratch = Scratch::new("json");
    let layout = scratch.layout();
    let aliases = aliases_file(&scratch);
    models::refresh(&layout, &aliases, &samples(), now(), &mut Vec::new()).unwrap();

    let mut out = Vec::new();
    models::show(&layout, Some(Provider::Agy), true, now(), &mut out).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert!(value.get("agy").is_some(), "{value}");
    assert!(value.get("codex").is_none(), "{value}");
}

#[test]
fn clap_parses_models_print_and_refresh() {
    let cli = Cli::try_parse_from(["hydra", "models"]).unwrap();
    assert!(matches!(
        cli.command,
        Command::Models {
            command: None,
            json: false,
            provider: None
        }
    ));
    let cli = Cli::try_parse_from(["hydra", "models", "refresh"]).unwrap();
    assert!(matches!(
        cli.command,
        Command::Models {
            command: Some(ModelsCommand::Refresh),
            ..
        }
    ));
    let cli = Cli::try_parse_from(["hydra", "models", "--json", "--provider", "agy"]).unwrap();
    match cli.command {
        Command::Models {
            command: None,
            json: true,
            provider: Some(provider),
        } => assert_eq!(provider.binary(), "agy"),
        _ => panic!("expected hydra models --json --provider agy"),
    }
}

fn models_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("integrations/models")
}

fn template() -> PathBuf {
    models_dir().join("launchd/co.hephaestuslab.hydra-models.plist")
}

fn calendar_hours(source: &str) -> Vec<i64> {
    let mut hours = Vec::new();
    let mut rest = source;
    while let Some(at) = rest.find("<key>Hour</key>") {
        rest = &rest[at + "<key>Hour</key>".len()..];
        let Some(open) = rest.find("<integer>") else {
            break;
        };
        rest = &rest[open + "<integer>".len()..];
        let Some(close) = rest.find("</integer>") else {
            break;
        };
        hours.push(rest[..close].trim().parse().unwrap());
        rest = &rest[close..];
    }
    hours
}

fn calendar_minutes(source: &str) -> Vec<i64> {
    let mut minutes = Vec::new();
    let mut rest = source;
    while let Some(at) = rest.find("<key>Minute</key>") {
        rest = &rest[at + "<key>Minute</key>".len()..];
        let Some(open) = rest.find("<integer>") else {
            break;
        };
        rest = &rest[open + "<integer>".len()..];
        let Some(close) = rest.find("</integer>") else {
            break;
        };
        minutes.push(rest[..close].trim().parse().unwrap());
        rest = &rest[close..];
    }
    minutes
}

#[test]
fn the_plist_runs_refresh_at_four_and_sixteen() {
    let source = std::fs::read_to_string(template()).unwrap();
    assert!(source.contains("StartCalendarInterval"), "{source}");
    assert_eq!(calendar_hours(&source), [4, 16]);
    assert_eq!(calendar_minutes(&source), [0, 0]);
    assert!(source.contains("/Users/USER/hydra/target/release/hydra"));
    assert!(source.contains("<string>models</string>"));
    assert!(source.contains("<string>refresh</string>"));
    assert_eq!(
        source.matches("<string>models</string>").count(),
        1,
        "{source}"
    );
}

#[test]
fn the_plist_is_a_property_list_to_the_system_too() {
    let plutil = Path::new("/usr/bin/plutil");
    if !plutil.exists() {
        return;
    }
    let ran = std::process::Command::new(plutil)
        .arg("-lint")
        .arg(template())
        .output()
        .unwrap();
    assert!(
        ran.status.success(),
        "{}",
        String::from_utf8_lossy(&ran.stdout)
    );
}

#[test]
fn the_readme_says_how_to_install_and_remove_it() {
    let readme = std::fs::read_to_string(models_dir().join("README.md")).unwrap();
    for said in [
        "launchctl bootstrap",
        "launchctl bootout",
        "cargo build --release",
        "s|/Users/USER|$HOME|g",
        "hydra models refresh",
    ] {
        assert!(readme.contains(said), "the README does not say `{said}`");
    }
}

#[test]
fn every_path_of_the_template_is_under_the_placeholder_home() {
    let source = std::fs::read_to_string(template()).unwrap();
    let installed = source.replace("/Users/USER", "/Users/someone");
    assert!(
        !installed.contains("USER"),
        "a placeholder the command would miss"
    );
}
