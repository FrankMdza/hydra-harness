import contextlib
import importlib.util
import io
import json
from datetime import datetime, timedelta, timezone
from pathlib import Path
import tempfile
import unittest

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


CAL = START + timedelta(days=15)
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
        fit, cal, test = baseline.split(rows(), CAL, TEST)
        self.assertTrue(max(baseline.launched_at(r["run_id"]) for r in fit) < CAL)
        self.assertTrue(min(baseline.launched_at(r["run_id"]) for r in test) >= TEST)
        self.assertEqual(len(fit) + len(cal) + len(test), 360)

    def test_refuses_a_split_without_enough_of_the_rarer_class(self):
        with self.assertRaises(SystemExit) as caught:
            baseline.train(rows(), "verdict", CAL, TEST)
        self.assertIn("0 of its rarer class", str(caught.exception))

    def test_learns_the_separating_feature_and_is_deterministic(self):
        data = rows()
        model = baseline.train(data, "outcome", CAL, TEST)
        self.assertEqual(model["version"], baseline.train(data, "outcome", CAL, TEST)["version"])
        bad = baseline.predict(model, dict(data[0], provider="bad"))
        good = baseline.predict(model, dict(data[0], provider="good"))
        self.assertGreater(bad, 0.8)
        self.assertLess(good, 0.2)
        report = baseline.report(model, data)
        self.assertEqual(report["calibrated"]["rank_auc"], 1.0)
        self.assertLess(report["calibrated"]["brier"], report["constant_base_rate"]["brier"])


class ScoreTests(unittest.TestCase):
    def test_scores_a_record_file_with_the_model_version(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            data = root / "dataset.jsonl"
            data.write_text("".join(json.dumps(r) + "\n" for r in rows()))
            model_path = root / "model.json"
            with contextlib.redirect_stdout(io.StringIO()):
                baseline.main(["train", str(data), str(model_path),
                               "--cal-from", CAL.date().isoformat(),
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


if __name__ == "__main__":
    unittest.main()
