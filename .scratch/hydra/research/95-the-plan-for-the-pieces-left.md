# 95 - The ports that are left, a plan

Written 2026-10-01 by a read-only run (claude opus 5.5, `01a0f776`, 1.4M tokens) for map ticket
95. Its tickets carry letters (H1 to H8 for `herdr`, A1 to A5 for `apprentice`, O1 and O2 for
`otel`, L1 and L2 for `local-llm`, S1 and S2 for scripts); the build tickets written from it
carry numbers and name the letter.

## Not confirmed

- I ran no Python, no cargo and no test. Every "likely to differ" comes from reading.
- Build 220's diff is not in this tree. I assumed, from its "Built and held" text, that `records.py`, `redact.py` and `models.py` sit in `integrations/apprentice/` once it lands.
- I read these only by outline or header: `hydrapet.py` lines 273 to 516 and 660 to 885, `hydra-mascot` lines 28 to 100, most test bodies, `docs/logo/*.py`, `verify35.py` past line 25, and `gate.py` past its docstring.
- Whether the committed frames are what today's renderer writes. Nobody ran `--frames` here.
- Herdr's output shapes come from how the scripts parse them. I ran no `herdr`.
- Whether the apprentice launchd job is installed, and whether `~/.zshrc` sources `hy.zsh`.
- The one-hour sizing is a guess. Grader tickets cost 5M to 22M tokens each.
- I did not check the last build ticket number, so tickets carry letters.

## The plan

Fixtures are recorded once under CPython 3.14.7 from an empty environment, as builds 196 and 212 did. Each ticket breaks the Rust on purpose and counts the failures. One crate and one binary per piece: `hydra-herdr`, `hydra-apprentice`, `hydra-otel`, `hydra-local-llm`. Tickets marked "if" exist only under the answers I recommend in section 5.

**H1. herdr in Rust: the crate, and the graph draws one run**
- Ports: `bin/hydra-graph` whole.
- Depends on: Q3.
- Compares: stdout for recorded `hydra show --events` JSON of the 12 fixture records and the `test_graph.py` cases, and `render` called twice with a `seen` set.
- Normalize: nothing.
- Likely to differ: `cut_args` (`hydra-graph:18-25`). A non-dict argument prints as Python's `str()`, and `first_line` uses `splitlines()`, which also splits on `\x0b`, `\x1c` and `\u2028`.

**H2. herdr in Rust: the bridge's table and tokens**
- Ports: every pure function of `bin/hydra-bridge`, lines 50 to 404.
- Depends on: H1.
- Compares: function by function as build 212 did, on the `test_mapping.py` and `test_interactive_denials.py` cases. State and label, the tokens as JSON, the title, denial counts and offsets across two polls.
- Normalize: the scratch directory. `now` is an argument.
- Likely to differ: `round()` in `elapsed_seconds` (`:115`) rounds half to even, after a `fromisoformat` parse. Reuse the grader's `isotime.rs`.

**H3. herdr in Rust: the bridge polls and reports**
- Ports: `main`, `get_hydra_status`, `get_hydra_events`, `list_panes`, `report_pane` (`:407-551`).
- Depends on: H2.
- Compares: the argv log of a stub `herdr` and a stub `hydra` over one poll cycle, and the stderr lines.
- Normalize: the `elapsed` token, and the Python exception text after `Error reading hydra status for <id>:`.
- Likely to differ: the Python loop never exits, so the recorder has to kill it after a counted number of stub calls. A second cycle changes what the "report only on change" key lets through.

**H4. herdr in Rust: the board refreshes a pane**
- Ports: `bin/hydra-board` whole.
- Depends on: H1.
- Compares: `render_once` text against a stub helper, and the refusals of `puppets_helper` with exit code and stderr.
- Normalize: the scratch directory.
- Likely to differ: `newest_goal_id` (`:55-68`) keeps the first of two equal mtimes in `glob` order, which is the directory's order.

