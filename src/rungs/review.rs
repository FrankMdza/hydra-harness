//! The rung-two review panel: judge answers, the critical-threshold rule and
//! [`ApprovedPatch`].
//!
//! `docs/rungs-spec.md` sections 2.1, 4.6 to 4.8, 5 and 5.1 are the design.
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

/// What four judges decided together.
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

/// One row of the review checklist, rungs spec section 5.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChecklistEntry {
    pub id: &'static str,
    pub advisory: bool,
    pub definition: &'static str,
}

/// Every checklist item with its ceiling and definition. Section 5.2 and 5.3.
pub const CHECKLIST_ENTRIES: &[ChecklistEntry] = &[
    ChecklistEntry {
        id: "secrets_check",
        advisory: false,
        definition: "A credential, token, key, or a literal path to one, entering source, tests or fixtures",
    },
    ChecklistEntry {
        id: "security_issues",
        advisory: false,
        definition: "A named unsafe construct: unchecked path join, shell interpolation of untrusted text, a widened `unsafe` block",
    },
    ChecklistEntry {
        id: "tests_affected",
        advisory: false,
        definition: "A behaviour change with no test touched, naming the changed function and the test file that should have moved",
    },
    ChecklistEntry {
        id: "self_consistency",
        advisory: false,
        definition: "The diff doing something its own commit message does not claim",
    },
    ChecklistEntry {
        id: "code_quality",
        advisory: true,
        definition: "Duplication, dead code, a panic path on a recoverable error",
    },
    ChecklistEntry {
        id: "cross_platform",
        advisory: true,
        definition: "Case sensitivity, path separators, an assumption that `HOME` looks like this machine's",
    },
    ChecklistEntry {
        id: "changelog_accuracy",
        advisory: true,
        definition: "A user-visible change missing from the changelog entry",
    },
    ChecklistEntry {
        id: "approved_patch_unreachable",
        advisory: false,
        definition: "A new constructor, a widened visibility, or a test helper that builds an `ApprovedPatch` outside the review module",
    },
    ChecklistEntry {
        id: "severity_ceiling_table",
        advisory: false,
        definition: "An item's ceiling raised, an item losing its ceiling, or a path carrying a model-supplied severity through unclamped",
    },
    ChecklistEntry {
        id: "reviewer_failure_fails_closed",
        advisory: false,
        definition: "A parse failure, timeout or refusal resolving to anything other than reviewer-failed. `[]` inside refusal prose is the case that produced this item",
    },
    ChecklistEntry {
        id: "panel_size_unshrunk",
        advisory: false,
        definition: "Any code that yields a verdict with fewer than the configured judges answering",
    },
    ChecklistEntry {
        id: "protected_path_set",
        advisory: false,
        definition: "An entry removed or narrowed, matching made case-sensitive, or the refusal moved to after the review",
    },
    ChecklistEntry {
        id: "admission_gate_order",
        advisory: false,
        definition: "A judge dispatched before fmt, clippy, build and test have passed, or an admission failure that still spends review budget",
    },
    ChecklistEntry {
        id: "baseline_test_provenance",
        advisory: false,
        definition: "A protected test read from the patched worktree rather than from the baseline sha",
    },
    ChecklistEntry {
        id: "env_allowlist_intact",
        advisory: false,
        definition: "A variable added to the allowlist without a named reason. `NODE_OPTIONS`, `PYTHONPATH` or `GEMINI_API_KEY` reaching a child at all",
    },
    ChecklistEntry {
        id: "hook_config_written_fresh",
        advisory: false,
        definition: "A hook config linked rather than copied, written once rather than per spawn, in the wrong per-provider location, or an `agy` spawn missing `--add-dir`",
    },
    ChecklistEntry {
        id: "hook_dialect_coverage",
        advisory: false,
        definition: "A gate script that answers in fewer than the three dialects it must, or that infers the dialect rather than taking it as an argument. `claude` fails open on a shape it does not recognise, so a missing branch is a silent hole rather than an error",
    },
    ChecklistEntry {
        id: "billed_spend_gate",
        advisory: false,
        definition: "A per-run human flag becoming inheritable, settable by the delegation policy, readable from the environment, or appearing on the MCP delegate face",
    },
    ChecklistEntry {
        id: "delegation_depth_cap",
        advisory: false,
        definition: "The depth cap removed or made unreachable, or the MCP face gaining `--shared-tree`, a timeout override, depth or `model`",
    },
    ChecklistEntry {
        id: "curator_write_surface",
        advisory: false,
        definition: "Rung one gaining a write path outside the skill store, above all to config, the policy or the allowlist",
    },
    ChecklistEntry {
        id: "evidence_is_read_only",
        advisory: false,
        definition: "Rungs gaining a call to `prune`, `reap` or `export`, or any write under `~/hydra/runs/`. For the three commands this item is the only mechanism, per section 2.2, so a reviewer may not defer to the gate here",
    },
    ChecklistEntry {
        id: "worktree_containment",
        advisory: false,
        definition: "A proposer path that reads or writes outside its own worktree, or a gate whose path matching can be escaped by a relative path or a symlink",
    },
    ChecklistEntry {
        id: "record_append_only",
        advisory: false,
        definition: "A second concurrent writer to a run file, an in-place rewrite, an ownership transfer that skips the liveness test, or a run id that is not a UUIDv7",
    },
    ChecklistEntry {
        id: "record_source_fields",
        advisory: false,
        definition: "`usage_source`, `outcome_source` or `permission_decision.decider` gaining an undocumented value or losing one",
    },
    ChecklistEntry {
        id: "raw_frame_preserved",
        advisory: false,
        definition: "A derived field overwriting the verbatim provider frame, or normalization dropping a frame it could not classify",
    },
    ChecklistEntry {
        id: "export_losslessness",
        advisory: true,
        definition: "A record change that cannot survive the SFT and DPO export in D12, or redaction moving out of export",
    },
    ChecklistEntry {
        id: "provider_claim_dated",
        advisory: true,
        definition: "A provider-specific assertion added to code or docs without the version and date it was observed",
    },
    ChecklistEntry {
        id: "no_new_crossing",
        advisory: false,
        definition: "Anything crossing between the harness and Rungs other than the run record and the skill store: a shared module, a database, a socket (D5)",
    },
    ChecklistEntry {
        id: "no_daemon",
        advisory: false,
        definition: "A scheduler, background service, timer, or anything that must be running for the harness to work (D10)",
    },
    ChecklistEntry {
        id: "spec_matches_code",
        advisory: true,
        definition: "A behaviour change that leaves `docs/harness-spec.md`, `docs/rungs-spec.md` or `docs/run-record-contract.md` promising the old behaviour",
    },
];

