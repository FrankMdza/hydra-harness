import contextlib
import csv
import importlib.util
import io
import json
import os
from datetime import datetime, timedelta, timezone
from pathlib import Path
import tempfile
import unittest
from unittest import mock

MODULE = Path(__file__).resolve().parents[1] / "baseline.py"
spec = importlib.util.spec_from_file_location("baseline", MODULE)
baseline = importlib.util.module_from_spec(spec)
spec.loader.exec_module(baseline)

START = datetime(2026, 9, 1, tzinfo=timezone.utc)


def run_id(at: datetime, n: int) -> str:
    millis = int(at.timestamp() * 1000)
    hexed = f"{millis:012x}" + "7000" + f"{n:016x}"
    return f"{hexed[:8]}-{hexed[8:12]}-{hexed[12:16]}-{hexed[16:20]}-{hexed[20:32]}"


def rows(days: int = 30, per_day: int = 12) -> list[dict]:
    """Every run on provider `bad` is rate limited and every other one completes."""
    out = []
    for d in range(days):
        for k in range(per_day):
            provider = "bad" if k % 3 == 0 else "good"
            out.append({
                "run_id": run_id(START + timedelta(days=d, minutes=k), d * 100 + k),
                "provider": provider,
                "model": "m",
                "policy": "standard",
                "task_text": "do the thing" * (k + 1),
                "outcome": "rate_limited" if provider == "bad" else "completed",
                "human_verdict": None,
            })
    return out


TEST = START + timedelta(days=22)


class MeasureTests(unittest.TestCase):
    def test_launch_time_comes_from_the_uuidv7(self):
        at = datetime(2026, 9, 24, 22, 50, 6, 465000, tzinfo=timezone.utc)
        self.assertEqual(baseline.launched_at(run_id(at, 1)), at)

    def test_scores_on_fixed_numbers(self):
        self.assertAlmostEqual(baseline.brier([0.8, 0.2], [1, 0]), 0.04)
        self.assertAlmostEqual(baseline.log_loss([0.5, 0.5], [1, 0]), 0.6931471805599453)
        self.assertEqual(baseline.rank_auc([0.9, 0.1, 0.5], [1, 0, 0]), 1.0)
        self.assertEqual(baseline.rank_auc([0.5, 0.5], [1, 0]), 0.5)
        self.assertIsNone(baseline.rank_auc([0.5], [1]))

    def test_reliability_puts_one_in_the_last_bin(self):
        table = baseline.reliability([0.05, 1.0], [0, 1])
        self.assertEqual([b["n"] for b in table], [1, 0, 0, 0, 1])

    def test_targets(self):
        self.assertEqual(baseline.target({"outcome": "gated"}, "outcome"), 1)
        self.assertEqual(baseline.target({"outcome": "completed"}, "outcome"), 0)
        self.assertIsNone(baseline.target({"outcome": None}, "outcome"))
        self.assertEqual(baseline.target({"human_verdict": "fixed"}, "verdict"), 1)
        self.assertEqual(baseline.target({"human_verdict": "applied"}, "verdict"), 0)
        self.assertIsNone(baseline.target({"human_verdict": None}, "verdict"))


class TrainTests(unittest.TestCase):
    def test_split_is_by_launch_time(self):
        fit, test = baseline.split(rows(), TEST)
        self.assertTrue(max(baseline.launched_at(r["run_id"]) for r in fit) < TEST)
        self.assertTrue(min(baseline.launched_at(r["run_id"]) for r in test) >= TEST)
        self.assertEqual(len(fit) + len(test), 360)

    def test_refuses_a_split_without_enough_of_the_rarer_class(self):
        with self.assertRaises(SystemExit) as caught:
            baseline.train(rows(), "verdict", TEST)
        self.assertIn("0 of its rarer class", str(caught.exception))

    def test_learns_the_separating_feature_and_is_deterministic(self):
        data = rows()
        model = baseline.train(data, "outcome", TEST)
        self.assertEqual(model["version"], baseline.train(data, "outcome", TEST)["version"])
        bad = baseline.predict(model, dict(data[0], provider="bad"))
        good = baseline.predict(model, dict(data[0], provider="good"))
        self.assertGreater(bad, 0.8)
        self.assertLess(good, 0.2)
        report = baseline.report(model, data)
        self.assertEqual(report["model"]["rank_auc"], 1.0)
        self.assertLess(report["model"]["brier"], report["constant_base_rate"]["brier"])


