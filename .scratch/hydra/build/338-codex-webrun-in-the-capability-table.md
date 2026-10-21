# 338 - `codex` 0.160.0's web tool `webrun` is in the capability table, so a web lookup under `permissive` is allowed and recorded instead of refused as unknown

Type: build
Status: resolved
Size: haiku
Blocked by:
Opened: 2026-10-15, by the session, from run `01a14290-c0a1`: a `codex` research seat under
`permissive` asked for a web lookup and the gate answered `hydra/unknown-tool: webrun is not in
Hydra's capability table for this provider`. The table (`src/policy/table.rs`, `CODEX`) was read
off 0.154.0 and knows `WebSearch` as an unhooked `network_read` row; the installed binary is
0.160.0 and names the tool `webrun`. The refusal is the design working, 7.1: an unknown name fails
closed under every policy. The table is what is stale.

## What

One row, measured, not guessed. A free probe first: launch `codex exec` the way
`src/provider/codex.rs` does for its login probe, with the hook pointed at a script that logs the
payload and allows, ask it to fetch one URL, and read the payload: the tool's name, whether the
hook fired at all (if it did not, the row is `unhooked`, as `WebSearch` is), and the field that
carries the URL. Then the row in `CODEX`, with the version and the run id in its comment, and the
`is_unhooked` and `capability` facts pinned in `table.rs`'s tests beside the `WebSearch` ones.
IMPLEMENTATION.md's `codex` paragraph gains the sentence; the drift table of build 317 gains
0.160.0 if it keys on version.

## Contract

Input: a hook payload for provider `codex` with `tool_name` and `tool_input`. Output: the gate's
decision and reason. Latency: the gate's own, unchanged.

## Cases

Written before the row, in `tests/gate.rs` beside the `codex` cases, each a synthetic payload fed
to the compiled gate; the first two go red on master today:

1. `webrun` with a `https://` URL under `permissive` is allowed with `hydra/allow`.
2. `webrun` under `read-only` and `judge` is refused and the reason starts `hydra/no-network` or
   whatever `network_read` refusals read today, not `hydra/unknown-tool`.
3. `webrun` naming `file:///Users/USER/hydra/runs/x.jsonl` is refused the way `WebFetch` on
   `claude` is (A11 in `tests/corpus/README.md`).
4. A name the table does not hold, `webrun2`, is still refused `hydra/unknown-tool` under
   `permissive`.

## Amended by the panel, 2026-10-15

The codex seat (`133-panel-sol.md`): `codex login status` is the free probe; a `codex exec` turn
that fetches a URL costs quota, `tests/drift.rs` near line 429 says so for the hook probe. So the
probe is one paid turn, named as such in the Outcome with its token count, and it needs a positive
control: the same turn with a shell call, so the log shows the hook fired for `Bash` in the same
session before `webrun` is called `unhooked` on the strength of a missing callback. Two more
cases: `webrun` with `file:///Users/USER/hydra/ru%6es/x.jsonl`, the percent-encoded spelling,
which the raw pass lets through today for `WebFetch` (`tests/gate.rs` near 2419) and should be
pinned as it is, allowed or refused, with the reason; and the Slack floor at `gate.py` near 1361,
which maps `WebFetch` and `WebSearch` fields to the pattern and has no `webrun` row, so a
`webrun` of a Slack URL must be refused by the floor once the field name is known.

The opus seat: the gate refused `webrun` in run `01a14290-c0a1`, so the hook fired on it, so the
row is hooked, unlike `WebSearch`; the paid turn confirms the field name, not the hooking. And
`hydra doctor` has printed `codex codex-cli 0.160.0 measured 0.154.0` since the binary updated;
the drift check exists, nobody read it, and the Outcome says so.

## Do

1. The probe, its payload quoted in the Outcome.
2. The cases, red.
3. The row, the cases green, `no_codex_tool_appears_twice` still green.
4. IMPLEMENTATION.md and, if it exists for `codex`, the drift fixture.

## Done means

`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo nextest run --release --workspace`, `cargo test --release --workspace --doc` green, and the
Outcome quotes the probe's payload.

## Outcome

