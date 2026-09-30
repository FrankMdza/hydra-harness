"""The prefix monitor, on synthetic records and one committed fixture.

A prefix is the first max(1, floor(fraction * lines)) lines. Fractions are 0.1, 0.25
and 0.5. The rest of the record must not move a feature.
"""
import contextlib
import importlib.util
import io
import json
import math
import os
from datetime import datetime, timedelta, timezone
from pathlib import Path
import tempfile
import unittest
from unittest import mock

MODULE = Path(__file__).resolve().parents[1] / "monitor.py"
spec = importlib.util.spec_from_file_location("monitor", MODULE)
monitor = importlib.util.module_from_spec(spec)
spec.loader.exec_module(monitor)

BASELINE = Path(__file__).resolve().parents[1] / "baseline.py"
_baseline_spec = importlib.util.spec_from_file_location("baseline_for_monitor_tests", BASELINE)
baseline = importlib.util.module_from_spec(_baseline_spec)
_baseline_spec.loader.exec_module(baseline)

FIXTURE = (
    Path(__file__).resolve().parents[3]
    / "tests/fixtures/records/claude@2.1.261/00000000-0000-7000-8000-000000000001.jsonl"
)
START = datetime(2026, 9, 1, tzinfo=timezone.utc)
TEST = START + timedelta(days=16)
T0 = "2026-09-01T00:00:00.000Z"


def run_id(at: datetime, n: int) -> str:
    millis = int(at.timestamp() * 1000)
    hexed = f"{millis:012x}" + "7000" + f"{n:016x}"
    return f"{hexed[:8]}-{hexed[8:12]}-{hexed[12:16]}-{hexed[16:20]}-{hexed[20:32]}"


def envelope(ident: str, seq: int, ts: str) -> dict:
    return {
        "run_id": ident,
        "provider": "claude",
        "model": "m",
        "parent_run_id": None,
        "secret_flag": None,
        "seq": seq,
        "ts": ts,
    }


def ts_at(seconds: int) -> str:
    return f"2026-09-01T00:00:{seconds:02d}.000Z"


def hook_deny(tool: str) -> dict:
    return {
        "type": "system",
        "subtype": "hook_response",
        "hook_event": "PreToolUse",
        "hook_name": f"PreToolUse:{tool}",
        "output": json.dumps({
            "hookSpecificOutput": {
                "hookEventName": "PreToolUse",
                "permissionDecision": "deny",
                "permissionDecisionReason": f"hydra/capability: no {tool}",
            }
        }),
    }


def write_jsonl(path: Path, lines: list[dict]) -> None:
    path.write_text("".join(json.dumps(line) + "\n" for line in lines))


def traced(ident: str, fail: bool) -> list[dict]:
    """Four lines. At 0.5 only the first two count.

    Read in full, a failure and a success carry the same refusals, the same first
    refused tool and the same token total. The prefix is the only place they differ.
    """
    lines = [dict(envelope(ident, 0, T0), hydra={
        "type": "run_start", "command": "run", "policy": "standard", "task": "do the thing",
    })]
    end = {
        "type": "run_end", "exit_code": 0, "refusal": None, "body_source": "stream",
    }
    if fail:
        lines.append(dict(envelope(ident, 1, T0), raw=hook_deny("Bash")))
        lines.append(dict(envelope(ident, 2, T0), raw=hook_deny("Write")))
        lines.append(dict(envelope(ident, 3, T0), hydra=end, raw={
            "type": "result", "usage": {"output_tokens": 99999},
        }))
    else:
        lines.append(dict(envelope(ident, 1, T0), raw={
            "type": "assistant",
            "message": {"role": "assistant", "content": [{"type": "text", "text": "ok"}]},
        }))
        lines.append(dict(envelope(ident, 2, T0), raw=hook_deny("Bash")))
        lines.append(dict(envelope(ident, 3, T0), hydra=end, raw=hook_deny("Write") | {
            "usage": {"output_tokens": 99999},
        }))
    return lines


def rec(seq: int, raw: dict, provider: str = "claude") -> dict:
    return dict(envelope("00000000-0000-7000-8000-0000000000e1", seq, T0), provider=provider, raw=raw)


