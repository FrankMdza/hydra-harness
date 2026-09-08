//! `hydra evolve`: Ouroboros rung two, one reviewed pass over Hydra's own source.
//!
//! A proposer under `standard`, protected-path refusal, the admission gate, a panel
//! of three `judge` runs, and a branch that carries the verdict. `docs/ouroboros-spec.md`
//! section 4 is the design; contract 5.5 is the harness run the pass writes.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use uuid::Uuid;

use crate::ouroboros::corpus;
use crate::ouroboros::review::{self, ApprovedPatch, JudgeOutcome, PanelVerdict};
use crate::outcome::RunResult;
use crate::paths::Layout;
use crate::policy::PolicySet;
use crate::provider::Provider;
use crate::record::{RunEnd, RunStart, SCHEMA_VERSION, Writer};
use crate::run::{Call, Request, run};
use crate::worktree::{Stat, diff_stat, git, seed_tree};

/// The maximum number of lines a proposer's patch may change before review.
/// Map ticket 52: a patch over this is rejected for size in one place.
pub const PATCH_LINE_CAP: usize = 400;

/// What the human asked for.
pub struct Pass {
    pub model: Option<String>,
    pub timeout: Duration,
    pub dry_run: bool,
}

/// What one pass did.
pub enum Done {
    /// The brief was composed and nothing was launched.
    Considered { brief: String },
    /// The pass finished inside the call's budget.
    Finished(Finished),
    /// The proposer outlived the call. Nothing is committed.
    Handed { pass_run_id: String },
}

pub struct Finished {
    pub pass_run_id: String,
    pub proposer_run_id: String,
    pub judge_run_ids: Vec<String>,
    pub branch: String,
    pub verdict: PanelVerdict,
    pub approved: Option<ApprovedPatch>,
    pub pass_record: PathBuf,
}

