# 48 - The delegate face needs `--parent <run-id>` and the MCP document cannot say so

Type: grilling
Status: resolved
Blocked by:

## Question

Build ticket 11 built `hydra delegate-face [--parent <run-id>]`, the stdio MCP server of harness
spec 9, and the way a run gets it is a `[[server]]` block in `config/mcp.md` with `command =
"hydra"` and `args = ["delegate-face"]`. The document is read once per launch and its servers are
copied into the run's config as written, so a face spawned that way carries no `--parent`, and
every child it delegates is depth 0. The tree is flat. A delegated child's own delegations are
bounded by no policy above them and counted against no cap, which is the drain section 9 put the
cap there to stop.

Hydra knows the run id at the moment it writes the config. What it lacks is a rule saying which
server wants it, and the document format of protected config #5 has no field to say it in.

## The three shapes

**A substitution token in `args`**, `{run_id}` or the like, expanded by `mcp::write_config` for any
server that carries it. Costs: a new rule in a protected document's format, and a token a human
has to know to write.

**A field on the server**, `per_run = true`, meaning Hydra appends `--parent <run-id>` to `args`.
Costs: the field means one thing for one server and nothing for any other, and an unknown field is
refused today on purpose.

**Hydra recognizes its own binary.** A server whose `command` is `hydra` gets `--parent <run-id>`
appended with no field at all. Costs: a rule that keys on a string, and a human who writes an
absolute path to the binary walks past it unless the match is on the basename.

Each is a document change and a code change together, which is why ticket 11 left it.

## Where the evidence is

Build ticket 11's `## Outcome` and `IMPLEMENTATION.md`'s section "Delegation, and the face",
the paragraph beginning "Nothing injects". `src/mcp.rs`, `write_config`. `config/mcp.md`, the
field table. Harness spec 6.5 and 9.

## Answer

Resolved 2026-09-07 with the human. **Hydra recognizes its own binary.** A server in the
canonical document whose `command` has the basename `hydra` gets `--parent <run-id>` appended
to its args at launch, and nothing else in the document changes: no token, no field, no new
rule a human has to know to write a server. The match is on the basename so an absolute path
to the binary enters too, and the appended flag lands in the run's own `mcp.json`, which the
record already names. The cost named above stands: it is a rule keyed on a string, and it is
written down in `config/mcp.md` beside the field table so the string is not a secret.

The human also answered the question this ticket did not ask, which policies admit the face:
`standard` and `permissive`, not `read-only`, `judge` or `curate`. The orchestrating session
writes the `[[server]]` block and the human reviews it in the commit, which is the protected
config rule met with the human present. Build ticket 26 carries the code and the block.
