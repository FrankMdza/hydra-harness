# TUI framework cost: ratatui vs Textual

Ticket: [07 - Rust TUI against Textual](../issues/07-tui-framework-cost.md)
Status: answered

## The memory argument, rejected

Ratatui apps run in single-digit megabytes. A Textual app with Rich, asyncio, and a Python interpreter behind it settles somewhere in the 30-60MB range depending on what else it imports. Both numbers are noise on a developer workstation running one TUI for one user. Neither number should decide anything here, and no source below is cited to make that case, because there is no case to make. The real cost is what each framework leaves for a human to build by hand, in weeks, before the TUI does what Hydra needs.

## Method

Checked ratatui's own crate ecosystem (crates.io, the `ratatui` and `ratatui/tui-widgets` GitHub orgs, `awesome-ratatui`) and Textual's own docs and source against the four requirements in the ticket. Where a claim in the source document (`~/Downloads/Harness CLI Unificado para Cursor, Gemini y Claude.md`) could be checked against the tool it names, I checked it rather than repeated it. One of its claims does not hold up; noted below.

## Requirement 1: streaming markdown with syntax highlighting

**Textual: shipped.** Textual's `Markdown` widget renders Rich's `Syntax` for fenced code blocks, and as of the 2026 releases the framework ships `MarkdownStream`, a purpose-built object for exactly this case: markdown arrives as fragments, `MarkdownStream` buffers fragments that arrive faster than the widget can render and flushes them as the widget catches up. This is not a pattern you assemble, it is an object you instantiate. Syntax highlighting runs through Pygments via Textual's own `Content` class, described by Textual's maintainer as a rewrite done specifically to make streamed syntax highlighting fast enough to keep up with token arrival.

**ratatui: community crate, maintained, not built for streaming.** `tui-markdown` (crates.io, 25 releases since Feb 2024, latest `0.3.9` published within the last month) converts a complete markdown string to a ratatui `Text` in one call, using `pulldown-cmark` for parsing and `syntect` for code highlighting when the `highlight-code` feature is on. There is no incremental or buffered API: `from_str()` takes the whole string and returns the whole `Text`. To stream, you re-run `from_str()` on the growing buffer each time a chunk arrives and throttle how often you do that yourself, so the render loop does not choke on a fast token stream. `Oatmeal`, a maintained ratatui LLM chat client (syntect-based highlighting, 257 commits, latest release `0.13.0`), takes this exact approach: hand-rolled re-render-and-throttle around a highlighting crate, not a streaming API. A second crate, `ratatui-markdown` (tree-sitter based, Mermaid support, actively released through May 2026), is younger, has no streaming API either, and at 42 stars has far less real-world mileage than `tui-markdown`.

Estimate: Textual, close to zero, it is already built. Ratatui, 0.5-1 engineer-week to wire `tui-markdown` and hand-build the buffering and throttling that Textual gives you for free.

## Requirement 2: diff viewer, per-hunk accept and reject

**Textual: community widget covers the hard half.** `textual-diff-view` (batrachianai, 190 stars, 68 commits, actively maintained) renders unified and split diffs with syntax highlighting, and it was extracted from a real production app, not written as a demo. That app is **Toad**, Will McGugan's own terminal front end for agentic coding CLIs (Claude Code, Gemini CLI, OpenHands, and other ACP-speaking agents) built on Textual. Toad is close to a direct precedent for Hydra's eventual TUI: same problem shape, one terminal harness fronting several agent CLIs. `textual-diff-view` itself is read-only, it has no accept/reject state; that layer, hunk selection, keyboard navigation, applying or discarding a hunk, is still yours to build. One real constraint: the widget is AGPL-3.0 with a commercial license option, which matters less for a single-user local tool than it would for anything redistributed, but it is a real term, not a footnote to skip.

**ratatui: nothing shipped, nothing to extract.** Searched for a reusable diff-view crate and found none, only standalone apps (`diff-tui`, `ftdv`, `git-viewer`) that are not built as libraries. Diff computation itself is a solved problem, `similar` or the faster `imara-diff` (used by `gitoxide`), both mature. Rendering and hunk-level interaction are not. The closest real prior art is `gitui`, a flagship ratatui app: its diff component (`src/components/diff.rs`) is roughly 900-1,000 lines built directly on ratatui's `Line`/`Span`/`Paragraph`/`Block`, hand-tracking a `Selection` enum, a `selected_hunk` index, and explicit `stage_hunk()`/`unstage_hunk()`/`reset_hunk()` methods. That is what "hand-built diff viewer with per-hunk actions" costs in ratatui today: a component the size of gitui's.

