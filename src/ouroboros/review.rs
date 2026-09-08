//! The rung-two review panel: judge answers, the critical-threshold rule and
//! [`ApprovedPatch`].
//!
//! `docs/ouroboros-spec.md` sections 2.1, 4.6 to 4.8, 5 and 5.1 are the design.
//! Nothing outside this module may construct an [`ApprovedPatch`]; the apply path
//! is unreachable rather than merely gated.

use serde::Deserialize;

/// A patch the panel offered for merge. Only [`panel_verdict`] returns this.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovedPatch {
    pass_run_id: String,
}

impl ApprovedPatch {
    pub fn pass_run_id(&self) -> &str {
        &self.pass_run_id
    }
}

/// One judge's closed JSON answer after parsing and the critical-threshold rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JudgeOutcome {
    pub provider: &'static str,
    pub verdict: VerdictField,
    pub findings: Vec<Finding>,
    /// Set when the response did not parse or an item name was unknown.
    pub parse_error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerdictField {
    Pass,
    Block,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub item: String,
    pub severity: Severity,
    pub note: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Critical,
    Major,
    Minor,
}

/// What three judges decided together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PanelVerdict {
    Offerable(ApprovedPatch),
    Blocked { reasons: Vec<String> },
}

#[derive(Deserialize)]
struct RawAnswer {
    verdict: String,
    findings: Vec<RawFinding>,
}

#[derive(Deserialize)]
struct RawFinding {
    item: String,
    severity: String,
    note: String,
}

/// Every checklist id a judge may name, ouroboros spec 5.
pub const CHECKLIST_IDS: &[&str] = &[
    "secrets_check",
    "security_issues",
    "tests_affected",
    "self_consistency",
    "code_quality",
    "cross_platform",
    "changelog_accuracy",
    "approved_patch_unreachable",
    "severity_ceiling_table",
    "reviewer_failure_fails_closed",
    "panel_size_unshrunk",
    "protected_path_set",
    "admission_gate_order",
    "baseline_test_provenance",
    "env_allowlist_intact",
    "hook_config_written_fresh",
    "hook_dialect_coverage",
    "billed_spend_gate",
    "delegation_depth_cap",
    "curator_write_surface",
    "evidence_is_read_only",
    "worktree_containment",
    "record_append_only",
    "record_source_fields",
    "raw_frame_preserved",
    "export_losslessness",
    "provider_claim_dated",
    "no_new_crossing",
    "no_daemon",
    "spec_matches_code",
];

fn ceiling(item: &str) -> Option<Severity> {
    match item {
        "code_quality" | "cross_platform" | "changelog_accuracy" | "export_losslessness"
        | "provider_claim_dated" | "spec_matches_code" => Some(Severity::Minor),
        _ if CHECKLIST_IDS.contains(&item) => Some(Severity::Critical),
        _ => None,
    }
}

/// Parses one judge's whole response as the closed JSON shape. Any deviation is a
/// block carrying the parse error, never retried and never read as a pass.
pub fn parse_judge_answer(provider: &'static str, text: &str) -> JudgeOutcome {
    let trimmed = text.trim();
    let parsed: RawAnswer = match serde_json::from_str(trimmed) {
        Ok(value) => value,
        Err(err) => {
            return JudgeOutcome {
                provider,
                verdict: VerdictField::Block,
                findings: Vec::new(),
                parse_error: Some(err.to_string()),
            };
        }
    };

    let verdict = match parsed.verdict.as_str() {
        "pass" => VerdictField::Pass,
        "block" => VerdictField::Block,
        other => {
            return JudgeOutcome {
                provider,
                verdict: VerdictField::Block,
                findings: Vec::new(),
                parse_error: Some(format!("verdict {:?} is not pass or block", other)),
            };
        }
    };

    let mut findings = Vec::new();
    for raw in parsed.findings {
        let Some(max) = ceiling(&raw.item) else {
            return JudgeOutcome {
                provider,
                verdict: VerdictField::Block,
                findings: Vec::new(),
                parse_error: Some(format!("unknown checklist item {:?}", raw.item)),
            };
        };
        let severity = match raw.severity.as_str() {
            "critical" => Severity::Critical,
            "major" => Severity::Major,
            "minor" => Severity::Minor,
            other => {
                return JudgeOutcome {
                    provider,
                    verdict: VerdictField::Block,
                    findings: Vec::new(),
                    parse_error: Some(format!("severity {:?} is not critical, major or minor", other)),
                };
            }
        };
        let severity = clamp_severity(severity, max);
        findings.push(Finding {
            item: raw.item,
            severity,
            note: raw.note,
        });
    }

    JudgeOutcome {
        provider,
        verdict,
        findings,
        parse_error: None,
    }
}