/// Every checklist id a judge may name, rungs spec 5.
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
    CHECKLIST_ENTRIES
        .iter()
        .find(|entry| entry.id == item)
        .map(|entry| {
            if entry.advisory {
                Severity::Minor
            } else {
                Severity::Critical
            }
        })
}

/// Parses one judge's whole response as the closed JSON shape. Any deviation is a
/// block carrying the parse error, never retried and never read as a pass.
pub fn parse_judge_answer(provider: &'static str, text: &str) -> JudgeOutcome {
    let trimmed = strip_one_fence(text.trim());
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
                    parse_error: Some(format!(
                        "severity {:?} is not critical, major or minor",
                        other
                    )),
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

/// One code fence wrapping the whole answer, and nothing else.
///
/// Map ticket 52 chose one closed shape where any deviation blocks, and the
/// first real panel, pass 01a08241-831d on 2026-09-08, showed what that costs:
/// the `claude` judge returned exactly the object asked for inside a ```json
/// fence and was recorded as a block for a syntax error. Wrapping JSON in a
/// fence is what a model does by default, so the fence comes off and everything
/// else still blocks: prose beside the object, two fences, a fence that opens
/// and does not close. The human decided this on the same day, against the
/// reference implementation's sentinel, which ticket 52 had already rejected.
fn strip_one_fence(text: &str) -> &str {
    let Some(rest) = text.strip_prefix("```") else {
        return text;
    };
    // ```json, ```JSON or a bare ```: the tag runs to the first newline.
    let Some((tag, body)) = rest.split_once('\n') else {
        return text;
    };
    if !tag.trim().is_empty() && !tag.trim().eq_ignore_ascii_case("json") {
        return text;
    }
    let Some(body) = body.trim_end().strip_suffix("```") else {
        return text;
    };
    // A second fence anywhere inside is not one wrapper, so the answer stands
    // as it came and the parse below refuses it.
    if body.contains("```") {
        return text;
    }
    body.trim()
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

/// Turns four judge outcomes into a panel verdict. Fewer than four is a block.
pub fn panel_verdict(pass_run_id: &str, outcomes: &[JudgeOutcome]) -> PanelVerdict {
    if outcomes.len() != 4 {
        return PanelVerdict::Blocked {
            reasons: vec![format!(
                "the panel needs four answers and got {}",
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
                    outcome.provider, finding.item, finding.note
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

/// The checklist text judges see in the prompt, rungs spec 5.
pub fn checklist_prompt() -> String {
    let mut text = String::from(
        "Review the diff against this checklist. Answer in exactly one JSON object and nothing else:\n\n\
         {\"verdict\": \"pass\" | \"block\", \"findings\": [{\"item\": <checklist id>, \"severity\": \"critical\" | \"major\" | \"minor\", \"note\": <string>}]}\n\n\
         Before marking any item critical you must be able to answer yes to all of:\n\
         1. You can name the exact file, symbol, function, test, or config path in this repository that makes this problem live right now.\n\
         2. That artifact actually appears in the diff or the touched-file context you have been given.\n\
         3. The fix requires a change to this diff, not a follow-up task.\n\n\
         If you cannot satisfy all three, use advisory severity, not critical. One root cause is one finding.\n\n\
         Checklist items:\n",
    );
    for entry in CHECKLIST_ENTRIES {
        let ceiling = if entry.advisory {
            "advisory"
        } else {
            "critical"
        };
        text.push_str(&format!(
            "- {} ({}): {}\n",
            entry.id, ceiling, entry.definition
        ));
    }
    text
}
