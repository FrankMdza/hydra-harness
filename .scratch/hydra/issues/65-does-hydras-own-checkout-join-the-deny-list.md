# 65 - Does the checkout the binary was built from join the deny list?

Type: grilling
Status: resolved
Blocked by:

## Question

Harness spec 15.2's deny list is Hydra's runtime directories and five config files. `config/mcp.md`
is protected config #5 because a server definition is a command Hydra spawns inside every run, and
the one server it declares is `/Users/USER/hydra/target/release/hydra`. The document is denied;
the binary it names is not, and neither is `src/`, which is where the next binary comes from, nor
`src/policy/gate.py`, the template every gate is compiled from. A `standard` run can rewrite the
command the protected document points at, and the next run's provider spawns it. `policy_sha`
does not move, because it hashes `policies/*.json`. The hiring-panel review of 2026-09-22 found it
and `SECURITY.md` states it.

1. **Does `hydra_home()` join `denied_write_paths` whole**, with the run's own worktree and, on a
   `curate` pass, its checkout carved back out? On this machine the checkout is the data root, so
   the deny is `~/hydra` and everything a run legitimately writes is already an own path. On a
   machine where the two differ, it is the checkout as well as the root. The gate's substring
   floor leaves a denied parent that holds an own path to the token pass, so nothing about the
   run's own paths changes.
2. **Or only `target/` and `src/`**, the two the review named, which is narrower and leaves
   `Cargo.toml`, `build.rs` and `config/mcp.md`'s siblings for the next review to find.
3. **What does the spec say?** 15.2's code rows are "refused at admission, not denied at run
   time", on the reasoning that a rung-two patch is what changes source. A run under `standard`
   is not a rung-two patch. Whether the "denied at run time" column gains a row for the checkout
   is the decision, and it changes 7.5's list.

Until this closes, `SECURITY.md` says the list covers the runtime directories and five files and
that the binary the MCP document names is writable by a run.

## Answer

Resolved 2026-09-22 with the human. **The whole checkout joins the deny list.** `hydra_home()` is
denied to writes under every policy, with the run's own worktree and a `curate` pass's own
checkout carved back out the way they already are, so on this machine the deny is `~/hydra` and
on a machine where the checkout and the root differ it is both. Harness spec 15.2's "denied at run
time" column gains the checkout as a row, and 7.5's list says so. The narrower "src/ and target/"
was declined because it leaves `Cargo.toml`, `build.rs` and `config/mcp.md`'s siblings for the
next reviewer, and "document only" because the binary the protected MCP document names is the
thing the protection is for. Build ticket 74 carries the code and the test under `sandbox-exec`.