class ScoreTests(unittest.TestCase):
    def test_scores_a_record_file_with_the_model_version(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            data = root / "dataset.jsonl"
            data.write_text("".join(json.dumps(r) + "\n" for r in rows()))
            model_path = root / "model.json"
            with contextlib.redirect_stdout(io.StringIO()):
                baseline.main(["train", str(data), str(model_path),
                               "--test-from", TEST.date().isoformat()])
            ident = run_id(START + timedelta(days=40), 1)
            record = root / f"{ident}.jsonl"
            record.write_text(json.dumps({
                "run_id": ident,
                "provider": "bad",
                "model": "m",
                "ts": "2026-10-11T00:00:00Z",
                "hydra": {"type": "run_start", "command": "run", "policy": "standard", "task": "x"},
            }) + "\n")
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                baseline.main(["score", str(model_path), str(record)])
            answer = json.loads(out.getvalue())
            self.assertEqual(answer["run_id"], ident)
            self.assertGreater(answer["p"], 0.8)
            self.assertEqual(answer["model_version"], json.loads(model_path.read_text())["version"])
            self.assertEqual(answer["label"], "outcome")


class TwoSliceTests(unittest.TestCase):
    """D2: a fit slice and a test slice, and no calibration slice or temperature."""

    def test_trains_on_two_slices_with_no_calibration_rows(self):
        data = rows()
        model = baseline.train(data, "outcome", TEST)
        self.assertEqual(set(model["rows"]), {"fit", "test"})
        self.assertEqual(model["rows"]["fit"] + model["rows"]["test"], len(data))
        self.assertNotIn("cal_from", model)
        self.assertNotIn("temperature", model)
        without = {k: v for k, v in model.items() if k != "temperature"}
        self.assertEqual(baseline.predict(without, data[0]), baseline.predict(model, data[0]))

    def test_the_cli_has_no_calibration_flag(self):
        with tempfile.TemporaryDirectory() as tmp:
            data = Path(tmp) / "dataset.jsonl"
            data.write_text("".join(json.dumps(r) + "\n" for r in rows()))
            with self.assertRaises(SystemExit), contextlib.redirect_stderr(io.StringIO()):
                baseline.main(["train", str(data), str(Path(tmp) / "m.json"),
                               "--cal-from", "2026-09-10", "--test-from", TEST.date().isoformat()])

    def test_the_floor_applies_to_the_test_slice(self):
        # Every test-slice run completes, so the test slice has none of the rarer class.
        data = [dict(r, outcome="completed") if baseline.launched_at(r["run_id"]) >= TEST else r
                for r in rows()]
        with self.assertRaises(SystemExit) as caught:
            baseline.train(data, "outcome", TEST)
        self.assertIn("test slice", str(caught.exception))


class ProviderBaseTests(unittest.TestCase):
    """D5: the bar is the fit slice's rate per provider and model, not one constant."""

    def test_rate_falls_back_from_pair_to_provider_to_overall(self):
        fit = [
            {"provider": "p", "model": "a", "outcome": "failed"},
            {"provider": "p", "model": "a", "outcome": "completed"},
            {"provider": "p", "model": "b", "outcome": "completed"},
            {"provider": "q", "model": "c", "outcome": "completed"},
        ]
        rate = baseline.provider_model_rates(fit, "outcome")
        # Shrunk with SHRINK = 5: overall 1/4; provider p (1 + 5 * 0.25) / (3 + 5) = 0.28125;
        # pair (p, a) (1 + 5 * 0.28125) / (2 + 5) = 0.34375. An unseen pair takes its
        # provider's rate, an unseen provider the overall one.
        self.assertAlmostEqual(rate({"provider": "p", "model": "a"}), 0.34375)
        self.assertAlmostEqual(rate({"provider": "p", "model": "z"}), 0.28125)
        self.assertAlmostEqual(rate({"provider": "r", "model": "z"}), 0.25)

    def test_a_model_that_only_learned_the_provider_does_not_beat_its_rate(self):
        data = rows()
        report = baseline.report(baseline.train(data, "outcome", TEST), data)
        bar = report["provider_model_base_rate"]
        self.assertIn("log_loss", bar)
        self.assertIn("brier", bar)
        self.assertLess(report["model"]["brier"], report["constant_base_rate"]["brier"])
        self.assertEqual(report["beats_provider_model_base"], {"log_loss": False, "brier": False})


class TwoModelTests(unittest.TestCase):
    """D1: outcome and verdict are two models, each in its own file."""

    def test_each_label_has_its_own_file_under_the_grader_home(self):
        home = Path("/scratch/grader")
        self.assertEqual(baseline.model_path("outcome", home), home / "models" / "outcome.json")
        self.assertEqual(baseline.model_path("verdict", home), home / "models" / "verdict.json")

    def test_train_without_a_path_writes_the_labels_file(self):
        with tempfile.TemporaryDirectory() as tmp:
            home = Path(tmp)
            data = home / "dataset.jsonl"
            data.write_text("".join(json.dumps(r) + "\n" for r in rows()))
            with mock.patch.dict(os.environ, {"HYDRA_GRADER_HOME": str(home)}):
                with contextlib.redirect_stdout(io.StringIO()):
                    baseline.main(["train", str(data), "--test-from", TEST.date().isoformat()])
            self.assertEqual(json.loads((home / "models" / "outcome.json").read_text())["label"],
                             "outcome")

    def test_verdict_refusal_says_how_many_rows_are_labelled(self):
        data = rows()
        for r in data[:7]:
            r["human_verdict"] = "fixed"
        with self.assertRaises(SystemExit) as caught:
            baseline.train(data, "verdict", TEST)
        self.assertIn("7 labelled rows", str(caught.exception))


OUTCOME_ONE = """# 200 - One

Type: build
Status: resolved

## What

run 01a0aaaa-0000 is named here and must not count.

## Outcome

Run `01a0e79c` wrote the parser and the session applied it. Then run 01a0d618-6b6e reworked
the tests, and commit f38881f2 went in.

## Notes

run 01a0bbbb-0000 is past the Outcome.
"""

OUTCOME_OPEN = """# 201 - Two

Type: build
Status: open

## Outcome

Run `01a0e79c` again, but the ticket is open.
"""


class LabelQueueTests(unittest.TestCase):
    """D3: a CSV the human fills in; nothing here decides a verdict."""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        root = Path(self.tmp.name)
        self.tickets = root / "build"
        self.tickets.mkdir()
        (self.tickets / "200-one.md").write_text(OUTCOME_ONE)
        (self.tickets / "201-two.md").write_text(OUTCOME_OPEN)
        self.dataset = root / "dataset.jsonl"
        self.dataset.write_text("".join(json.dumps(r) + "\n" for r in [
            {"run_id": "01a0e79c-1111-7000-8000-000000000001", "provider": "codex", "model": "gpt",
             "human_verdict": None},
            {"run_id": "01a0d618-6b6e-7000-8000-000000000002", "provider": "claude",
             "model": "opus", "human_verdict": None},
            {"run_id": "01a0bbbb-0000-7000-8000-000000000003", "provider": "agy", "model": "x",
             "human_verdict": None},
        ]))
        self.out = root / "queue.csv"

    def queue(self):
        with contextlib.redirect_stdout(io.StringIO()):
            baseline.main(["label-queue", str(self.out), "--tickets", str(self.tickets),
                           "--dataset", str(self.dataset)])
        with self.out.open(newline="") as handle:
            return list(csv.DictReader(handle))

    def test_one_row_per_run_the_outcome_of_a_resolved_ticket_names(self):
        got = self.queue()
        self.assertEqual([(r["ticket"], r["run_id"]) for r in got], [
            ("200", "01a0e79c-1111-7000-8000-000000000001"),
            ("200", "01a0d618-6b6e-7000-8000-000000000002"),
        ])
        self.assertEqual((got[0]["provider"], got[0]["model"]), ("codex", "gpt"))
        self.assertTrue(got[0]["outcome"].startswith("Run `01a0e79c` wrote"))
        self.assertLessEqual(len(got[0]["outcome"]), 400)
        self.assertEqual({r["verdict"] for r in got}, {""})

    def test_the_outcome_is_cut_at_400_characters(self):
        (self.tickets / "200-one.md").write_text(OUTCOME_ONE.replace(
            "## Notes", "x" * 1000 + "\n\n## Notes"))
        self.assertEqual(len(self.queue()[0]["outcome"]), 400)

    def test_import_puts_the_humans_verdicts_on_the_dataset(self):
        got = self.queue()
        got[0]["verdict"] = "fixed"
        got[1]["verdict"] = "unknown"
        with self.out.open("w", newline="") as handle:
            writer = csv.DictWriter(handle, fieldnames=list(got[0]))
            writer.writeheader()
            writer.writerows(got)
        home = Path(self.tmp.name) / "home"
        with contextlib.redirect_stdout(io.StringIO()):
            baseline.main(["label-import", str(self.out), "--dataset", str(self.dataset),
                           "--home", str(home)])
        by_id = {r["run_id"]: r for r in baseline.records.read_jsonl(self.dataset)}
        self.assertEqual(by_id["01a0e79c-1111-7000-8000-000000000001"]["human_verdict"], "fixed")
        self.assertIsNone(by_id["01a0d618-6b6e-7000-8000-000000000002"]["human_verdict"])
        kept = json.loads((home / "ticket-verdicts.json").read_text())
        self.assertEqual(kept, {"01a0e79c-1111-7000-8000-000000000001": "fixed"})

    def test_import_refuses_a_verdict_outside_the_three(self):
        got = self.queue()
        got[0]["verdict"] = "good"
        with self.out.open("w", newline="") as handle:
            writer = csv.DictWriter(handle, fieldnames=list(got[0]))
            writer.writeheader()
            writer.writerows(got)
        with self.assertRaises(SystemExit) as caught:
            baseline.main(["label-import", str(self.out), "--dataset", str(self.dataset),
                           "--home", str(Path(self.tmp.name) / "home")])
        self.assertIn("good", str(caught.exception))


class SmoothedBarTests(unittest.TestCase):
    def test_a_pair_that_never_failed_in_fit_is_not_priced_at_zero(self):
        # An unsmoothed bar predicts 0 for such a pair and pays about 27.6 log loss on its
        # first failure in test, which lets any model beat it without merit.
        fit_rows = [
            {"provider": "a", "model": "m", "outcome": "completed"},
            {"provider": "a", "model": "m", "outcome": "completed"},
            {"provider": "b", "model": "m", "outcome": "rate_limited"},
            {"provider": "b", "model": "m", "outcome": "completed"},
        ]
        rate = baseline.provider_model_rates(fit_rows, "outcome")
        p = rate({"provider": "a", "model": "m"})
        self.assertGreater(p, 0.0)
        self.assertLess(p, 0.25)
        q = rate({"provider": "b", "model": "m"})
        self.assertGreater(q, p)
        self.assertLess(q, 1.0)


if __name__ == "__main__":
    unittest.main()
