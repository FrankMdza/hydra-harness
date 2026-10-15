# Map 130 — outside engineer's seat, 2026-10-15

I would trust this repository for publishing its limits and supplying reproducible evidence. I would distrust a green badge obtained by silently dropping failures, a record described as proof when its contents can be forged, or cleanup that changes repository semantics without consent. These are recommendations for the human, not decisions closing the originating tickets. “Checked” below means read in this checkout; it does not mean executed.

## 1. Linux CI and the platform decisions

**Recommendation:** narrow the required Linux job to a demonstrably passing scope, disclose that scope beside the badge, and open the eight follow-up groups. Keep the full failing command reproducible and its failures linked; do not make complete Linux support a prerequisite to publishing a macOS project. In particular, do not adopt the readings' proposed `--lib --workspace` command on faith: `crates/puppets/src/sheet.rs:644` is an exact assertion inside a library unit test, one of the reported Linux failures. A build-only job is the defensible initial fallback until the selected tests actually pass on Linux.

**Checked versus claimed:** `.github/workflows/ci.yml` contains the reported Ubuntu build and workspace test command with `--skip sandbox`; README explicitly limits runtime measurement to macOS. `tests/seats.rs:47` really hardcodes the macOS temporary root, and `tests/corpus/mod.rs:675` really panics on any `CI` environment when sandbox-exec is unavailable. `tests/git_config.rs:353` unwraps the commit result, while `src/worktree.rs:266–278` supplies only configured identity values, with no fallback. `tests/gate.rs:3464` really asserts allow for the NUL-cwd payload. `tests/pty_tee.rs:174` checks that each delivered line parses, but that assertion alone cannot identify the defect; investigate whether the shell emitted a valid complete frame as well as tee reassembly. `src/record/validate.rs:31` is the generic report assertion, not a diagnosis of Linux's empty type. The held-slave explanation is present in `tests/detached.rs:372` onward.

`crates/puppets/src/sheet.rs:644` really demands exact lgamma values; its nearby near-tie test shows ranking can depend on tiny differences. `crates/puppets/tests/differential.rs:3863` and `crates/grader/tests/differential.rs:147` are mismatch assertions, not evidence of the particular CI counts or causes. The launch fixtures do contain the Apple environment variable. The readings' 51 failures, Python versions, 4 KiB delivery, FMA attribution and shell-error explanation remain reported observations, not reproduced findings here.

For **c**, use a visibly synthetic fallback such as `Hydra <hydra@users.invalid>` when no identity is configured; preserve a configured identity, disclose fallback use, and define partial configuration explicitly. Do not guess a human name or email from the host. For **d**, reject an embedded NUL explicitly and consistently before filesystem resolution; malformed context should not receive an accidental allow because one Python tolerates it. For **g**, prefer platform-specific exact CPython compatibility tests over a blanket one-ulp allowance. First establish the actual error bound: the readings do not prove it is one ulp. Record the runtime/platform reference, test near-tie ordering, and state that cross-platform ranking equality is not yet guaranteed. If identical ordering across machines is required, that needs a separate canonical arithmetic decision.

**Closure:** Ubuntu evidence for the selected required command; one regression per group; identity tests with global, system and XDG configuration absent and partial identities; explicit NUL denial on both Python families; long-frame and descendant-lifetime probes on Linux; and numerical differentials including ranking boundaries. Narrowing CI is reversible; commits with misleading authorship and published rankings are much harder to correct after consumption.

## 2. Build 316: additive host and sandbox kind

**Recommendation:** retain the existing sandbox sha-or-null and add `sandbox_kind` and `host`. Specify combinations, absent legacy values, and the distinction between “no wrapper” and “reason no wrapper was used.” Hostname is descriptive metadata, not authenticated identity; explain its privacy cost before public export and avoid usernames.

**Checked versus claimed:** contract section 15 explicitly says adding a run_start field does not bump the version, while retyping one does. The present validator still lists `sandbox` as nullable `Shape::Sha` at `src/record/validate.rs:84`, and the writer passes an optional string in `src/run/mod.rs`. Thus the current checkout has not adopted the proposed object. Ticket 316 itself asks for an object while calling the amendment additive: that is inconsistent with section 15. A version-2 object could be legitimate, but would need an explicitly breaking contract change. The readings' unapplied 27-file patch, six tests and four regenerated fixtures were not available for inspection within this checkout.

