# 85 - A cockpit for the orchestrating session

Research for ticket 85, 2026-09-29. Herdr on this machine is `herdr 0.8.2` (`herdr --version`).
Nothing here was run against a live pane: no pane was opened, no provider launched. Where a claim
is about Herdr's behaviour and I only have documentation, I say so.

## Short answer

Most of what the human described already exists in pieces. `puppets.py launch` opens one Herdr
pane per agent, the bridge labels it, and the mascot wakes one head per live pane. What is missing
is the table beside the chat, kept fresh. Herdr can hold that as a pane running a loop over
`puppets.py board`. It cannot draw a table or react to a click inside a pane it does not own, so
"click a cell and it expands" and "a head grows per agent" beyond three need either Herdr's own
pane zoom (probably enough) or a small TUI of our own, later.

## 1. What Hydra already shows

| Piece | Renders | Source | Refresh |
|---|---|---|---|
| `integrations/puppets/puppets.py board <goal>` (`cmd_board`, line 1306) | goal text, plans, chosen plan, a task table (`id class grade agent status run` plus verdict), and a `waiting on you:` line | the goal log, folded by `fold(args.goal)` | once per call, no loop. It needs a goal id: the argparse parser takes `goal` as a positional (line 1966). |
| `integrations/herdr/bin/hydra-bridge` | sets pane tokens and a pane title | `hydra status` per pane that carries a `run_id` token, `hydra show --events`, and `~/hydra/approvals/` | header comment says every two seconds (`integrations/herdr/README.md`, "What is here") |
| `integrations/herdr/bin/hydra-mascot --watch` (`watch_loop`, line 509) | the three-headed dragon and a status line `live=N +extra` | Herdr's pane list (`list_herdr_panes()`), tokens only. It never calls `hydra status` and never reads the runs directory. | pane list polled once a second (`last_poll`, line 519) |
| `integrations/herdr/bin/hydra-graph` | a graph of one run's events in the terminal | a run JSON, from a file or stdin, or `--follow <run-id>` | `--follow` sleeps two seconds (lines 149 and 154) |
| `integrations/herdr/hy.zsh` | opens two panes right of the current one, mascot on top, gated `claude` below (lines 203 to 209) | shell function, no polling | once, at launch |
| `docs/status.html` | not found. `find . -iname "status*.html"` outside `target/` returned nothing in this worktree, so I say nothing about what it shows. | | |

Two findings that matter for the design.