fn clamp_severity(reported: Severity, max: Severity) -> Severity {
    if severity_rank(reported) > severity_rank(max) {
        max
    } else {
        reported
    }
}

fn severity_rank(severity: Severity) -> u8 {
    match severity {
        Severity::Critical => 3,
        Severity::Major => 2,
        Severity::Minor => 1,
    }
}

/// Whether one outcome blocks whatever the verdict field says: a parse failure,
/// an explicit block, or any critical finding (spec 5.1's rule copied in 4.8).
pub fn outcome_blocks(outcome: &JudgeOutcome) -> bool {
    if outcome.parse_error.is_some() {
        return true;
    }
    if outcome.verdict == VerdictField::Block {
        return true;
    }
    outcome
        .findings
        .iter()
        .any(|finding| finding.severity == Severity::Critical)
}

/// Turns three judge outcomes into a panel verdict. Fewer than three is a block.
pub fn panel_verdict(pass_run_id: &str, outcomes: &[JudgeOutcome]) -> PanelVerdict {
    if outcomes.len() != 3 {
        return PanelVerdict::Blocked {
            reasons: vec![format!(
                "the panel needs three answers and got {}",
                outcomes.len()
            )],
        };
    }

    let mut reasons = Vec::new();
    for outcome in outcomes {
        if let Some(err) = &outcome.parse_error {
            reasons.push(format!("{}: parse error: {err}", outcome.provider));
            continue;
        }
        if outcome.verdict == VerdictField::Block {
            reasons.push(format!("{}: verdict block", outcome.provider));
        }
        for finding in &outcome.findings {
            if finding.severity == Severity::Critical {
                reasons.push(format!(
                    "{}: critical {} — {}",
                    outcome.provider,
                    finding.item,
                    finding.note
                ));
            }
        }
    }

    if reasons.is_empty() {
        PanelVerdict::Offerable(ApprovedPatch {
            pass_run_id: pass_run_id.to_string(),
        })
    } else {
        PanelVerdict::Blocked { reasons }
    }
}

/// The checklist text judges see in the prompt, ouroboros spec 5.
pub fn checklist_prompt() -> &'static str {
    CHECKLIST_PROMPT
}

const CHECKLIST_PROMPT: &str = r#"Review the diff against this checklist. Answer in exactly one JSON object and nothing else:

{"verdict": "pass" | "block", "findings": [{"item": <checklist id>, "severity": "critical" | "major" | "minor", "note": <string>}]}

Before marking any item critical you must be able to answer yes to all of:
1. You can name the exact file, symbol, function, test, or config path in this repository that makes this problem live right now.
2. That artifact actually appears in the diff or the touched-file context you have been given.
3. The fix requires a change to this diff, not a follow-up task.

If you cannot satisfy all three, use advisory severity, not critical. One root cause is one finding.

Checklist ids and ceilings:
- secrets_check (critical)
- security_issues (critical)
- tests_affected (critical)
- self_consistency (critical)
- code_quality (advisory)
- cross_platform (advisory)
- changelog_accuracy (advisory)
- approved_patch_unreachable (critical)
- severity_ceiling_table (critical)
- reviewer_failure_fails_closed (critical)
- panel_size_unshrunk (critical)
- protected_path_set (critical)
- admission_gate_order (critical)
- baseline_test_provenance (critical)
- env_allowlist_intact (critical)
- hook_config_written_fresh (critical)
- hook_dialect_coverage (critical)
- billed_spend_gate (critical)
- delegation_depth_cap (critical)
- curator_write_surface (critical)
- evidence_is_read_only (critical)
- worktree_containment (critical)
- record_append_only (critical)
- record_source_fields (critical)
- raw_frame_preserved (critical)
- export_losslessness (advisory)
- provider_claim_dated (advisory)
- no_new_crossing (critical)
- no_daemon (critical)
- spec_matches_code (advisory)
"#;