**Closure:** validators and all four normalizers accept legacy records and additive records, reject inconsistent kind/sha combinations, and show distinguishes unknown legacy enforcement from explicit no-wrapper enforcement. Guard fixtures and walk the old corpus without new findings; the reported count of 952 is not itself verified here. Additive fields retain future options; publishing a second sandbox representation imposes continuing compatibility costs, and public hostnames cannot reliably be recalled.

## 3. Build 326: nested repositories at reap

**Recommendation:** refuse by path before committing or changing the checkout. Preserve work and metadata, identify every offending path, and explain the human's recovery options. A later explicit flatten operation can be added once its handling of history, ignored files and submodules is specified.

**Checked versus claimed:** `src/worktree.rs:743` defines the index repair, `:758` deliberately excludes mode 160000, and `:909` begins commit_everything, whose first operation is the dirty-status check. The two cleanup tests at `tests/cleanup.rs:1910` and `:1963` assert today's failures; they are regression tests of undesirable behavior, not necessarily failing tests. The readings conflate the staged error with the unstaged removal error. Build 325's Outcome and the assertions distinguish them: staged fails during git status; unstaged commits a gitlink and then fails removal with code 128. The unstaged test checks three newly unreachable objects. I did not execute either case.

Automatic flattening worries an outside engineer because a legitimate submodule is also a gitlink, `.git` can be a file, and renaming metadata to `.git.nested` before `git add -A` can publish configuration, hooks or repository internals. Renaming is not meaningfully reversible if reap subsequently deletes the checkout without preserving its complete object history.

**Closure:** staged, unstaged and two-repository cases all refuse with named paths and unchanged HEAD, index, files and metadata; include legitimate submodules, `.git` files, existing `.git.nested` names and symlink boundaries. Put preflight before object import if “untouched” includes the parent object store. Prefer a before/after object-store delta to demanding globally empty fsck output on an arbitrary repository. Refusal preserves the option to flatten later; flattening and deletion can destroy the option to recover history.

## 4. Build 321: standing of refusal evidence

**Recommendation:** accept the sidecar as unauthenticated supplementary diagnostics, explicitly excluded from authoritative provenance or automatic training/admission decisions without corroboration. Do not describe it simply as “written by the hook” without the qualification that a run can forge the same shape. Signing is a future design task if stronger provenance is required.

**Checked versus claimed:** `src/policy/gate.py:1782` onward builds the event and caps UTF-8 command bytes at 65,536. `src/sandbox.rs:238` names the literal file-write exception. `src/watch.rs:336` opens the sidecar without following symlinks, validates each JSON object's shape and writes it into the record; that is validation, not authentication. Contract 7.7 already states the tampering limitation and close-time timestamp semantics, contrary to any implication that this disclosure still needs to be invented. The two normal close paths are documented in ticket 321; its heredoc byte measurements and claimed green runs remain ticket reports.

Signing alone does not establish completeness: deleting signed lines still hides refusals, and a secret must actually be unavailable to every process the run controls. A trusted receiver, sequence/completeness design and key boundary matter more than adding a signature field.

**Closure:** adversarial end-to-end cases forge a valid line, truncate/delete/reorder evidence, and supply oversized or malformed evidence. Show and every consumer must retain the correct trust distinction, finish the record safely, and never promote forged diagnostics into verified decisions. This can ship as a documented diagnostic feature; a public claim of tamper-proof auditability would be difficult to undo.

## 5. Build 320: text or extra JSON keys

**Recommendation:** keep the three facts in text now. Open the compatibility probes only when there is a concrete structured-data consumer; machine-readable fields could instead live in Hydra's own stored evidence without altering vendor replies.

**Checked versus claimed:** `src/policy/gate.py:308–353` chooses the longest matching protected root and defines `longest`; the token/link helpers and fixed provider reply dictionaries support ticket 320's description. Harness spec 6.6 explicitly records Claude's fail-open response to an unrecognized dialect. That supports caution; it does not prove an extra key will fail open. The ticket's Outcome explicitly admits that the requested JSON fields were not implemented. The historical skill-test line numbers have shifted: around 2543/2601 now are other agy cases, while the linked-shell test is at `tests/gate.rs:2992` onward and actually creates links and asserts refusal.

**Closure:** existing facts tested across relevant shell/file paths; if keys are added, measure all four providers on pinned versions, including Cursor's distinct hook events and Codex's reply path. Preserve denial on unexpected responses and retain drift coverage. Text formatting is readily reversible; silently weakening a deny while improving its explanation can compromise work before anyone notices.