**The mascot already knows how many seats are alive.** `compute_watch_frame` (line 391) takes the
pane list, keeps the panes where `is_live_pane` is true (`run_id` token present, `state == live`,
`alive == yes`, lines 374 to 376), wakes up to three heads, and reports the rest as `extra`. So
"a head per agent" works today for three agents, and the fourth and later show as `+N` in the text
line. The mascot has a fixed three-headed art, so a fourth head means new art (MASCOTS.md, "Adding
a sprite", says rows must be the same width and a palette and three `mouth` lines are needed).

**Puppets already opens one pane per agent.** `attach_pane` (`puppets.py` lines 645 to 671), when
`HERDR_ENV` and `HERDR_PANE_ID` are set, runs `herdr pane split <pane> --direction down --no-focus`,
`herdr pane report-metadata` with tokens `run_id`, `provider`, `policy`, and `herdr pane run` of
`hydra logs <run-id> --follow`. Its docstring says the pane is opened after the launch because
`hydra-in-pane` would not return until the run ended. So the panes the human calls "a kanban, one
per agent" exist as a vertical stack of log panes. There is no column-by-status layout.

## 2. What Herdr can do

### Commands, quoted

`herdr --help` (0.8.2) lists these groups:

```
  herdr workspace <subcommand>     Workspace helpers over the socket API
  herdr worktree <subcommand>      Git worktree helpers over the socket API
  herdr tab <subcommand>           Tab helpers over the socket API
  herdr notification <subcommand>  Notification helpers over the socket API
  herdr agent <subcommand>         Agent/terminal helpers over the socket API
  herdr pane <subcommand>          Pane control helpers over the socket API
```

`herdr pane --help`:

```
  list, current, get, layout, process-info, neighbor, edges, focus, resize,
  zoom                  Toggle or set pane zoom
  read                  Read pane terminal output
  rename                Rename a pane
  input                 Set pane input routing
  split                 Split a pane
  swap, move, close
  send-text             Send literal text to a pane
  send-keys             Send key presses to a pane
  wait-output           Wait for matching pane output
  run                   Run a command in a pane
  report-agent          Report pane agent lifecycle state
  report-metadata       Report display-only pane metadata
```

`herdr agent --help` adds `prompt  Submit a prompt to an agent`, `wait`, `attach  Attach directly
to an agent terminal`, `start` and `explain`. `herdr notification --help` has one command,
`show  Show a notification`. `herdr plugin --help` (not listed in the top-level help, but it
answers) has `link`, `list`, `action`, `log`, and `pane  Manage plugin-owned panes`. `herdr plugin
pane --help` has `open`, `focus`, `close`, and `open` takes:

```
      --placement <PLACEMENT>
          [possible values: overlay, split, tab, zoomed]
      --direction <DIRECTION>
          [possible values: right, down]
      --env <KEY=VALUE>
```

A limit of the measurement: `herdr <group> <command> --help` prints the top-level help again, not
per-command help. The exact flags of `pane split`, `pane run` and `report-metadata` therefore come
from two other measured sources: `herdr api schema --json` (protocol 20, `schema_version: 1`), and
the calls Hydra's own code already makes (`hy.zsh` lines 203 to 207 uses `pane split --direction
--ratio --cwd --env` and `pane run`; `puppets.py` line 660 uses `report-metadata --source --token`).

The bundled schema (`herdr api schema --json`, 255 KB) lists these methods, among 130 or so:
`pane.report_metadata`, `pane.read`, `pane.zoom`, `pane.split`, `pane.send_keys`,
`pane.wait_for_output`, `pane.graphics.set`, `agent.view.set`, `notification.show`,
`plugin.pane.open`, `popup.close`, `events.subscribe`, `layout.apply`, `layout.export`.

`PaneReportMetadataParams` in that schema allows: `title`, `tokens` (at most 16, names matching
`^[A-Za-z0-9_-]{1,32}$`, string or null values), `state_labels` (a string map), `display_agent`,
`source`, `seq`, and `ttl_ms` (at most 86,400,000). So a pane can carry sixteen short strings
and a title. That is a label, not a table.

`PluginPanePlacement` is `["overlay", "popup", "split", "tab", "zoomed"]` in the schema.
`PopupSize` takes cells or a percentage such as `80%`. Note the CLI help lists four placements and
the schema five: `popup` exists in the socket API and, by the CLI text alone, not in the flag.

`PaneGraphicsSetParams` takes `data_base64`, `format` (`png`, `rgb`, `rgba`, `bgra`), a size, and a
placement in grid cells. So a process can push an image into a pane it controls. I did not test it.

### The plugin manifest

`integrations/herdr/herdr-plugin.toml` declares id `hydra.bridge`, `min_herdr_version = "0.8.0"`,
two `[[panes]]` (`bridge` and `mascot`, both `placement = "split"`, each a `command`) and two
`[[actions]]` (`run` and `interactive`, `contexts = ["workspace"]`). `herdr plugin list` on this
machine prints `hydra.bridge (Hydra bridge) enabled [local:...]`, and `herdr plugin action list`
returns both actions as JSON.

Herdr's plugin documentation (v0.9.3, the version `https://herdr.dev/llms.txt` points at; the
installed binary is 0.8.2, so a newer feature may not be here yet) says:

> "there is no runtime action registration and native non-terminal plugin UI."

and lists `[[actions]]`, `[[events]]`, `[[panes]]`, `[[startup]]`, `[[build]]` and
`[[link_handlers]]`, with pane placements `overlay`, `split`, `tab`, `zoomed` and `popup`
("session-modal window with optional width/height"). Keybindings can route to a plugin action with
`type = "plugin_action"`. The only mouse hook documented is link handlers, which "intercept
modified clicks (Control key) on URLs." Source:
https://raw.githubusercontent.com/herdrdev/herdr/v0.9.3/docs/next/website/src/content/docs/plugins.mdx

### The human's wants against Herdr

