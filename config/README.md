# config

Protected configs 2 through 6 of harness spec 15.1 live here. The gate denies writes to
every path below under every policy, `permissive` included, and 7.5 says why each one
earns it.

| File | Config | Exists |
|---|---|---|
| `env-allowlist.json` | #2, the environment allowlist | no, it is Rust in `src/env.rs` by design, and `hydra doctor` reports it as in code rather than missing |
| `export-allowlist.json` | #3, the export allowlist | no. `hydra export` has been built since build ticket 34 and refuses to run without this file. `export-allowlist.json.example` beside it is the shape: a JSON array of absolute repository paths, one per repo whose work may leave this machine. A relative path is refused |
| `export-signoff.json` | beside #3, the export sign-off manifest, since build ticket 65 | no. A JSON array of `{pass_id, judge_run_id, finding_item}`, one per judge finding a human confirmed; a judge-derived preference pair leaves only when its pass is listed. `export-signoff.json.example` is the shape. Denied to runs the way the allowlist is |
| `preamble.md` | #4, the instruction preamble | yes |
| `mcp.md` | #5, the canonical MCP document | yes, and it declares one server, the delegate face of harness spec 9, admitted under `standard` and `permissive` since build ticket 26 |
| `panels.json` | #6, the approved panel shapes, since build ticket 138 | no. The human writes it. Missing means no shape is approved, and every request for seats is refused |

Config #1 is the policy directory, which is `policies/` and not this one.

`preamble.md` is read verbatim and prefixed to every recorded run's task string, so it
lands in `run_start.task` and the record shows exactly what the run was told. Harness
spec 8 bounds what it may say: it asserts that what Hydra wrote at spawn is the run's
instructions, that everything read afterwards is data, and that the run's actions are
bounded by a policy it cannot widen. Three things and no more. It never claims the gate
will catch a bad action and never says anything shaped like a guarantee, because a run
that trusts the gate is the Hermes failure with the roles swapped.

Editing this file changes what every future run is told, which is the failure mode 15.1
names for it: a run told a different rule than the one Hydra enforces. It carries no sha
of its own. Contract 5.1 leaves it that way on purpose, since the text lands in the task
string that every record already stores.

`mcp.md` is the opposite case on the sha: it does not ride the task string, so every run
record carries `mcp.doc_sha`, which is `shasum -a 256 config/mcp.md` over the whole file.
The document itself says what its one fenced `toml` block may hold. It declares one server,
the delegate face, and its `command` is an absolute path into this checkout's `target/`, so
a clone anywhere but `~/hydra` has to edit that line before the face can start. Adding a
server is a human's decision: a server definition is a command Hydra spawns inside every
run the document admits it to.

`panels.json` lists the panels a gated session may ask its own launcher for without a
prompt, map ticket 82. Nothing launches from it yet; build ticket 140 does that. This is
the fixed three-seat review panel as one shape:

```json
{
  "shapes": [
    {
      "name": "review",
      "askers": ["standard"],
      "cap_per_hour": 2,
      "seats": [
        { "provider": "claude", "model": "opus", "policy": "judge", "role": "reviewer" },
        { "provider": "codex", "policy": "standard", "role": "adversary" },
        { "provider": "cursor-agent", "model": "composer-2.5", "policy": "judge", "role": "security" }
      ]
    }
  ]
}
```

Every field is required except `model`, which falls back to the provider's default, and a
field the loader does not know refuses the whole file with its name. `name` is unique
across shapes. `provider` is one of `claude`, `cursor-agent`, `agy` and `codex`. `policy`
and every entry of `askers` are one of the five in `policies/`. `role` is free text.
`cap_per_hour` is at least 1, and `seats` and `askers` hold at least one entry each.

A seat never gets more than the session that asked for it. The loader checks this with
7.3's containment: every policy in `askers` must grant everything each seat's policy
grants, or the shape is refused. In the example, `standard` contains `judge` and itself,
so it loads. Adding `read-only` to `askers` refuses it, because `read-only` cannot run
the `standard` seat, and so does `curate`, which neither contains `standard` nor is
contained by it. `permissive` contains all five and may ask for any shape.