## 6. Map 126: foreign records and enforcement populations

**Recommendation to question 1:** count foreign records and expose their contents, with an explicit “not replay-verified here” classification. Do not let their self-reported host or layout authorize access to the reader's filesystem or establish trustworthy gate results. Missing host means unknown, not local. Same OS/hostname is not proof of replay compatibility.

**Recommendation to question 2:** split evidence by enforcement population and exclude hook-only and unknown-enforcement records from evidence supporting gate changes, for both curate and evolve. They remain useful for other analyses with their limits attached. Do not replace a missing isolation guarantee with an arbitrary numerical weight; adding a Linux wrapper later still requires measurement before promoting that population.

**Checked versus claimed:** `GateMark::of` in `src/normalize/mod.rs:748` compares a locally compiled gate hash and classifies mismatch as Foreign. However, “Foreign and never re-evaluated” is stronger than this enum alone proves. More importantly, contract 7.7 actually says stored scopes take precedence over reconstructed local ones; `ReplaySpawn::of` implements that while retaining the reader's Layout and deriving other inputs locally. Thus the readings' blanket statement that 7.7 forbids taking layout off the record is imprecise: stored scopes and wholesale foreign-layout trust are different. The claimed macOS realpath transformation was not reproduced.

**Closure:** foreign-host and missing-host fixtures, matching and mismatching hashes, malicious stored paths, and same-host/different-layout fixtures. A foreign walk must count records without replay-derived verification or unsafe filesystem access. Curate and evolve both need mixed-population tests proving excluded records cannot support a gate proposal. Population labels can be refined later; silently contaminated gate-learning evidence cannot be cleanly unwound once it produces changes.

## 7. Map 127: reading a pinned skill snapshot

**Recommendation to question 1:** eventually allow shell naming of this run's exact pinned snapshot only when an actual applicable kernel wrapper protects it. A non-null sandbox field alone is insufficient; it must describe an enforcement kind that supplies the required protection. Until implemented and measured, document file-tool reads for the providers that support them.

**Recommendation to question 2:** do not generalize this to every runtime directory the kernel permits reading. The snapshot has a specific purpose and pinned scope; sibling snapshots, records, spawn artifacts and credentials do not follow from it. Maintain explicit write denial independently of the shell token exemption.

**Recommendation to question 3:** today use the scoped file tools on cursor-agent/agy; any preamble amendment belongs to the human. Copy the protected skill where appropriate if provider capabilities require it. Do not make an unmeasured promise that a shell read works.

**Checked versus claimed:** the linked-shell test at `tests/gate.rs:2992` confirms allowed root listing versus refused linked-file reads, while the snapshot-scope tests cover file-tool scope boundaries. SECURITY says agy uses its file tool and dates Codex read-only copies to build 87; the readings say map 71, so that attribution needs correction rather than repetition. `src/policy/mod.rs:964` currently starts denied_spellings, not a dedicated snapshot carve-out; its own-path discussion explains the mechanism. These code/test readings support the practical distinction, not a fresh live-provider measurement.

**Closure:** both linked-skill shell cases flip only under the chosen wrapper; sibling reads and every write form stay denied; verify Linux/no-wrapper behavior, symlink escape, and real cursor-agent/agy reads recorded without refusal. This narrow policy adjustment is reversible; a general runtime exemption would grant much more than the usability problem requires.

## 8. Publication order and omissions

**Recommendation:** before the visibility flip, decide explicitly what personal history may become public, complete a real whole-history secret scan including relevant refs, inspect tracked notes/configuration for private data, prevent accidental inclusion of the personal task file, and diagnose security-workflow startup. Then establish honestly scoped CI, publish exact support/trust limits, review a release candidate from a clean clone, and tag that reviewed tree. Topics, social preview and a release cadence can follow. Git gc is maintenance, not a publication gate.

**Checked versus claimed:** README now discloses the Linux limitation. `.gitignore` does not name the personal task file. `config/README.md` describes absolute allowlist entries but not a complete per-machine setup story. The grader still formats a whole Sha256 digest with LowerHex in `crates/grader/src/sheet.rs:201`, supporting the claimed incompatibility location, not proving the untested dependency update. The security workflow disables SARIF upload already, so adding security-events permission is not an established fix; obtain the startup diagnostic first. CHANGELOG's Unreleased section already includes 2026-10-15 entries, contradicting research 129's stale “stops on 2026-09-22” premise. Its proposed v0.1.0 notes need review, not just renaming a heading. `IMPLEMENTATION.md:317` now discusses case folding, not the historical personal-path example cited by 129.