/// Reads the corpus, runs the proposer, reviews the patch, lands on a branch.
pub fn evolve(pass: Pass) -> Result<Done> {
    let layout = Layout::discover()?;
    let policies = PolicySet::discover()?;
    let pass_run_id = Uuid::now_v7().to_string();
    let repo = crate::worktree::repo_root(&std::env::current_dir()?)
        .context("hydra evolve needs a git repository to branch a worktree from")?;
    let selection = corpus::select(&layout, &policies)?;
    let brief = brief(&pass_run_id, &layout, &selection, &repo);

    if pass.dry_run {
        return Ok(Done::Considered { brief });
    }

    let pass_record = layout.run_file(&pass_run_id);
    write_harness_start(&layout, &pass_run_id, &pass_record)?;

    let proposer_run_id = Uuid::now_v7().to_string();
    let proposer_model = pass.model.clone();
    let call = run(Request {
        provider: Provider::Claude,
        task: brief,
        model: pass.model,
        policy: "standard".to_string(),
        floor_lifted: false,
        timeout: pass.timeout,
        parent: Some(pass_run_id.clone()),
        run_id: Some(proposer_run_id.clone()),
        command: "run".to_string(),
        resumed_from: None,
        resume_session_id: None,
        working_dir: None,
    });

    let proposer = match call {
        Ok(Call::Ended(result)) => *result,
        Ok(Call::Handle(_)) => {
            write_harness_end(&pass_record, None, Some("the proposer outlived the call"))?;
            return Ok(Done::Handed {
                pass_run_id: pass_run_id.clone(),
            });
        }
        Err(err) => {
            write_harness_end(&pass_record, None, Some(&err.to_string()))?;
            return Err(err);
        }
    };

    if let Some(abort) = rate_limit_abort(&layout)? {
        write_harness_end(&pass_record, None, Some(&abort))?;
        bail!(abort);
    }

    let patch = read_proposer_patch(&layout, &policies, &proposer)?;
    let blocked_branch = format!("ouroboros/blocked/{pass_run_id}");
    let offerable_branch = format!("ouroboros/{pass_run_id}");

    if let Some(count) = patch_over_cap(&patch.stat) {
        let message = commit_message(
            &pass_run_id,
            &proposer_run_id,
            &[],
            &format!("rejected for size: {count} lines changed exceeds cap of {PATCH_LINE_CAP}"),
        );
        commit_pass_branch(&repo, &proposer, &blocked_branch, &message)?;
        write_harness_end(&pass_record, Some(0), None)?;
        return Ok(Done::Finished(Finished {
            pass_run_id,
            proposer_run_id,
            judge_run_ids: Vec::new(),
            branch: blocked_branch,
            verdict: PanelVerdict::Blocked {
                reasons: vec![format!("patch size {count} exceeds {PATCH_LINE_CAP}")],
            },
            approved: None,
            pass_record,
        }));
    }

    if let Some(path) = protected_path_hit(&patch.paths, &repo) {
        let message = commit_message(
            &pass_run_id,
            &proposer_run_id,
            &[],
            &format!("refused at admission: protected path {path}"),
        );
        commit_pass_branch(&repo, &proposer, &blocked_branch, &message)?;
        write_harness_end(&pass_record, Some(0), None)?;
        return Ok(Done::Finished(Finished {
            pass_run_id,
            proposer_run_id,
            judge_run_ids: Vec::new(),
            branch: blocked_branch,
            verdict: PanelVerdict::Blocked {
                reasons: vec![format!("protected path {path}")],
            },
            approved: None,
            pass_record,
        }));
    }

    if let Err(output) = admission_gate(&patch.worktree) {
        let message = commit_message(
            &pass_run_id,
            &proposer_run_id,
            &[],
            &format!("admission gate failed:\n\n{output}"),
        );
        commit_pass_branch(&repo, &proposer, &blocked_branch, &message)?;
        write_harness_end(&pass_record, Some(0), None)?;
        return Ok(Done::Finished(Finished {
            pass_run_id,
            proposer_run_id,
            judge_run_ids: Vec::new(),
            branch: blocked_branch,
            verdict: PanelVerdict::Blocked {
                reasons: vec!["admission gate failed".into()],
            },
            approved: None,
            pass_record,
        }));
    }

    let judge_prompt = judge_task(&patch.diff);
    let judges = [
        (Provider::Claude, "claude"),
        (Provider::CursorAgent, "cursor-agent"),
        (Provider::Agy, "agy"),
    ];
    let mut judge_run_ids = Vec::new();
    let mut outcomes = Vec::new();

    for (provider, name) in judges {
        let judge_run_id = Uuid::now_v7().to_string();
        // `--model` names a model inside one provider, and the pass's flag was
        // typed for the proposer, which is `claude`. The `claude` judge takes it
        // too, so a haiku pass gets a haiku judge rather than the human's
        // default model; the other two providers run their own defaults, since
        // `cursor-agent --model haiku` names nothing there.
        let judge_model = if provider == Provider::Claude {
            proposer_model.clone()
        } else {
            None
        };
        let call = run(Request {
            provider,
            task: judge_prompt.clone(),
            model: judge_model,
            policy: "judge".to_string(),
            floor_lifted: false,
            timeout: pass.timeout,
            parent: Some(pass_run_id.clone()),
            run_id: Some(judge_run_id.clone()),
            command: "run".to_string(),
            resumed_from: None,
            resume_session_id: None,
            working_dir: None,
        })?;

        let result = match call {
            Call::Ended(result) => *result,
            Call::Handle(_) => {
                write_harness_end(&pass_record, None, Some("a judge outlived the call"))?;
                bail!("judge {name} outlived the call");
            }
        };

        if result.outcome == Some("rate_limited") || result.outcome == Some("auth_failed") {
            let why = result
                .error
                .clone()
                .unwrap_or_else(|| format!("judge {name} could not run"));
            write_harness_end(&pass_record, None, Some(&why))?;
            bail!(why);
        }

        judge_run_ids.push(judge_run_id.clone());
        let text = result.result_text.unwrap_or_default();
        outcomes.push(review::parse_judge_answer(name, &text));
    }

    let verdict = review::panel_verdict(&pass_run_id, &outcomes);
    let (branch, approved, body) = match &verdict {
        PanelVerdict::Offerable(patch) => (
            offerable_branch.clone(),
            Some(patch.clone()),
            verdict_body(&outcomes, true),
        ),
        PanelVerdict::Blocked { .. } => {
            (blocked_branch.clone(), None, verdict_body(&outcomes, false))
        }
    };
    let message = commit_message(&pass_run_id, &proposer_run_id, &judge_run_ids, &body);
    commit_pass_branch(&repo, &proposer, &branch, &message)?;
    write_harness_end(&pass_record, Some(0), None)?;

    Ok(Done::Finished(Finished {
        pass_run_id,
        proposer_run_id,
        judge_run_ids,
        branch,
        verdict,
        approved,
        pass_record,
    }))
}