def claude_use(n: int, name: str, **inp) -> dict:
    block = {"type": "tool_use", "id": f"u{n}", "name": name}
    if inp:
        block["input"] = inp
    return {"type": "assistant", "message": {"content": [block]}}


def cursor_start(n: int, path: str | None) -> dict:
    body = {"args": {"path": path}} if path is not None else {}
    return {
        "type": "tool_call", "subtype": "started", "call_id": f"k{n}",
        "tool_call": {"readToolCall": body},
    }


def codex_run(n: int, command: str | None) -> dict:
    item = {"type": "command_execution", "id": f"x{n}", "exit_code": 0, "aggregated_output": ""}
    if command is not None:
        item["command"] = command
    return {"type": "item.completed", "item": item}


def agy_step(n: int, params: dict | None) -> dict:
    info = {"parameters": params} if params is not None else {}
    return {"event": "step_update", "step_update": {
        "step_type": "tool", "step_index": n, "state": "ACTIVE",
        "tool_name": "run_command", "tool_info": info,
    }}


def repeats(raws: list[dict], provider: str = "claude") -> tuple[int, int]:
    lines = [rec(i, raw, provider) for i, raw in enumerate(raws)]
    out = monitor.summarize(lines, 0.5)
    return out["repeated_calls_so_far"], out["longest_repeat_run"]


class RepeatTests(unittest.TestCase):
    def test_claude_counts_a_call_seen_again_and_the_longest_streak(self):
        raws = [claude_use(1, "Read", path="a"), claude_use(2, "Read", path="b"),
                claude_use(3, "Read", path="a"), claude_use(4, "Read", path="a")]
        # Half of four lines would read two, so pad to twice the length.
        raws += [{"type": "result"}] * 4
        self.assertEqual(repeats(raws), (2, 2))

    def test_the_same_tool_with_other_input_is_not_a_repeat(self):
        raws = [claude_use(1, "Read", path="a"), claude_use(2, "Read", path="b")] + [{"type": "result"}] * 2
        self.assertEqual(repeats(raws), (0, 1))

    def test_key_order_of_the_input_does_not_matter(self):
        raws = [claude_use(1, "Grep", pattern="x", path="a"),
                claude_use(2, "Grep", path="a", pattern="x")] + [{"type": "result"}] * 2
        self.assertEqual(repeats(raws), (1, 2))

    def test_a_call_without_input_keys_on_the_name_alone(self):
        raws = [claude_use(1, "Bash"), claude_use(2, "Bash"), claude_use(3, "Read")] + [{"type": "result"}] * 3
        self.assertEqual(repeats(raws), (1, 2))

    def test_a_streak_breaks_on_a_different_call(self):
        raws = [claude_use(1, "Read", path="a"), claude_use(2, "Read", path="b"),
                claude_use(3, "Read", path="a"), claude_use(4, "Read", path="a"),
                claude_use(5, "Read", path="a")] + [{"type": "result"}] * 5
        self.assertEqual(repeats(raws), (3, 3))

    def test_cursor_reads_the_args_of_the_tool_call(self):
        raws = [cursor_start(1, "a"), cursor_start(2, "a"), cursor_start(3, "b"), cursor_start(4, None)]
        raws = raws + [{"type": "result"}] * 4
        self.assertEqual(repeats(raws, "cursor-agent"), (1, 2))

    def test_codex_reads_the_command(self):
        raws = [codex_run(1, "ls"), codex_run(2, "ls"), codex_run(3, "pwd"), codex_run(4, "ls")]
        raws = raws + [{"type": "result"}] * 4
        self.assertEqual(repeats(raws, "codex"), (2, 2))

    def test_agy_reads_the_tool_info_parameters(self):
        raws = [agy_step(1, {"CommandLine": "ls"}), agy_step(2, {"CommandLine": "ls"}),
                agy_step(3, {"CommandLine": "pwd"}), agy_step(4, None)]
        raws = raws + [{"type": "result"}] * 4
        self.assertEqual(repeats(raws, "agy"), (1, 2))

    def test_features_print_the_two_counts_but_train_leaves_them_out(self):
        # Measured on 2026-09-30: in the fitted vector they lowered the outcome AUC at every
        # fraction, so `features` prints them and `raw_features` does not carry them.
        self.assertNotIn("repeated_calls_so_far", monitor.SCALE_NAMES)
        self.assertNotIn("repeated_calls_so_far", monitor.raw_features({"_prefix": {"repeated_calls_so_far": 3}}))
        lines = [dict(rec(0, {"type": "system"}), raw=None, hydra={
            "type": "run_start", "command": "run", "policy": "standard", "task": "x",
        })]
        lines += [rec(i + 1, raw) for i, raw in enumerate(
            [claude_use(1, "Read", path="a"), claude_use(2, "Read", path="a")] + [{"type": "result"}] * 4
        )]
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "00000000-0000-7000-8000-0000000000e1.jsonl"
            write_jsonl(path, lines)
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                monitor.main(["features", str(path), "--at", "0.5"])
        feats = json.loads(out.getvalue())["features"]
        self.assertEqual(feats["repeated_calls_so_far"], 1)
        self.assertEqual(feats["longest_repeat_run"], 2)