**H5. herdr in Rust: the mascot draws the pet and watches the panes**
- Ports: `compute_watch_frame`, the pane helpers, `draw`, `pet_cells`, `write_frames`, `watch_loop`, the skin file, and `render_compact_dragon` from `hydrapet.py:517-652`.
- Depends on: H1, Q1.
- Compares: `--frames` output against the 11 committed `.ans` files and `index.json`, byte for byte. `pet_cells` at recorded values of `t`. The `test_mascot.py` watch cases.
- Normalize: nothing.
- Likely to differ: `int(math.sin(t * 2.0 + idx * 1.8) * 1.2)` and the `> 0.2` breathing threshold. One bit of `sin` moves a pixel. `hydra-pyjson` already has matched math.

**H6, if Q1 keeps the HD skin. herdr in Rust: the HD dragon**
- Ports: `render_hd_dragon`, the three drawing functions, `hd_cells`.
- Depends on: H5.
- Compares: recorded canvases, with a stated tolerance.
- Likely to differ: numpy's float32 pairwise sums in `mean` and `sum`. Byte equality is unlikely, so map 90's fallback applies.

**H7, if Q2 ports the shell. herdr in Rust: the launchers and `hy`**
- Ports: `hydra-in-pane`, `hydra-tui-in-pane`, and the logic of `hy.zsh`, which stays as a one-line function.
- Depends on: H3.
- Compares: the stub `herdr` and `hydra` argv logs, the stderr refusals and the exit codes, over the `test_hy.py` cases.
- Normalize: the pane ids the stub invents.
- Likely to differ: `hydra-tui-in-pane:40` tees stderr through a process substitution while stdin and stdout stay on the tty. A Rust wrapper must leave both untouched.

**H8. herdr in Rust: the cutover**
- Changes: `herdr-plugin.toml`, the README, `MASCOTS.md`, `smoke.sh`'s five `python3` snippets, the comment at `src/cockpit/mascot.rs:3`.
- Deletes: the four Python `bin/` scripts, `hydrapet.py`, the tests, `integrations/requirements.txt`.
- Depends on: H1 to H7.
- Gate: the session runs `smoke.sh` against a live Herdr. A run cannot, and it costs one haiku call.

**A1. apprentice in Rust: the crate and the bank**
- Ports: `apprentice.py` lines 61 to 319.
- Depends on: build 220 on master, Q3.
- Compares: `bank.jsonl` bytes and stdout for `bank build` over the 12 fixtures plus synthesized records with seat requests, refusals and an interactive start.
- Normalize: the runs path in the stdout line.
- Likely to differ: `_dump` (`:163`) writes `sort_keys` with `ensure_ascii=False` into the prompt, and `write_jsonl` then escapes it again. `cut` counts code points.

**A2. apprentice in Rust: attempt asks the local model, and score**
- Ports: lines 325 to 499.
- Depends on: A1.
- Compares: `attempts.jsonl`, the body the stub Ollama received, stdout, stderr and exit code, with the stub from `record_judge.py`.
- Normalize: `seconds` and the port.
- Likely to differ: `grade` (`:383-392`). `\d` and `\s` match Unicode digits and spaces, and `int()` accepts them.

**A3. apprentice in Rust: pairs**
- Ports: lines 505 to 543.
- Depends on: A1.
- Compares: `pairs.jsonl` and stdout over the `test_apprentice.py` graded files, run twice.
- Normalize: `graded_at`.
- Likely to differ: the skip lines print `{...!r}`, Python's repr of a string or `None` (`:518`).

**A4, if Q4 ports it. apprentice in Rust: the BitNet shim serves one request at a time**
- Ports: `bitnet_serve.py` whole. Needs a hand-written HTTP server over `TcpListener`.
- Compares: status and body for the `test_bitnet_serve.py` requests against a stub binary, and the stub's argv.
- Normalize: `id`, `created`, and the `Server` and `Date` headers.
- Likely to differ: `--temp` gets `str(float(temperature))`, so `0` becomes `0.0` (`:65`, `:122`).