/// What the proposer is told. Contract 5.1 stores this verbatim as `run_start.task`.
pub fn brief(
    pass_run_id: &str,
    layout: &Layout,
    selection: &corpus::Selection,
    repo: &Path,
) -> String {
    let mut text = String::new();
    text.push_str(
        "You are Ouroboros rung two, Hydra's reviewed self-editor. A human typed \
         `hydra evolve`. You edit Hydra's own Rust in a worktree and produce a patch \
         for review.\n\n",
    );
    text.push_str("## What you may write\n\n");
    text.push_str(&format!(
        "Your run's worktree under {}, branch hydra/<your-run-id>, and nothing else. \
         The gate refuses writes outside it. When you exit, Hydra reads your diff against \
         the tree it seeded you with.\n\n",
        layout.worktrees().display(),
    ));
    text.push_str(&format!(
        "The repository root is {}. Do not touch `policies/`, `config/`, `docs/`, \
         `.scratch/`, `IMPLEMENTATION.md` or `CLAUDE.md`; a patch naming any protected \
         path is refused before review.\n\n",
        repo.display(),
    ));
    text.push_str(&format!(
        "Keep the patch under {PATCH_LINE_CAP} lines changed. A larger diff is rejected \
         for size and you are told to split it.\n\n",
    ));
    text.push_str("## What you may read\n\n");
    text.push_str(
        "Your worktree, the Rust toolchain through `cargo`, and the corpus brief below. \
         Run records reach you as this brief, never as a directory you can open.\n\n",
    );
    text.push_str("## Corpus brief\n\n");
    text.push_str(&format!(
        "{} run(s) rung one may learn from. Pass id: {pass_run_id}.\n\n",
        selection.learn.len(),
    ));
    for run in &selection.learn {
        text.push_str(&format!(
            "  {}  {}  {}/{}\n",
            run.run_id,
            run.provider,
            run.outcome_name(),
            run.source_name(),
        ));
    }
    if selection.learn.is_empty() {
        text.push_str("  (none)\n");
    }
    text.push_str(
        "\nPropose a patch that improves Hydra from what the corpus shows. Run \
         `cargo fmt`, `cargo clippy -D warnings`, `cargo build` and `cargo test` before \
         you exit; the admission gate runs them again after you.\n",
    );
    text
}

pub struct ProposerPatch {
    pub worktree: PathBuf,
    pub base_tree: String,
    pub stat: Stat,
    pub paths: Vec<String>,
    pub diff: String,
}

/// Reads a finished proposer's diff the way `hydra reap` reads work: against
/// `base_tree` on the record.
pub fn read_proposer_patch(
    layout: &Layout,
    policies: &PolicySet,
    result: &RunResult,
) -> Result<ProposerPatch> {
    let text = std::fs::read_to_string(&result.run_file)
        .with_context(|| format!("reading {}", result.run_file))?;
    let run = crate::normalize::read_with(&text, policies, layout)?;
    let worktree_path = run
        .run_start()
        .and_then(|start| start.body["worktree"]["path"].as_str())
        .context("the proposer record has no worktree")?;
    let base_tree = run
        .run_start()
        .and_then(|start| start.body["worktree"]["base_tree"].as_str())
        .or(result
            .diff_stat
            .as_ref()
            .map(|stat| stat.base_tree.as_str()))
        .context("the proposer record has no base_tree")?;
    let worktree = PathBuf::from(worktree_path);
    let index = layout.stat_index(&result.run_id);
    let stat = diff_stat(&worktree, &index, base_tree)?;
    let paths = changed_paths(&worktree, &index, base_tree)?;
    let diff = diff_patch(&worktree, &index, base_tree)?;
    Ok(ProposerPatch {
        worktree,
        base_tree: base_tree.to_string(),
        stat,
        paths,
        diff,
    })
}

/// Returns the line count when the patch exceeds [`PATCH_LINE_CAP`].
pub fn patch_over_cap(stat: &Stat) -> Option<usize> {
    let count = stat.insertions as usize + stat.deletions as usize;
    (count > PATCH_LINE_CAP).then_some(count)
}

/// Case-insensitive check against harness spec 15.1 paths and the `policies/` and
/// `config/` directories named in the ticket.
pub fn protected_path_hit(paths: &[String], repo: &Path) -> Option<String> {
    let protected = [
        repo.join("policies"),
        repo.join("config"),
        repo.join("config").join("preamble.md"),
        repo.join("config").join("mcp.md"),
        repo.join("config").join("export-allowlist.json"),
        repo.join("config").join("env-allowlist.json"),
    ];
    for path in paths {
        let file = if Path::new(path).is_absolute() {
            PathBuf::from(path)
        } else {
            repo.join(path)
        };
        for root in &protected {
            if crate::normalize::paths::inside(&file, root) {
                return Some(path.clone());
            }
        }
    }
    None
}

