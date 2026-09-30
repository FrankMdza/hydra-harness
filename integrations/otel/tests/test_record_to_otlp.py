import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

HERE = Path(__file__).resolve().parents[1]
FIXTURES = HERE.parents[1] / 'tests/fixtures/records'


class TranslatorTests(unittest.TestCase):
    def setUp(self):
        spec = importlib.util.spec_from_file_location('otel', HERE / 'record_to_otlp.py')
        self.otel = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.otel)
        self.fixture = next(FIXTURES.glob('claude*/*001.jsonl'))

    def spans(self, request):
        return request['resourceSpans'][0]['scopeSpans'][0]['spans']

    def attrs(self, span):
        return {a['key']: next(iter(a['value'].values())) for a in span['attributes']}

    def test_all_provider_and_harness_fixtures(self):
        for path in FIXTURES.rglob('*.jsonl'):
            with self.subTest(path=path.name):
                spans = self.spans(self.otel.convert(path))
                self.assertEqual(spans[0]['name'], 'hydra.run')
                self.assertEqual(len(spans[0]['traceId']), 32)
                self.assertGreaterEqual(int(spans[0]['endTimeUnixNano']), int(spans[0]['startTimeUnixNano']))
                if path.stem.endswith('006'):
                    self.assertFalse(self.attrs(spans[0])['run.complete'])

    def test_tools_tokens_decisions_and_sizes(self):
        spans = self.spans(self.otel.convert(self.fixture))
        self.assertEqual(len(spans), 2)
        attrs = self.attrs(spans[0])
        self.assertEqual(attrs['outcome'], 'gated')
        self.assertEqual(attrs['tokens.output_tokens'], '1270')
        tool = spans[1]
        self.assertEqual(tool['parentSpanId'], spans[0]['spanId'])
        self.assertEqual(self.attrs(tool)['gate.decision'], 'deny')
        self.assertGreater(int(self.attrs(tool)['output.size_bytes']), 0)
        rows = [json.loads(line) for line in self.fixture.read_text().splitlines()]
        self.assertEqual(tool['startTimeUnixNano'], self.otel.nano(rows[2]['ts']))
        self.assertEqual(tool['endTimeUnixNano'], self.otel.nano(rows[4]['ts']))
        self.assertNotIn('HELLO', json.dumps(spans))

    def test_redaction_before_task_truncation(self):
        rows = [json.loads(line) for line in self.fixture.read_text().splitlines()]
        rows[0]['hydra']['task'] = 'x' * 1990 + ' sk-proj-' + 'A' * 40
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / self.fixture.name
            path.write_text(''.join(json.dumps(row) + '\n' for row in rows))
            result = self.otel.convert(path)
        task = self.attrs(self.spans(result)[0])['task']
        self.assertLessEqual(len(task), 2000)
        self.assertNotIn('sk-proj-', json.dumps(result))

    def test_seats_and_multilevel_trace_parent(self):
        rows = [json.loads(line) for line in self.fixture.read_text().splitlines()]
        session, parent, child = ['00000000-0000-7000-8000-0000000000' + n for n in ('21', '22', '23')]
        with tempfile.TemporaryDirectory() as tmp:
            for run_id, parent_id in ((session, None), (parent, session), (child, parent)):
                copied = [dict(row, run_id=run_id, parent_run_id=parent_id) for row in rows]
                copied.insert(-1, dict(copied[0], seq=99, hydra={'type': 'seat_request', 'request_id': 'request', 'decision': 'launched', 'runs': [child]}))
                path = Path(tmp) / (run_id + '.jsonl')
                path.write_text(''.join(json.dumps(row) + '\n' for row in copied))
            root = self.spans(self.otel.convert(Path(tmp) / (session + '.jsonl')))[0]
            middle = self.spans(self.otel.convert(Path(tmp) / (parent + '.jsonl')))[0]
            spans = self.spans(self.otel.convert(path))
        self.assertEqual(root['traceId'], spans[0]['traceId'])
        self.assertEqual(middle['spanId'], spans[0]['parentSpanId'])
        seat = next(s for s in spans if s['name'] == 'hydra.seat_request')
        self.assertEqual(seat['links'][0]['spanId'], spans[0]['spanId'])

    def test_batch_length_skip_and_refresh(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / 'out'
            runs = Path(tmp) / 'runs'
            runs.mkdir()
            path = runs / self.fixture.name
            path.write_bytes(self.fixture.read_bytes())
            self.otel.batch(runs, out)
            target = out / (path.stem + '.json')
            before = target.stat().st_mtime_ns
            self.otel.batch(runs, out)
            self.assertEqual(before, target.stat().st_mtime_ns)
            path.write_bytes(path.read_bytes() + b'\n')
            self.otel.batch(runs, out)
            self.assertNotEqual(before, target.stat().st_mtime_ns)

    def test_cli_file_and_stdout(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / 'trace.json'
            command = [sys.executable, str(HERE / 'record_to_otlp.py'), str(self.fixture)]
            stdout = subprocess.run(command, check=True, capture_output=True, text=True)
            subprocess.run(command + ['--out', str(out)], check=True, capture_output=True)
            self.assertEqual(json.loads(stdout.stdout), json.loads(out.read_text()))


if __name__ == '__main__':
    unittest.main()