**A5. apprentice in Rust: the cutover**
- Changes: the plist runs `hydra-apprentice attempt --minutes 30`, its test becomes Rust, README.
- Deletes: `apprentice.py`, `records.py`, `models.py`, the tests. `redact.py` moves to `integrations/otel/`, because `record_to_otlp.py:15-19` still loads it.
- Gate: the session runs `bank build` both ways over the real runs.

**O1. otel in Rust: a record becomes a trace**
- Ports: `record_to_otlp.py` whole.
- Depends on: Q3.
- Compares: stdout, `--out` file bytes, and the batch directory with its size stamps, over the 12 fixtures and a three-level session. A stub `hydra` replays recorded `show --events` answers.
- Normalize: nothing.
- Likely to differ: `normalized` rewrites the record with `json.dumps(row)` before `hydra show` reads it (`:69-71`). `attributes` prints any non-int value through `str()` (`:46`).

**O2. otel in Rust: the cutover.** Deletes the script, its test and `redact.py`. Gate: `--all` both ways over the real runs.

**L1. local-llm in Rust: the MCP server answers `ask`**
- Ports: `hydra-local-llm` whole.
- Compares: reply lines for recorded request lines, and the body the stub Ollama received.
- Normalize: the port, and the OS text after `ollama is unreachable:`.
- Likely to differ: the `-32700` reply carries Python's `JSONDecodeError` message (`:151`). `isinstance(max_tokens, int)` lets `true` through (`:140`).

**L2. local-llm in Rust: the cutover.** The README's `mcp.md` block names the binary. Deletes the script and test.

**S1, if Q5 says so. scripts in Rust: the frontier**
- Ports: `.scratch/hydra/frontier.py`.
- Compares: stdout over the committed `issues/` and `build/` directories.
- Likely to differ: `lstrip("# ")` strips a set of characters, and `\w+` is Unicode (`:24-29`).

**S2. scripts: the rest are deleted or kept.** `verify35.py`, `docs/logo/*.py`, `docs/demo/verdict.py`, by Q5.

## The five answers

**1. herdr**

- Commands run against `herdr`:
  - `pane list`, JSON read at `result.panes[]` with `pane_id` and `tokens` (`hydra-bridge:438-443`, `hydra-mascot:430-435`).
  - `pane report-agent` and `pane report-metadata` with `--state-label`, `--title`, `--token` (`hydra-bridge:452-483`, `hydra-in-pane:26-31`, `hydra-tui-in-pane:29-34`).
  - `pane split`, JSON read at `result.pane.pane_id`, and `pane run` (`hy.zsh:203-211`).
  - Smoke only: `status server` printing `status: running`, `pane get`, `pane close`, `plugin pane open` and `close` (`smoke.sh:27`, `:73`, `:16`, `:53`, `:20`).
- Draws to a terminal: `draw` in half blocks with truecolor (`hydra-mascot:196-225`), `watch_loop` with cbreak and cursor codes (`:509-565`), the board's clear code (`hydra-board:10`).
- numpy does two different jobs:
  - The compact pet uses it as a 24 by 24 array of bytes with scalar writes (`hydrapet.py:517-519`). The math is `math.sin`. Rust needs no numpy for it.
  - The HD dragon is a 160 by 160 float32 canvas with anti-aliased circles, capsules and triangles (`hydrapet.py:149-241`, `:273-275`), box-averaged in `hd_cells` (`hydra-mascot:455-478`).
- The frames are generated once and committed. `write_frames` (`hydra-mascot:665-678`) writes 11 poses from the compact pet, and `src/cockpit/mascot.rs:3-10` and `:88` read them. So the cockpit needs no renderer.
- The watch pane does need one. It draws per frame from wall time and defaults to the HD skin (`hydra-mascot:441`, `:543-547`).
- Without a live Herdr a test can compare the pure functions, the frames, and the argv a stub `herdr` logs, as `test_hy.py:17-36` already does.