/// `cargo fmt --check`, `clippy`, `build` and `test` in the worktree, spec 4.5.
pub fn admission_gate(worktree: &Path) -> Result<(), String> {
    let steps: &[(&str, &[&str])] = &[
        ("cargo", &["fmt", "--check"]),
        // `--` before `-D`: without it cargo reads the flag as its own and
        // refuses, which is what blocked the second real pass, 01a0814a-c730 on
        // 2026-09-08, on a patch nobody had judged.
        ("cargo", &["clippy", "--", "-D", "warnings"]),
        ("cargo", &["build"]),
        ("cargo", &["test"]),
    ];
    for (bin, args) in steps {
        let out = Command::new(bin)
            .args(*args)
            .current_dir(worktree)
            .output()
            .map_err(|err| format!("{bin} {}: {err}", args.join(" ")))?;
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            let stdout = String::from_utf8_lossy(&out.stdout);
            return Err(format!(
                "{bin} {} failed:\n{stdout}{stderr}",
                args.join(" ")
            ));
        }
    }
    Ok(())
}

fn judge_task(diff: &str) -> String {
    format!(
        "Review this patch to Hydra's source.\n\n{}\n\n## Diff\n\n```diff\n{}\n```\n",
        review::checklist_prompt(),
        diff
    )
}

fn verdict_body(outcomes: &[JudgeOutcome], offered: bool) -> String {
    let mut text = if offered {
        String::from("verdict: offerable\n\n")
    } else {
        String::from("verdict: blocked\n\n")
    };
    for outcome in outcomes {
        text.push_str(&format!("## {}\n", outcome.provider));
        if let Some(err) = &outcome.parse_error {
            text.push_str(&format!("parse error: {err}\n"));
            continue;
        }
        text.push_str(&format!(
            "verdict: {}\n",
            if outcome.verdict == review::VerdictField::Pass {
                "pass"
            } else {
                "block"
            }
        ));
        for finding in &outcome.findings {
            let sev = match finding.severity {
                review::Severity::Critical => "critical",
                review::Severity::Major => "major",
                review::Severity::Minor => "minor",
            };
            text.push_str(&format!(
                "- {} ({}) — {}\n",
                finding.item, sev, finding.note
            ));
        }
        text.push('\n');
    }
    text
}

fn commit_message(
    pass_run_id: &str,
    proposer_run_id: &str,
    judge_run_ids: &[String],
    body: &str,
) -> String {
    let mut text = format!("hydra evolve pass {pass_run_id}\n\n");
    text.push_str(&format!("proposer: {proposer_run_id}\n"));
    if judge_run_ids.is_empty() {
        text.push_str("judges: (none)\n\n");
    } else {
        text.push_str(&format!(
            "judges: {}, {}, {}\n\n",
            judge_run_ids[0], judge_run_ids[1], judge_run_ids[2]
        ));
    }
    text.push_str(body);
    text
}

fn commit_pass_branch(
    repo: &Path,
    proposer: &RunResult,
    branch: &str,
    message: &str,
) -> Result<()> {
    let policies = PolicySet::discover()?;
    let run = crate::normalize::read_file(std::path::Path::new(&proposer.run_file), &policies)?;
    let worktree_path = run
        .run_start()
        .and_then(|start| start.body["worktree"]["path"].as_str())
        .context("proposer worktree missing")?;
    let checkout = PathBuf::from(worktree_path);
    let exclude = crate::provider::own_worktree_paths();
    // Spec 4.9: every pass ends with exactly one branch and the verdict lives in
    // its commit message, so a pass whose proposer left the tree equal to HEAD
    // still gets a commit, an empty one carrying the message. The first real
    // pass, 01a08129-de0c on 2026-09-08, had a proposer that undid the seeded
    // uncommitted work and nothing else, `commit_everything` found nothing, and
    // the blocked branch pointed at master with no verdict anywhere.
    if crate::worktree::commit_everything(&checkout, message, &exclude)?.is_none() {
        git(
            &checkout,
            &[
                "commit",
                "-q",
                "--allow-empty",
                "--no-verify",
                "-m",
                message,
            ],
        )?;
    }
    // The verdict commit is the checkout's HEAD, on `hydra/<proposer>`. The
    // pass branch has to point at that commit, not at the parent repo's HEAD:
    // the second real pass, 01a0814a-c730 on 2026-09-08, made the commit and
    // then named master, and the blocked branch carried master's last message
    // as its verdict.
    let verdict_commit = git(&checkout, &["rev-parse", "HEAD"])?;
    git(repo, &["branch", "-f", branch, verdict_commit.trim()])?;
    Ok(())
}