Missing from the flip checklist: explicit publication consent for personal email/hostnames and research notes, clear sidecar trust language in public-facing security documentation, and triage of the unexplained Ungated Edit. A scanner does not establish consent or detect every kind of private text. I would not rewrite history merely for presentation, but the human must settle privacy before publication; after the flip copies persist. Keep an operational allowlist working rather than relocating protected configuration as cosmetic cleanup. Metadata and future release style are reversible; disclosure of a secret, personal notes or an email is not.

**Closure:** retained scan findings with justified fixture exclusions, successful actual security jobs, green documented CI on a clean clone and a representative dependency PR, an inspected publication file set, and a tag pointing to the reviewed commit. Release notes should match the approved changelog text. GitHub visibility, stars, CI results, tag absence and the historical email counts in 129 remain reports, not fresh checks here.

## 9. Unopened tickets, ranked

**1 — Investigate the Ungated Edit immediately.** Establish whether the call executed, whether its policy allowed it, whether a hook ran, and whether event joining lost the evidence. Contract 7.7 distinguishes Ungated from other provider limits, so do not call this a bypass or harmless logging noise prematurely. Closure needs the original record examined through the supported reader, a minimized reproduction or bounded diagnosis, and a regression. The supplied run/call identifier and “1 of 28” are unverified reports here. Resolve or publicly bound this before relying on a comprehensive gating claim.

**2 — Remove the cockpit timing fragility.** `tests/cockpit.rs:3865–3873` really checks the five-to-seven-second interval. Test the timeout decision through controlled process state or a controllable clock, with only a generous integration watchdog; preserve the guarantee that approval is awaited. Closure is a behavior regression independent of routine scheduler slowness. Small, reversible work with direct CI value.

**3 — Add show --verdict.** Useful if it explains derived outcome alongside provider-reported completion and the record's limits. Do not imply that `completed` proves task correctness. Close with representative gated, mixed, failed, foreign and incomplete records, no launches needed. This strengthens inspectability with a reversible CLI addition.

**4 — JSON refusal fields/probes.** Open as deferred compatibility work with item 5's prerequisites; do not duplicate the decision in a competing ticket. Require a consumer before spending subscription quota.

**5 — OpenCode.** Defer implementation until the existing four-provider evidence and platform story are dependable. If desired, open a research ticket for authentication, owned configuration, headless protocol, hook fail-open behavior and isolation probes. The external feature/star claims in 129 are unverified here and do not establish routability. A fifth adapter is reversible code but creates a substantial continuing compatibility promise.

## Vote

Publish after the privacy/secret checks, security-workflow diagnosis, and a truthful required CI scope, with the unexplained Ungated Edit either resolved or explicitly bounded. Prefer additive records, named refusal for nested repositories, unauthenticated sidecar diagnostics, and separate enforcement populations over changes that manufacture stronger evidence than the system possesses. Defer full Linux runtime parity, extra vendor JSON fields and a fifth provider, because they remain reversible work while public disclosure and lost repository history do not.

## What I could not verify

I ran no Cargo command, test suite, live provider probe, secret scanner or GitHub query. I did not access other worktrees, runtime records or machine configuration; build 316's unapplied patch, the 952-record walk, CI run 38695261071, the exact 51-test tally, live Python/libm behavior, and the anomalous Edit remain unverified. Source inspection establishes code paths and assertions, not that tests pass or that the proposed fixes work. Some combined read outputs were truncated; this memo relies on the relevant visible excerpts and focused subsequent reads, not unseen output. External comparison claims are treated only as claims in research 129, not current facts about those projects.

## Pendiente de mí

Nothing further in this seat's authorized scope. The human retains all decisions; the session should reconcile these recommendations with the other seats and write accepted decisions into their originating tickets.

## Cambiado

Created only `.scratch/hydra/research/130-seat-gpt.md`. No source, configuration, ticket or other file was edited; no tests or Cargo commands were run.

## Encontrado

The proposed Linux library-only fallback still includes a reported failing numerical assertion; build 325 distinguishes two reap failures that the brief conflates; contract 7.7 already prefers stored scopes and already discloses sidecar tampering. Research 129 also has stale changelog and source-line claims, and SECURITY attributes Codex skill copies to build 87 rather than the readings' map 71.