Estimate: Textual, roughly 1 engineer-week to add hunk-selection and accept/reject state on top of `textual-diff-view`'s existing render and layout. Ratatui, 2-3 engineer-weeks to reach the same result from primitives, gitui being the evidence for that number.

## Requirement 3: session list with search

**Textual: pieces shipped, wiring is yours.** The framework ships `textual.fuzzy.Matcher`, the same fuzzy matcher the built-in Command Palette uses to score and highlight hits, plus `ListView`, `OptionList`, and `DataTable`. No single "searchable session list" widget exists, but the composition (`Matcher` plus a list widget) is proven inside Textual's own source, not just documented in the abstract.

**ratatui: same shape, different maintained pieces.** No shipped searchable-list widget either. Compose the built-in `List` widget with `nucleo` (the fuzzy matcher from Helix, roughly 6x faster than the older `fuzzy-matcher` crate, description states its "core matcher implementation is considered complete") and an input widget, `tui-input` or `ratatui-textarea` (both recommended in ratatui's own docs, both maintained, `ratatui-textarea` has built-in search support).

Estimate: both sides land around 2-3 engineer-days. Call it a wash.

**Correcting the source document here.** It cites `parllama` for "robust session management." Checked `parllama`'s own `help.md` and its `widgets/views` source tree: the session panel (`ctrl+s`) lists and loads sessions, but there is no search or filter over that list. The `ctrl+f` filter box in `parllama` filters a separate tab, the list of Ollama models pulled from ollama.com. The claim that `parllama` demonstrates session search does not hold. It is not evidence for either framework on this requirement.

## Requirement 4: panes survive a provider subprocess dying

This is not a widget question on either side, it is task-supervision architecture, and the two frameworks default in opposite directions.

**Rust/tokio:** a panic inside a spawned task is contained to that task by default. It returns as an `Err` through the task's `JoinHandle`; the runtime and every other task keep running. Building a pane that survives its subprocess dying still takes hand-written work, a state machine watching exit status and swapping the pane into an error view, maybe 1-2 days, but the framework's default is already the safe one.

**Textual:** the documented default behavior when a worker raises an unhandled exception is to exit the whole app and print the traceback. Surviving that requires passing `exit_on_error=False` to every worker that wraps a subprocess reader, then watching `Worker.StateChanged` for an `ERROR` state to recover the pane. Same order of hand-built work, 1-2 days, but the unsafe behavior is the default and the safe one is opt-in per call site. Forget the flag on one provider's subprocess reader and that provider crashing takes the whole TUI down with it.

Estimate: comparable effort both sides, edge to Rust on default safety rather than hours saved.

## The coupling question

If Textual wins, the TUI is a separate Python process. Whether that seam is clean depends on which half of the TUI you mean.

**For history: clean.** Session list, search, diff review of a finished run, streaming markdown for anything already recorded, all of this only ever reads `~/hydra/runs/*.jsonl`. Per D10, that JSONL is the source of truth and nothing needs to be running for the harness to work, so a Python reader tailing those files needs no RPC, no shared memory, no FFI. Per D7, launching a new run already goes through a CLI call (`hydra run ...`) returning JSON on stdout, a boundary that does not care what language calls it. A Python TUI shelling out to the Rust `hydra` binary is the same pattern Hydra already uses for the provider CLIs themselves.

**For live, interactive panes: it leaks.** A "pane that survives a subprocess dying" implies the TUI is attached to a *running* provider process, not just tailing its finished log, forwarding keystrokes, showing permission prompts, watching it die in real time. JSONL tailing alone does not carry that: a file on disk has no way to accept your keystroke and no way to push you a state change the instant it happens. Either the Rust harness keeps owning every provider subprocess and exposes a live attach channel (a socket, a continuing process, a second stream alongside the trajectory JSONL) that the Python TUI proxies input and output through, or the Python TUI spawns and supervises provider subprocesses itself, duplicating the process-teeing logic that already lives in Rust. The first option keeps Rust as the only thing that ever touches a provider subprocess, and keeps the TUI a pure display-and-dispatch layer. The second reopens D5, which says the two systems converge on exactly the run record and the skill store and nothing else. A live attach channel is a third convergence point. It is not a reason to reject Textual, but it is a real design task for whoever closes the run-record-schema and session-launch-lifecycle tickets, not something "just read the JSONL" resolves on its own.

**Packaging leaks too, separately from data.** Two languages means two toolchains, two dependency managers, two release pipelines, and no single static binary to hand someone. That cost is real and worth naming, but it is a one-time operational cost, not a recurring design tax the way the live-attach channel is.

## One thing outside the four requirements, worth a sentence

Textualize, the company, wound down in 2025. Textual continues as an open-source project maintained mainly by Will McGugan personally, with releases as recent as June 2026. Ratatui is maintained by an organization with multiple maintainers across several repos. That is a real difference in bus factor. It is offset by the fact that McGugan's active project right now is Toad, the same problem Hydra is trying to solve, which means the person keeping Textual alive is also the person stress-testing it against this exact use case in production.

## Verdict

**Textual wins.** Two of the four requirements, streaming markdown and the diff viewer, are the two most expensive ones to get right, and Textual either ships them or ships a maintained, production-tested extraction of them (`MarkdownStream`, `textual-diff-view` out of Toad). Ratatui's equivalents are a markdown-to-text crate with no streaming API and no diff widget at all, backed by gitui as the honest 2-3 week estimate for hand-building one. Session search is a wash. Subprocess survival favors Rust's default but costs about the same in hours either way. Add it up and Textual saves roughly 3-4 engineer-weeks against the two hardest requirements, against a coupling cost that is clean for the read side (most of what a TUI does) and requires deliberate channel design for live interaction, a design task Hydra would face in either language once panes need to be truly interactive rather than replayed.

D8 put the *router core* in Rust because a self-edit that does not compile must never reach review. That reasoning is about Ouroboros rung two editing Hydra's own routing logic, not about a presentation layer that never gets self-edited. Choosing Textual for the TUI does not reopen D8. It answers the one thing the map already flagged as open: "if Textual wins, it is a separate Python process reading the same JSONL." It is, and that process should stay exactly that, a reader and a CLI-caller, never a second owner of provider subprocesses, never a second implementation of routing logic.

## Sources

- Textual `MarkdownStream`: https://textual.textualize.io/widgets/markdown_viewer/ , https://willmcgugan.github.io/streaming-markdown/
- Textual `Markdown` widget and Rich `Syntax`: https://textual.textualize.io/widgets/markdown/
- Will McGugan on the Content-class syntax highlighting rewrite: https://x.com/willmcgugan/status/1945194498885104081
- `tui-markdown`: https://docs.rs/tui-markdown , https://crates.io/crates/tui-markdown , https://github.com/joshka/tui-markdown
- `ratatui-markdown`: https://github.com/celestia-island/ratatui-markdown , https://crates.io/crates/ratatui-markdown
- Oatmeal: https://github.com/dustinblackman/oatmeal
- `textual-diff-view`: https://github.com/batrachianai/textual-diff-view
- Toad: https://willmcgugan.github.io/announcing-toad/ , https://willmcgugan.github.io/toad-released/ , https://github.com/batrachianai/toad
- `similar`: https://github.com/mitsuhiko/similar
- `imara-diff`: https://github.com/pascalkuthe/imara-diff
- gitui diff component: https://github.com/gitui-org/gitui (`src/components/diff.rs`)
- Textual `fuzzy.Matcher` / command palette: https://textual.textualize.io/guide/command_palette/ , https://textual.textualize.io/api/fuzzy_matcher/
- `nucleo`: https://github.com/helix-editor/nucleo
- `tui-input` / `ratatui-textarea`: https://ratatui.rs/examples/apps/user_input/ , https://github.com/ratatui/ratatui-textarea
- Textual worker error handling: https://textual.textualize.io/guide/workers/
- Tokio task panic isolation: https://users.rust-lang.org/t/handling-unintended-panics-in-tokio-tasks-gracefully/34025
- parllama: https://github.com/paulrobello/parllama/blob/main/src/parllama/help.md , https://github.com/paulrobello/parllama/tree/main/src/parllama/widgets/views
- Future of Textualize: https://textual.textualize.io/blog/2025/05/07/the-future-of-textualize/
- Source document checked against primary sources: `~/Downloads/Harness CLI Unificado para Cursor, Gemini y Claude.md`
