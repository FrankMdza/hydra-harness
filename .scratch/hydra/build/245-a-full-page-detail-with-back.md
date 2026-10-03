# 245 - Enter opens a run as a full page, and Left goes back

Type: build
Status: resolved
Blocked by: 240

## What

The human, 2026-10-03, after the usability probes: Enter on a row of a table should open the
detail as a full page over the whole cockpit, Left should go back to where he was, as a browser's
back does, and the detail should scroll from the keyboard. Today:

- `Enter` puts the run in the small `detail` panel (`App::open_selected`, `src/cockpit/app.rs:980`)
  and `Esc` empties it.
- `Left` and `Right` move focus between panels (`step_focus`), the same as `Shift-Tab` and `Tab`.
- The detail scrolls one line per `Up` or `Down` only once `Tab` has put the focus on it, and in
  the raw view (`r`) those keys move between calls instead (`move_selection`, `Focus::Detail`).
  The mouse wheel scrolls three lines. There is no `PageUp`, `PageDown`, `Home` or `End` there.

## Do

1. `Enter` on a row of the task table, a lane or the lanes by agent opens a full page: the
   detail of that run drawn over the whole right side (over the whole screen without
   `--session`), with a title line naming the run, provider, model, policy and state, and a
   footer naming the keys.
2. A history stack. `Left` or `Backspace` on a page goes back one step: to the previous page if
   one was opened from it, else to the cockpit with the same panel focused and the same row
   selected. `Esc` goes all the way back. Off a page, `Left` keeps its meaning today.
3. On a page: `Up`/`Down` scroll one line, `PageUp`/`PageDown` or `Space`/`b` one screen,
   `Home`/`g` and `End`/`G` to the ends. The raw view (`r`) and `i` work there as they do in the
   small panel; in the raw view `Up`/`Down` still move between calls and `PageUp`/`PageDown`
   scroll.
4. In the small `detail` panel, `PageUp`/`PageDown`, `Home` and `End` scroll as well.
5. The help view of build 240 lists the new keys; one table feeds both, as 240 sets up.
6. `README.md` (the cockpit's key table) and `IMPLEMENTATION.md`: one row and one sentence.

## Done when

- Tests on a synthetic app: `Enter` on a lane row opens the page with that run; `Left` returns
  with the same focus and row; two pages deep, `Left` twice returns to the cockpit; `PageDown`
  then `Home` leaves the scroll at zero; a page longer than the screen reads its last line back
  after `End`.
- fmt, clippy `--workspace --all-targets -- -D warnings` and `cargo test` pass.

## Outcome

Resolved 2026-10-03 by run `01a101a3-5835` (`cursor-agent` composer-2.5, `standard`, 7.62M
tokens), applied by the session with one change to the tests.

Built: `Enter` (or a click) on a run opens it as a full page over the main area, with a title
line (run, provider, model, policy, state) and a key footer. A back stack: `Left` or `Backspace`
goes back one page or to the cockpit with the same focus and row, `Esc` all the way. On the page
and in the small detail pane: `Up`/`Down` a line, `PageUp`/`PageDown` or `Space`/`b` a screen,
`Home`/`g` and `End`/`G` the ends; in the raw view `Up`/`Down` still move between calls. Seat
requests and Puppets steps still open in the small pane. The new keys are in the `HELP` table.
Four tests in `tests/cockpit.rs`, the ones the ticket names.

Changed by the session: the run rewrote the shared `steps_app` helper to open its step by key
instead of by click, added a seat request to its snapshot, and moved one assertion from 1 to 2 to
match. The original helper passes with the change applied, so the rewrite only removed the
click's coverage; the session put it back. The run reported cockpit tests failing under
parallel threads in its worktree; on master the suite passed four times in a row in parallel,
119 passed and 1 ignored. `--lib` 234, fmt and clippy clean.

Not checked by hand: the page in a real terminal. The human asked for this view on 2026-10-03.