| Want | Herdr as it is | Verdict |
|---|---|---|
| The chat where the session runs | a normal pane. `hy` already does it. | covered |
| The table of tasks, agent, model, done | no table widget. A pane can run any process, so `puppets.py board` in a loop fills it. `agent.view.set` (a filter and sort over Herdr's own agent list) is unrelated to Hydra's goal log. | needs a process drawing in a pane |
| One pane per agent, kanban style | `attach_pane` opens them stacked downward. `layout.apply` and `pane.split --direction` can arrange columns by status; nothing in Hydra does it. | covered for panes, not for columns |
| Click a pane to expand it | `pane zoom` exists (help: `Toggle or set pane zoom`). Whether Herdr's own UI zooms on a mouse click I could not confirm from help or the plugin doc. It surely does by key. | probably covered, unconfirmed for the mouse |
| Put hands in, ask for review | `pane send-text`, `send-keys` and `agent prompt` write into a pane. The human can type in a focused pane. | covered |
| Change policies under approval | a `popup` pane can run any interactive process, so a modal is possible. See section 3 for what it may write. | possible, constrained |
| Mascot with a head per agent | mascot exists, three heads, `+N` beyond. `pane.graphics.set` could push images. | covered up to three |
| Click a table cell | no plugin UI primitive, no cell events | cannot do in Herdr; only a process of ours reading its own mouse events |

## 3. The three constraints

**D10, nothing has to be running.** `.scratch/hydra/map.md` line 212: "Nothing needs to be running
for the harness to work." `docs/rungs-spec.md` lists `no_daemon` as a critical reviewer item: "A
scheduler, background service, timer, or anything that must be running for the harness to work
(D10)." A cockpit is a viewer. If it is a pane the human opens, nothing in the harness depends on
it, so it passes. It fails only if a launch, an approval or the record ever waits for the cockpit.
That rules out one design: a cockpit that owns the approval channel.

**The gated session cannot read `runs/`.** The cockpit runs in a pane Herdr owns, outside the
seatbelt, like the bridge (`hydra-bridge` calls `hydra status`) and the mascot (tokens only,
so it never touches the runs directory at all). The safest cockpit reads the same two ways:
tokens from Herdr, and CLI output from `hydra` and `puppets.py`. It must not be started from
inside a gated session.

**Protected config.** CLAUDE.md, "Nothing here is enforced": the six protected configs of
`docs/harness-spec.md` section 15.1 "are protected against Hydra's own runs", and a change to
one is made by a human. `policies/` is protected config 1 and `config/panels.json` is another.
`src/approve.rs` is "the store and the human-typed `hydra approve`" (CLAUDE.md, Layout). So
"under human approval" can mean three things that keep the rule:

1. The pane shows the current policy and a proposed diff, read only, and prints the command the
   human types in a plain shell (`$EDITOR policies/standard.json`, then commit). The pane never
   writes. Cheapest, and it fits the existing rule.
2. A `popup` pane that runs `hydra approve` itself, for the requests that command already owns
   (seat requests no panel shape covers). The human types in it. The cockpit only opens it. No new
   write path.
3. A popup that edits `policies/` directly. The write is by a human, in a process the human
   started, so the rule "only the human writes" holds in letter. But it then needs the same
   treatment as `hydra approve`: a typed confirmation, a sha shown before and after, and a test
   that the process refuses to run when stdin is not a tty. That is new code in a protected area,
   and by CLAUDE.md's own rule a change to protected config is something to "say so and let a
   human make". I recommend against it in the first two steps.

## 4. Ratatui against Textual

All quotes are from the pages named. A claim I could not source is marked.

| Need | Ratatui (Rust) | Textual (Python) |
|---|---|---|
| Mouse events | Ratatui does not read input. Its docs: "ratatui does not directly expose any event catching; the programmer will depend on the chosen backend's library." (https://ratatui.rs/concepts/event-handling/). crossterm provides `EnableMouseCapture`, "Mouse events can be captured with read/poll" (https://docs.rs/crossterm/latest/crossterm/event/struct.EnableMouseCapture.html). | Built in. "Textual processes mouse events like `Click` and `MouseDown` similarly to keyboard events" through its message queue; handlers are `on_click` or `@on(Click)` (https://textual.textualize.io/guide/events/). |
| Clickable table cells | `Table` with `TableState` "to allow the user to scroll through the rows and select one of them". The Table page does not say anything about mouse (https://docs.rs/ratatui/latest/ratatui/widgets/struct.Table.html). To click a cell you compute the cell from the mouse column and row against the layout `Rect` yourself. That inference is mine, not a documented feature. | `DataTable` posts `CellHighlighted` and `CellSelected`, the second "when a user presses Enter on a highlighted cell" (https://textual.textualize.io/widgets/data_table/). Its source defines `_on_mouse_move` and `_on_leave`; the part I could read showed no click handler, and the file was truncated, so I cannot say whether a click selects a cell. Test before relying on it. |
| Modals | none built in. You draw a `Clear` block and route input by hand. (Not sourced beyond the docs.rs index, which lists widgets and no modal; treat as likely.) | `ModalScreen[T]`, `push_screen(Screen(), callback)`, `dismiss(value)`. Docs: it "Prevents app keybindings from being processed while modal is active" (https://textual.textualize.io/guide/screens/). |
| Several streaming panes | You own the loop. Read each child's output in threads or async tasks, feed a channel, redraw. Immediate mode: "for each frame, your app must render all widgets" (https://docs.rs/ratatui/latest/ratatui/). A terminal-in-a-pane widget is not in the crate; the index shows no PTY widget, and a separate crate would be needed (I did not check tui-term). | Workers: `run_worker()` or `@work`, async ones "directly update widgets", thread workers use `call_from_thread()` (https://textual.textualize.io/guide/workers/). The page has no subprocess example. |
| Cost for one person | Compile-checked and a single binary, and Hydra's core is Rust. But you build the mouse hit-testing, the modal and the scroll views. | Faster to a first screen, and the repo already runs Python under `integrations/` with a venv (recent commit `66397ef`). Adds a dependency to a directory that today has only numpy for the mascot. |
| A head per agent | Draw N sprites in a `Layout` row. Art is the work. The renderer would be written from `hydra-mascot`'s half-block code, which is Python, so it is a port. | Same art problem, but the current renderer is Python and can be imported or re-used. |

Neither library changes the art cost of a fourth head. The art is the cost, not the toolkit.

Stated plainly: Textual makes clicks and modals cheap and pays in a Python dependency; Ratatui
makes the binary self-contained and pays in hit-testing you write yourself. For one person and a
viewer that is not part of the harness, Textual is cheaper. The `integrations/` directory is
already Python and "nothing under `src/` imports it", so a Textual cockpit would not touch the
Rust or the D10 line.

## 5. The smallest step

**Step 1: one pane, one loop, about one session.**

- A launcher next to `hy` that splits a pane and runs `puppets.py board <goal>` in a loop, for
  example `while :; do clear; python3 integrations/puppets/puppets.py board "$GOAL"; sleep 2; done`
  in the pane via `herdr pane run`. Add `--watch` to `cmd_board` only if the shell loop flickers.
- The goal id: the board needs one. The launcher takes it as an argument, or reads the newest
  goal. I did not measure how goals are listed, so that is a small open question.
- The mascot needs nothing new. `attach_pane` already tags each agent pane with `run_id`, so the
  dragon wakes a head per live pane, up to three, with `+N` after that.
- Result: chat in one pane, the task table (agent, model in the agent column, status, and the
  `waiting on you:` line) in the next, and one log pane per agent below. The human zooms a pane
  with Herdr's own zoom.

What it reuses: `cmd_board`, `attach_pane`, `hydra-mascot --watch`, `hy`'s split calls. What it
does not do: click a cell, a definition-of-done column (the board prints no such column; whether
the task records carry one I did not check), columns by status, a fourth head, any policy editing.

**Step 2, if step 1 gets used: a read-only Textual board.** A `DataTable` of the same folded
state, `Enter` on a row runs `herdr pane zoom` or `herdr pane focus` on that run's pane by
token, and a `ModalScreen` that shows a proposed policy diff and prints the command to type.
About two to three sessions. Still no writes. Open the approval popup (`hydra approve`) from a
key. The head-per-agent art is a separate ticket.

**Not before then:** a policy editor that writes, and a cockpit that owns approvals.

## Draft answer

## Answer

*Draft for the human, 2026-09-29. Nothing below is decided until you say so.*

Build step 1 only: a pane that loops `puppets.py board <goal>` beside the chat, with the panes and
mascot Hydra already opens. Herdr, as measured, cannot draw a table or react to a cell click; it
runs processes, splits and zooms panes, and reports up to sixteen tokens and a title per pane.
That is enough for the table and the per-agent panes without building a TUI. It is not enough for
clicking a cell, and a fourth head needs new art whichever toolkit draws it.

Policy changes stay out of the cockpit's write path. The pane shows the policy and prints the
command you type; a `popup` may open `hydra approve` for requests that command already owns.

**What would change this:** if step 1 is used and the lack of clicks hurts, step 2 is a read-only
Textual board (Textual over Ratatui because the mouse and modals are built in and `integrations/`
is already Python). If a first look shows Herdr zoom does not respond to a mouse click and the
keys are not enough, that also moves step 2 up. If you want the policy editor to write, that is a
map ticket, because it changes what protected config means.

**Not confirmed:** Herdr zoom on a mouse click; whether the definition of done is stored on a
task; how a goal id is found; Textual `DataTable` mouse selection; `popup` in the CLI on 0.8.2.