Built. `webrun` is a hooked `network_read` row in `CODEX`, the gate reads its URLs for the
Slack floor, and the cases are pinned. Two of the ticket's assumptions were false, and the
probe found both.

**The probe.** One paid `codex exec` turn on 2026-10-21, `codex-cli` 0.160.0, run from a
scratch directory under `/tmp` with the hook command in the shape `config_overrides` builds.
The turn asked for `echo probe-338-shell`, then a fetch of `https://example.com`. Usage was
43,186 input tokens, 39,808 of them cached, 129 output and 0 reasoning. The hook logged two
payloads. The web one:

```
{"session_id":"01a1609f-d4ad-71e2-ad3f-4c3ca07ad126","turn_id":"01a1609f-d8a7-75b3-ae0f-b97bd61a5d6e","transcript_path":"/Users/USER/.codex/sessions/2026/10/21/rollout-2026-10-21T17-40-50-01a1609f-d4ad-71e2-ad3f-4c3ca07ad126.jsonl","cwd":"/tmp/hydra-338-probe.WVYaiQ","hook_event_name":"PreToolUse","model":"gpt-6-luna","permission_mode":"bypassPermissions","tool_name":"webrun","tool_input":{"open":[{"ref_id":"https://example.com"}],"response_length":"short"},"tool_use_id":"exec-2cc2aa80-74a6-40c4-8ee4-4a44974c5951"}
```

The shell payload in the same session carried `tool_name: "Bash"` and
`tool_input: {"command": "echo probe-338-shell"}`, so the hook fired for `Bash` first. Nested
`codex` then ran that shell call under its own seatbelt and got `sandbox_apply: Operation not
permitted`. The hook still saw it. The stream carried the page as an `item.completed` of type
`web_search` with an `open_page` action and the title `Example Domain`.

**What the probe changed.** The ticket assumed `webrun` would be unhooked, as `WebSearch` is,
and that its URL would sit in a flat field. Neither held. The hook fires, and the URL sits in
`open[].ref_id`, a list. The row is `tool("webrun", Capability::NetworkRead)`, and the Slack
floor reads `open[].ref_id` through the new `fetch_urls` in `gate.py`.

**What was built.** `tests/gate.rs` gained `webrun_is_in_the_codex_table_and_its_urls_are_read_from_open`,
written red first and red on `hydra/unknown-tool`. It covers the four cases, the percent-encoded
spelling and the Slack floor. `codex_names_mcp_tools_with_claudes_prefix` in `src/policy/table.rs`
pins `webrun` as hooked and `NetworkRead`, beside `WebSearch`. `file:///` refusals for
`webrun` come from the raw protected-path pass, which already walks nested arguments, so
that pass needed no change. The percent-encoded case is allowed with `hydra/allow`, as
`WebFetch` is.

**Checks.** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo nextest run --release --workspace` with 1464 passed and 4 skipped, and
`cargo test --release --workspace --doc` all pass. `no_codex_tool_appears_twice` stays green.
The ticket's fast-loop crate, `hydra-apprentice`, does not hold the gate or the table, so the
targeted runs used `-p hydra`.

**Drift.** `hydra doctor` reports `codex codex-cli 0.160.0 measured 0.154.0`. I did not run
`hydra doctor` in this session. The ticket reports the message, and I took that as given.
`MEASURED_VERSIONS` still reads 0.154.0. Moving it to 0.160.0 would claim the rest of the
`codex` drift set was re-measured, and it was not. The drift fixture is recorded 0.154.0 runs,
and no new record exists to add, so it is unchanged.

**Found.** The replay does not follow `webrun` yet. `src/normalize/codex.rs` names every
`web_search` item `WebSearch`, and that reader does not handle an `open_page` item. Its
doc comment still says hosted search never reaches the hook on 0.154.0. This ticket left the
reader alone. A `webrun` call will likely show a replay mismatch until it is fixed. The probe
payload names the model `gpt-6-luna`, while `IMPLEMENTATION.md` says the account's model is
`gpt-6-astra`. My own shell refused one command during the probe setup. It was a substitution
in a command naming `git`, and I rewrote the call without the substitution.