class FeatureTests(unittest.TestCase):
    def test_a_prefix_does_not_see_the_rest_of_the_record(self):
        ident = run_id(START, 1)
        task = baseline.PREAMBLE + "\n\nprobe"
        lines = [
            dict(envelope(ident, 0, ts_at(0)), hydra={
                "type": "run_start", "command": "run", "policy": "read-only", "task": task,
            }),
            dict(envelope(ident, 1, ts_at(5)), hydra={
                "type": "seat_request", "request_id": "r", "shape": None, "seats": [],
                "task": "t", "decision": "launched", "reason": None, "runs": [],
            }),
            dict(envelope(ident, 2, ts_at(10)), raw={
                "type": "assistant",
                "message": {"role": "assistant", "content": [
                    {"type": "tool_use", "id": "toolu_01", "name": "Bash", "input": {}},
                    {"type": "tool_use", "id": "toolu_02", "name": "Write", "input": {}},
                ]},
            }),
            dict(envelope(ident, 3, ts_at(15)), raw=hook_deny("Write")),
            dict(envelope(ident, 4, ts_at(20)), raw={
                "type": "user",
                "message": {"role": "user", "content": [{
                    "type": "tool_result", "tool_use_id": "toolu_02", "is_error": True,
                    "content": "hydra/capability: policy read-only does not allow file_write (Write).",
                }]},
                "usage": {"output_tokens": 12},
            }),
            dict(envelope(ident, 5, ts_at(40)), raw=hook_deny("Bash")),
            dict(envelope(ident, 6, ts_at(50)), raw={
                "type": "result", "usage": {"output_tokens": 99999},
            }),
            dict(envelope(ident, 7, ts_at(50)), hydra={"type": "run_end"}),
            dict(envelope(ident, 8, ts_at(50)), raw={"type": "assistant", "message": {"content": []}}),
            dict(envelope(ident, 9, ts_at(50)), raw={"type": "result"}),
        ]
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / f"{ident}.jsonl"
            write_jsonl(path, lines)
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                monitor.main(["features", str(path), "--at", "0.5"])
            view = json.loads(out.getvalue())
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                monitor.main(["features", str(path), "--at", "0.1"])
            early = json.loads(out.getvalue())["features"]
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                monitor.main(["features", str(path), "--at", "0.25"])
            quarter = json.loads(out.getvalue())
        self.assertEqual(view["run_id"], ident)
        self.assertEqual(view["at"], 0.5)
        self.assertEqual(view["events"], 10)
        self.assertEqual(view["events_read"], 5)
        feats = view["features"]
        self.assertEqual(feats["events_so_far"], 5)
        self.assertEqual(feats["tool_calls_so_far"], 2)
        self.assertEqual(feats["distinct_tools"], 2)
        self.assertEqual(feats["refusals_so_far"], 1)
        self.assertEqual(feats["first_refused_tool"], "Write")
        self.assertEqual(feats["output_tokens_so_far"], 12)
        self.assertEqual(feats["seconds_elapsed"], 20)
        self.assertIs(feats["seat_requested"], True)
        self.assertEqual(feats["provider=claude"], 1.0)
        self.assertEqual(feats["model=m"], 1.0)
        self.assertEqual(feats["policy=read-only"], 1.0)
        self.assertEqual(feats["preamble"], 1.0)
        self.assertAlmostEqual(feats["log_task_chars"], math.log1p(len(task)))

        self.assertEqual(early["events_so_far"], 1)
        self.assertEqual(early["tool_calls_so_far"], 0)
        self.assertEqual(early["refusals_so_far"], 0)
        self.assertIsNone(early["first_refused_tool"])
        self.assertIsNone(early["output_tokens_so_far"])
        self.assertIs(early["seat_requested"], False)
        self.assertEqual(early["seconds_elapsed"], 0)

        self.assertEqual(quarter["events_read"], 2)
        self.assertIs(quarter["features"]["seat_requested"], True)
        self.assertEqual(quarter["features"]["tool_calls_so_far"], 0)
        self.assertEqual(quarter["features"]["seconds_elapsed"], 5)

    def test_the_claude_fixture_prefix_counts_the_denied_write(self):
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            monitor.main(["features", str(FIXTURE), "--at", "0.5"])
        view = json.loads(out.getvalue())
        self.assertEqual(view["events"], 8)
        self.assertEqual(view["events_read"], 4)
        feats = view["features"]
        self.assertEqual(feats["tool_calls_so_far"], 1)
        self.assertEqual(feats["distinct_tools"], 1)
        self.assertEqual(feats["refusals_so_far"], 1)
        self.assertEqual(feats["first_refused_tool"], "Write")
        self.assertIsNone(feats["output_tokens_so_far"])
        self.assertIs(feats["seat_requested"], False)
        self.assertEqual(feats["seconds_elapsed"], 0)
        self.assertEqual(feats["provider=claude"], 1.0)
        self.assertEqual(feats["policy=read-only"], 1.0)
        self.assertEqual(feats["model=None"], 1.0)

        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            monitor.main(["features", str(FIXTURE), "--at", "0.25"])
        quarter = json.loads(out.getvalue())["features"]
        # The init frame lists Bash, Read and Write. Those are not calls.
        self.assertEqual(quarter["events_so_far"], 2)
        self.assertEqual(quarter["tool_calls_so_far"], 0)
        self.assertEqual(quarter["refusals_so_far"], 0)

    def test_cursor_agy_and_codex_calls_use_their_own_frames(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)

            def six(ident: str, second: dict, third: dict | None = None) -> Path:
                lines = [dict(envelope(ident, 0, T0), provider="cursor-agent", hydra={
                    "type": "run_start", "command": "run", "policy": "standard", "task": "x",
                })]
                lines[0]["provider"] = second.get("provider", lines[0]["provider"])
                lines.append(dict(envelope(ident, 1, T0), provider=lines[0]["provider"], raw=second["raw"]))
                if third is not None:
                    lines.append(dict(envelope(ident, 2, T0), provider=lines[0]["provider"], raw=third))
                else:
                    lines.append(dict(envelope(ident, 2, T0), provider=lines[0]["provider"], raw={
                        "type": "assistant", "message": {"content": []},
                    }))
                while len(lines) < 6:
                    n = len(lines)
                    lines.append(dict(envelope(ident, n, T0), provider=lines[0]["provider"], raw={
                        "type": "result", "usage": {"output_tokens": 1},
                    }))
                path = root / f"{ident}.jsonl"
                write_jsonl(path, lines)
                return path

            cursor = six("00000000-0000-7000-8000-0000000000c1", {
                "provider": "cursor-agent",
                "raw": {
                    "type": "tool_call", "subtype": "started", "call_id": "c1",
                    "tool_call": {"readToolCall": {"args": {"path": "README"}}},
                },
            }, {
                "type": "tool_call", "subtype": "completed", "call_id": "c1",
                "tool_call": {"readToolCall": {"result": {"error": "hydra/capability: no"}}},
            })
            agy = six("00000000-0000-7000-8000-0000000000a1", {
                "provider": "agy",
                "raw": {
                    "event": "step_update",
                    "step_update": {
                        "step_type": "tool", "step_index": 1, "state": "ERROR",
                        "tool_name": "Edit",
                        "tool_info": {"error": {"message": "hydra/capability: no"}},
                    },
                },
            })
            codex = six("00000000-0000-7000-8000-0000000000d1", {
                "provider": "codex",
                "raw": {
                    "type": "item.completed",
                    "item": {
                        "type": "command_execution", "id": "c1", "command": "ls",
                        "exit_code": 0, "aggregated_output": "README\n",
                    },
                },
            })

            def feats(path: Path) -> dict:
                out = io.StringIO()
                with contextlib.redirect_stdout(out):
                    monitor.main(["features", str(path), "--at", "0.5"])
                return json.loads(out.getvalue())["features"]

            got = feats(cursor)
            self.assertEqual(got["tool_calls_so_far"], 1)
            self.assertEqual(got["distinct_tools"], 1)
            self.assertEqual(got["refusals_so_far"], 1)
            self.assertEqual(got["first_refused_tool"], "Read")
            self.assertIsNone(got["output_tokens_so_far"])

            got = feats(agy)
            self.assertEqual(got["tool_calls_so_far"], 1)
            self.assertEqual(got["refusals_so_far"], 1)
            self.assertEqual(got["first_refused_tool"], "Edit")

            got = feats(codex)
            self.assertEqual(got["tool_calls_so_far"], 1)
            self.assertEqual(got["distinct_tools"], 1)
            self.assertEqual(got["first_refused_tool"], None)
            self.assertEqual(got["refusals_so_far"], 0)

    def test_rejects_a_fraction_outside_the_three(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "r.jsonl"
            path.write_text("{}\n")
            with self.assertRaises(SystemExit), contextlib.redirect_stderr(io.StringIO()):
                monitor.main(["features", str(path), "--at", "0.3"])


def corpus(root: Path):
    records = root / "runs"
    records.mkdir()
    rows = []
    for d in range(24):
        for k in range(4):
            ident = run_id(START + timedelta(days=d, minutes=k), d * 10 + k)
            fail = (d + k) % 2 == 0
            rows.append({
                "run_id": ident,
                "provider": "same",
                "model": "m",
                "policy": "standard",
                "task_text": "do the thing",
                "outcome": "failed" if fail else "completed",
                "human_verdict": "fixed" if fail else "applied",
            })
            if d == 0 and k == 0:
                continue
            write_jsonl(records / f"{ident}.jsonl", traced(
                ident, fail,
            ))
            # traced() stamps provider claude. The dataset row says provider same.
            # Rewrite the envelope provider so a reader of the record agrees with the row
            # when score builds the row from the record. Train uses the dataset row.
            text = (records / f"{ident}.jsonl").read_text()
            (records / f"{ident}.jsonl").write_text(text.replace('"provider": "claude"', '"provider": "same"'))
    data = root / "dataset.jsonl"
    data.write_text("".join(json.dumps(row) + "\n" for row in rows))
    return data, records, rows


class TrainTests(unittest.TestCase):
    def test_learns_the_prefix_and_writes_the_report(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            data, records, rows = corpus(root)
            home = root / "home"
            out = io.StringIO()
            with mock.patch.dict(os.environ, {"HYDRA_GRADER_HOME": str(home)}):
                with contextlib.redirect_stdout(out):
                    monitor.main([
                        "train", str(data), "--at", "0.5", "--label", "outcome",
                        "--test-from", TEST.date().isoformat(), "--records", str(records),
                    ])
            report = json.loads(out.getvalue())
            model_path = home / "models" / "monitor" / "monitor-outcome-0.5.json"
            model = json.loads(model_path.read_text())
            self.assertEqual(model["label"], "outcome")
            self.assertEqual(model["at"], 0.5)
            self.assertEqual(model["rows"]["fit"] + model["rows"]["test"], len(rows) - 1)
            self.assertTrue(model["version"].startswith("monitor-outcome-0.5-"))
            self.assertIn("events_so_far", model["vocab"])
            self.assertIn("first_refused=Bash", model["vocab"])
            self.assertEqual(set(report), {
                "version", "label", "test_rows", "test_positive", "fit_base_rate",
                "constant_base_rate", "provider_model_base_rate", "model",
                "beats_provider_model_base", "reliability",
            })
            for block in ("model", "constant_base_rate", "provider_model_base_rate"):
                self.assertIn("rank_auc", report[block])
            self.assertEqual(report["model"]["rank_auc"], 1.0)
            self.assertEqual(report["version"], model["version"])
            self.assertTrue(report["beats_provider_model_base"]["brier"])
            self.assertTrue(report["beats_provider_model_base"]["log_loss"])

            again = io.StringIO()
            with mock.patch.dict(os.environ, {"HYDRA_GRADER_HOME": str(home)}):
                with contextlib.redirect_stdout(again):
                    monitor.main([
                        "train", str(data), "--at", "0.5", "--label", "outcome",
                        "--test-from", TEST.date().isoformat(), "--records", str(records),
                    ])
            self.assertEqual(json.loads(again.getvalue())["version"], report["version"])

            fail_id = run_id(START + timedelta(days=40), 1)
            good_id = run_id(START + timedelta(days=40), 2)
            fail_path = records / f"{fail_id}.jsonl"
            good_path = records / f"{good_id}.jsonl"
            write_jsonl(fail_path, traced(fail_id, True))
            write_jsonl(good_path, traced(good_id, False))
            for path in (fail_path, good_path):
                path.write_text(path.read_text().replace('"provider": "claude"', '"provider": "same"'))

            def score(path: Path) -> dict:
                buf = io.StringIO()
                with contextlib.redirect_stdout(buf):
                    monitor.main(["score", str(model_path), str(path)])
                return json.loads(buf.getvalue())

            failed = score(fail_path)
            succeeded = score(good_path)
            self.assertGreater(failed["p"], 0.8)
            self.assertLess(succeeded["p"], 0.2)
            self.assertEqual(failed["events_read"], 2)
            self.assertEqual(succeeded["events_read"], 2)
            self.assertEqual(failed["at"], 0.5)
            self.assertEqual(failed["model_version"], model["version"])
            self.assertEqual(failed["label"], "outcome")
            self.assertEqual(failed["run_id"], fail_id)

            verdict_out = io.StringIO()
            with mock.patch.dict(os.environ, {"HYDRA_GRADER_HOME": str(home)}):
                with contextlib.redirect_stdout(verdict_out):
                    monitor.main([
                        "train", str(data), "--at", "0.25", "--label", "verdict",
                        "--test-from", TEST.date().isoformat(), "--records", str(records),
                    ])
            verdict = json.loads((home / "models" / "monitor" / "monitor-verdict-0.25.json").read_text())
            self.assertEqual(verdict["label"], "verdict")
            self.assertEqual(verdict["at"], 0.25)
            # 0.25 of four lines is one line, the run_start, so the prefix no longer
            # separates and this only checks that the verdict model is its own file.
            self.assertEqual(json.loads(verdict_out.getvalue())["label"], "verdict")

    def test_refuses_a_split_without_enough_of_the_rarer_class(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            data, records, _rows = corpus(root)
            # Drop every failure's record so the joined set is all completions.
            for path in records.glob("*.jsonl"):
                text = path.read_text()
                if "PreToolUse:Bash" in text.splitlines()[1]:
                    path.unlink()
            with self.assertRaises(SystemExit) as caught:
                monitor.main([
                    "train", str(data), "--at", "0.5", "--label", "outcome",
                    "--test-from", TEST.date().isoformat(), "--records", str(records),
                ])
            self.assertIn("rarer class", str(caught.exception))

    def test_task_label_trains_and_drops_gated_rows(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            data, records, rows = corpus(root)
            for row in rows[:6]:
                row["outcome"] = "gated"
            data.write_text("".join(json.dumps(row) + "\n" for row in rows))
            out = root / "m.json"
            monitor.main([
                "train", str(data), str(out), "--at", "0.5", "--label", "task",
                "--test-from", TEST.date().isoformat(), "--records", str(records),
            ])
            model = json.loads(out.read_text())
            self.assertEqual(model["label"], "task")
            self.assertEqual(model["positive"], "failed for the task")


if __name__ == "__main__":
    unittest.main()
