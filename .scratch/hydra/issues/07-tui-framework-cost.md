# 07 - Rust TUI against Textual, for a chat and diff-review interface

Type: research
Status: resolved

## Question

The TUI is deferred, but the framework choice is worth knowing now, because it decides whether Hydra stays a single-language project.

The memory argument is the weak one and should not decide this. A single-user local TUI's footprint is noise. The real question is what each framework makes you build by hand.

Compare ratatui against Textual on the work Hydra's eventual TUI actually needs:

- Streaming markdown with syntax highlighting, rendered as tokens arrive
- A side-by-side or unified diff viewer with per-hunk accept and reject
- A session list with search
- Panes that survive a provider subprocess dying

For each: shipped widget, community crate or package, or hand-built. Estimate the hand-built ones honestly.

Then answer the coupling question. If Textual wins, the TUI is a separate Python process reading `~/hydra/runs/` JSONL, and Hydra becomes two languages. Say whether that seam is clean or whether it leaks.

Name a winner and give the reason. Do not hedge.

## Answer

Textual wins, and the memory argument never mattered.

The decision comes down to the two most expensive requirements. Textual ships `MarkdownStream`, a buffering API built for exactly this, and can take `textual-diff-view` from Toad, Will McGugan's own ACP-based multi-agent terminal harness, which is close to a direct precedent for Hydra. Ratatui has `tui-markdown`, maintained but with no streaming API, so buffering is hand-rolled, and no diff-view crate at all. gitui's hand-built diff component runs 900 to 1000 lines, which makes 2 to 3 weeks the honest estimate for that gap alone.

Session search is a wash, 2 to 3 days either way. Subprocess survival is comparable, with Rust ahead only on defaults: tokio isolates task panics, while a Textual worker crashes the whole app unless you opt out at each call site.

The source document's `parllama` citation does not hold. Its own help file and source show no search or filter on the session list; `ctrl+f` filters a separate Ollama models tab.

On coupling, the seam is clean for history and leaky for live panes. Tailing JSONL and shelling out to `hydra run` matches D7 and D10 exactly. But "a pane survives its subprocess dying" implies live attach, and live attach needs a channel that static JSONL does not provide. Inventing one would be a third thing crossing between the two systems, which D5 forbids. That is a design constraint on the run record, not a blocker, and it belongs to ticket 09.

Detail: [research/07-tui-framework-cost.md](../research/07-tui-framework-cost.md)
