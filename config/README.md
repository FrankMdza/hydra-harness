# config

Protected configs 2 through 5 of harness spec 15.1 live here. The gate denies writes to
every path below under every policy, `permissive` included, and 7.5 says why each one
earns it.

| File | Config | Exists |
|---|---|---|
| `env-allowlist.json` | #2, the environment allowlist | no, it is still Rust in `src/env.rs` |
| `export-allowlist.json` | #3, the export allowlist | no. `hydra export` has been built since build ticket 34 and refuses to run without this file. `export-allowlist.json.example` beside it is the shape: a JSON array of absolute repository paths, one per repo whose work may leave this machine. A relative path is refused |
| `export-signoff.json` | beside #3, the export sign-off manifest, since build ticket 65 | no. A JSON array of `{pass_id, judge_run_id, finding_item}`, one per judge finding a human confirmed; a judge-derived preference pair leaves only when its pass is listed. `export-signoff.json.example` is the shape. Denied to runs the way the allowlist is |
| `preamble.md` | #4, the instruction preamble | yes |
| `mcp.md` | #5, the canonical MCP document | yes, and it declares one server, the delegate face of harness spec 9, admitted under `standard` and `permissive` since build ticket 26 |

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
The document itself says what its one fenced `toml` block may hold. It ships with no
server declared, because nobody has configured one on this machine, and adding one is a
human's decision: a server definition is a command Hydra spawns inside every run the
document admits it to.