pub fn write_harness_start(layout: &Layout, pass_run_id: &str, path: &Path) -> Result<()> {
    layout.ensure()?;
    let pid = std::process::id();
    // Microseconds, the precision `watch.rs` writes and `procinfo::is_alive`
    // compares by exact equality: the first real pass, 01a08129-de0c on
    // 2026-09-08, wrote milliseconds and `hydra status` reported the pass
    // abandoned while its process was alive. No fallback to the clock either,
    // for the same reason: a time that is not the process's is worse than null.
    let start_time = crate::procinfo::start_time(pid)?
        .map(|t| t.to_rfc3339_opts(chrono::SecondsFormat::Micros, true));
    let start = RunStart {
        kind: "run_start",
        schema_version: SCHEMA_VERSION,
        hydra_version: env!("CARGO_PKG_VERSION").to_string(),
        command: "evolve".to_string(),
        launch_mode: None,
        provider_version: None,
        provider_session_id: None,
        task: None,
        policy: None,
        policy_sha: None,
        floor_lifted: None,
        scopes: None,
        cwd: std::env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| ".".into()),
        worktree: None,
        depth: 0,
        parent_policy: None,
        pid: Some(pid),
        process_start_time: start_time,
        skills: None,
        mcp: None,
        auth: None,
        env_allowlist: None,
        subscription: None,
        resumed_from: None,
    };
    let mut writer = Writer::create(path, pass_run_id, None, None, None)?;
    writer.hydra_line(&start)?;
    Ok(())
}

pub fn write_harness_end(path: &Path, exit_code: Option<i32>, error: Option<&str>) -> Result<()> {
    let mut writer = Writer::resume(
        path,
        path.file_stem().and_then(|s| s.to_str()).unwrap_or(""),
        None,
        None,
        None,
        crate::record::next_seq(path)?,
    )?;
    writer.hydra_line(&RunEnd::streamed(exit_code, error.map(str::to_string)))?;
    Ok(())
}

fn rate_limit_abort(layout: &Layout) -> Result<Option<String>> {
    let runs_dir = layout.runs();
    if !runs_dir.is_dir() {
        return Ok(None);
    }
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&runs_dir)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("jsonl"))
        .collect();
    paths.sort_by(|a, b| b.cmp(a));
    for path in paths {
        let text = std::fs::read_to_string(&path)?;
        let provider = text
            .lines()
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
            .find_map(|envelope| {
                envelope["provider"]
                    .as_str()
                    .map(|provider| provider.to_string())
            });
        if provider.as_deref() != Some("claude") {
            continue;
        }
        for line in text.lines() {
            let Ok(envelope) = serde_json::from_str::<serde_json::Value>(line) else {
                continue;
            };
            let Some(raw) = envelope.get("raw") else {
                continue;
            };
            if raw.get("type").and_then(|v| v.as_str()) != Some("rate_limit_event") {
                continue;
            }
            if let Some(status) = raw.get("status").and_then(|v| v.as_str())
                && matches!(status, "rejected" | "allowed_warning")
            {
                return Ok(Some(format!(
                    "the last claude run is rate-limited ({status}); log in or wait before reviewing"
                )));
            }
        }
        break;
    }
    Ok(None)
}

fn changed_paths(worktree: &Path, index: &Path, base: &str) -> Result<Vec<String>> {
    let head = seed_tree(worktree, index)?;
    let names = git(worktree, &["diff-tree", "-r", "--name-only", base, &head])?;
    Ok(names
        .lines()
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect())
}

fn diff_patch(worktree: &Path, index: &Path, base: &str) -> Result<String> {
    let head = seed_tree(worktree, index)?;
    git(worktree, &["diff", base, &head])
}

/// What a human reads back.
pub fn report(done: &Done) -> String {
    match done {
        Done::Considered { brief } => format!("{brief}\n--\nNothing was launched.\n"),
        Done::Handed { pass_run_id } => {
            format!("pass {pass_run_id} outlived the call; watch children with `hydra status`\n")
        }
        Done::Finished(finished) => {
            let mut text = format!(
                "evolve pass {}\n  proposer {}\n  record {}\n  branch {}\n",
                finished.pass_run_id,
                finished.proposer_run_id,
                finished.pass_record.display(),
                finished.branch,
            );
            match &finished.verdict {
                PanelVerdict::Offerable(_) => {
                    text.push_str("  verdict: offerable; merge the branch or do not\n");
                }
                PanelVerdict::Blocked { reasons } => {
                    text.push_str("  verdict: blocked\n");
                    for reason in reasons {
                        text.push_str(&format!("    {reason}\n"));
                    }
                }
            }
            text
        }
    }
}