**2. apprentice, otel, local-llm**

- `apprentice` reads `$HYDRA_ROOT/runs/*.jsonl` (`apprentice.py:84`, `:314`) and the preamble (`:145`). It writes `bank.jsonl`, `attempts.jsonl` and `pairs.jsonl` under the grader home (`:66`, `:316`, `:439`, `:541`). It POSTs to `HYDRA_LOCAL_LLM_URL`, default `127.0.0.1:11434` (`:39`, `:346-350`).
- `bitnet_serve.py` listens on `127.0.0.1` (`:144`) and runs `llama-cli` (`:63-74`). It sends nothing out.
- `otel` reads one record and its siblings (`record_to_otlp.py:53`, `:87-90`), runs `hydra show` on a scratch copy (`:65-75`), and writes files (`:157-172`). No network (`README.md:57`).
- `local-llm` reads stdin, writes stdout (`hydra-local-llm:199-207`), and POSTs to the same URL (`:93-100`).
- No TLS or HTTP dependency is needed. `judge_http::post_json` (`judge_http.rs:281`) covers `generate` in the apprentice and `call_ollama` in local-llm.
- Two gaps in that reuse. `Failure::Status` carries no body (`judge_http.rs:33`) and local-llm prints the error body (`hydra-local-llm:103-109`). The client refuses any host but loopback (`judge_http.rs:3-7`), where `urllib` goes anywhere.
- The BitNet shim is a server, so the client helps it not at all.

**3. Shell**

- `hy.zsh` is a function sourced into the shell (`hy.zsh:2`). It sets no shell state, so its logic can move to a binary. It calls `python3` at `:59`, `:83`, `:97` and `:204-208`.
- `hydra-in-pane` and `hydra-tui-in-pane` are bash and call `python3` for one JSON field (`:23-24`, `:27-28`). Porting makes sense only to drop that.
- `smoke.sh` should stay shell. It needs a live Herdr.
- `docs/demo/demo.sh`, `record.sh` and `docs/logo/round3/show-all.sh` drive asciinema and print art. They should stay shell.

**4. What stays Python**

Training on a rented GPU with PyTorch (`90-the-integrations-in-rust.md:41-42`). No file is that. `apprentice/README.md:8` says nothing trains yet, and `bitnet_serve.py` is inference. The recorders under `crates/*/tests/record*.py` stay Python because they record the Python.

**5. Questions**

- **Q1. The mascot.** Options:
  - Port the compact pet only. Drop the HD skin, the standalone `hydrapet.py` and the map 75 modes `--image`, `--small`, `--sprite`, `--png`, `--demo`. I would take this, and H6 goes.
  - Port both.
  - Keep `hydrapet.py` as a frame generator, as `mascot.rs:3` says today. The pane then shows committed frames and herdr keeps one Python file.
- **Q2. The shell.** Port `hy` and the two launchers into `hydra-herdr`, or keep them and replace only the `python3` snippets, or leave them alone. I would port. That is H7.
- **Q3. Shared code.** `records`, `redact`, `isotime` and `judge_http` live in `crates/grader`. Options: depend on `hydra-grader` as a library, or move them to a shared crate first, which adds a ticket in front of H1. And does the loopback-only rule apply to the apprentice and local-llm?
- **Q4. `bitnet_serve.py`.** Port, drop, or keep. Its binary does not work (`apprentice/README.md:123`). I would drop it.
- **Q5. Scripts.**
  - `src/policy/gate.py` is 1,320 lines of Python compiled into every gate (`src/policy/mod.rs:1181`), Python on purpose (`gate.py:22-25`). In or out? In is a map ticket.
  - `frontier.py` starts every session. Port, and into which binary?
  - `verify35.py`, `docs/logo/*.py`, `verdict.py`, `tests/fixtures/mcp-probe-server.py`: delete, keep, or port.
