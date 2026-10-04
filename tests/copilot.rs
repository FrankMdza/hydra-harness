//! The ungated copilot's short document and launcher, build ticket 249.

use std::path::Path;

use clap::Parser;
use hydra::cli::{Cli, Command as HydraCommand};

fn manifest() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn copilot_document() -> std::path::PathBuf {
    manifest().join("integrations/copilot/COPILOT.md")
}

fn copilot_bin() -> std::path::PathBuf {
    manifest().join("integrations/copilot/bin/hydra-copilot")
}

#[test]
fn the_copilot_document_is_short_and_names_wait_and_cockpit_help() {
    let text = std::fs::read_to_string(copilot_document())
        .unwrap_or_else(|err| panic!("reading {}: {err}", copilot_document().display()));
    let lines = text.lines().count();
    assert!(
        lines < 150,
        "integrations/copilot/COPILOT.md is {lines} lines; ticket 249 caps it at 150"
    );
    assert!(
        text.contains("hydra wait"),
        "the copilot document must tell the session to wait with hydra wait"
    );
    assert!(
        !text.contains("../.."),
        "the copilot document must not reach the repository by a relative path"
    );
    assert!(
        text.contains("{{REPO}}"),
        "the launcher renders the repository's path into {{REPO}}"
    );
    assert!(
        text.contains("COCKPIT.md"),
        "the copilot document must point at COCKPIT.md for cockpit help"
    );
}

#[test]
fn hydra_copilot_argv_matches_cli_without_launching() {
    let script = std::fs::read_to_string(copilot_bin())
        .unwrap_or_else(|err| panic!("reading {}: {err}", copilot_bin().display()));
    // Claude Code reads every CLAUDE.md from its directory up to the root, so
    // the chat must start outside the repository to skip the build
    // instructions.
    assert!(
        script.contains("cd \"$HOME_DIR\"") && script.contains("is inside $REPO"),
        "hydra-copilot must start the chat in a directory outside the repository"
    );
    assert!(
        script.contains("cockpit --session") && script.contains("--model"),
        "hydra-copilot must exec ungated cockpit with session and model flags"
    );
    assert!(
        script.contains("MODEL=claude-opus-5-5"),
        "hydra-copilot must default a claude copilot's model to opus"
    );

    for want in ["sonnet", "opus"] {
        let argv = vec!["hydra", "cockpit", "--session", "claude", "--model", want];
        let cli = Cli::try_parse_from(argv).expect("cockpit argv must parse");
        match cli.command {
            HydraCommand::Cockpit {
                session,
                model,
                policy,
                ..
            } => {
                assert_eq!(session.as_deref(), Some("claude"));
                assert_eq!(model.as_deref(), Some(want));
                assert!(policy.is_none(), "ungated copilot must not pass --policy");
            }
            _ => panic!("expected cockpit"),
        }
    }
}
