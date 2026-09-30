"""Translate Hydra records to local OTLP JSON files, using only the stdlib."""
from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

_spec = importlib.util.spec_from_file_location(
    'otel_redact', Path(__file__).resolve().parents[1] / 'grader/redact.py'
)
_redact = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_redact)


def nano(timestamp: str) -> str:
    dt = datetime.fromisoformat(timestamp.replace('Z', '+00:00'))
    delta = dt - datetime(1970, 1, 1, tzinfo=timezone.utc)
    return str((delta.days * 86400 + delta.seconds) * 1_000_000_000 + delta.microseconds * 1000)


def identifier(value: str, length: int) -> str:
    return hashlib.sha256(value.encode()).hexdigest()[:length]


def span_id(run_id: str) -> str:
    return identifier('run:' + run_id, 16)


def attributes(values: dict) -> list:
    result = []
    for key, value in values.items():
        if value is None:
            continue
        if isinstance(value, bool):
            encoded = {'boolValue': value}
        elif isinstance(value, int):
            encoded = {'intValue': str(value)}
        else:
            encoded = {'stringValue': _redact.redact_string(str(value))}
        result.append({'key': _redact.redact_string(key), 'value': encoded})
    return result


def read_rows(path: Path) -> list:
    rows = []
    for line in path.read_bytes().splitlines(keepends=True):
        if not line.endswith(b'\n'):
            break
        if line.strip():
            rows.append(json.loads(line))
    if not rows or rows[0].get('hydra', {}).get('type') != 'run_start':
        raise ValueError('record must begin with run_start')
    return rows


def normalized(path: Path, rows: list) -> dict:
    # Copy only this record. No discovery, harvesting or access to the user's runs.
    with tempfile.TemporaryDirectory() as tmp:
        runs = Path(tmp) / 'runs'
        runs.mkdir()
        run_id = rows[0]['run_id']
        (runs / (run_id + '.jsonl')).write_text(
            ''.join(json.dumps(row) + '\n' for row in rows), encoding='utf-8'
        )
        answer = subprocess.run(
            [os.environ.get('HYDRA_BIN', 'hydra'), 'show', run_id, '--events'],
            env=dict(os.environ, HYDRA_ROOT=tmp), capture_output=True, text=True, check=True,
        )
    return json.loads(answer.stdout)


def session_id(first: dict, directory: Path) -> str:
    current = first
    seen = {first['run_id']}
    while current.get('parent_run_id'):
        parent = current['parent_run_id']
        if parent in seen:
            raise ValueError('cycle in parent_run_id')
        seen.add(parent)
        path = directory / (parent + '.jsonl')
        if not path.is_file():
            return parent
        current = read_rows(path)[0]
    return current['run_id']


def convert(path: Path) -> dict:
    rows = read_rows(path)
    first = rows[0]
    run_id = first['run_id']
    start = first['hydra']
    answer = normalized(path, rows)
    projection = answer.get('projection') or {}
    end = next((r['hydra'] for r in reversed(rows)
                if r.get('hydra', {}).get('type') == 'run_end'), {})
    trace_id = identifier('trace:' + session_id(first, path.parent), 32)
    times = {r['seq']: nano(r['ts']) for r in rows}
    root_start = nano(first['ts'])
    root_end = nano(end['ended_at']) if end.get('ended_at') else times[rows[-1]['seq']]
    usage = projection.get('usage') or {}
    values = {key: first.get(key) for key in ('provider', 'model')}
    values.update(policy=start.get('policy'), outcome=projection.get('outcome'),
                  task=_redact.redact_string(start.get('task') or '')[:2000],
                  **{'usage.source': usage.get('usage_source'), 'run.complete': bool(end)})
    values.update({'tokens.' + k: v for k, v in (usage.get('counts') or {}).items()})

    def span(name, sid, begin, finish, attrs, parent=None):
        result = {'traceId': trace_id, 'spanId': sid,
                  'name': _redact.redact_string(name), 'kind': 1,
                  'startTimeUnixNano': begin, 'endTimeUnixNano': finish,
                  'attributes': attributes(attrs)}
        if parent:
            result['parentSpanId'] = parent
        return result

    root = span('hydra.run', span_id(run_id), root_start, root_end, values,
                span_id(first['parent_run_id']) if first.get('parent_run_id') else None)
    spans = [root]
    events = answer.get('events') or []
    decisions = {e['call_id']: e for e in events if e['type'] == 'permission_decision'}
    results = {e['call_id']: e for e in events if e['type'] == 'tool_result'}
    for event in events:
        if event['type'] != 'tool_call':
            continue
        call_id = event['call_id']
        result = results.get(call_id, {})
        decision = decisions.get(call_id, {})
        begin = times[event['seq']]
        finish = times[result['seq']] if result else begin
        attrs = {'tool.name': event['name'], 'gate.decision': decision.get('decision'),
                 'gate.decider': decision.get('decider'), 'result.status': result.get('status'),
                 'timing.complete': bool(result)}
        if result:
            attrs['output.size_bytes'] = len((result.get('output') or '').encode('utf-8'))
        spans.append(span(event['name'], identifier(f'{run_id}:tool:{event["seq"]}:{call_id}', 16),
                          begin, finish, attrs, root['spanId']))
    for row in rows:
        body = row.get('hydra') or {}
        if body.get('type') != 'seat_request':
            continue
        seat = span('hydra.seat_request', identifier(f'{run_id}:seat:{row["seq"]}', 16),
                    times[row['seq']], times[row['seq']],
                    {'gate.decision': body.get('decision'), 'shape': body.get('shape')}, root['spanId'])
        seat['links'] = [{'traceId': trace_id, 'spanId': span_id(child)} for child in body.get('runs', [])]
        spans.append(seat)
    return {'resourceSpans': [{'resource': {'attributes': attributes({'service.name': 'hydra'})},
                               'scopeSpans': [{'scope': {'name': 'hydra.otel'}, 'spans': spans}]}]}


def write_json(path: Path, value) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temp = path.with_suffix(path.suffix + '.tmp')
    temp.write_text(json.dumps(value, indent=2, allow_nan=False) + '\n', encoding='utf-8')
    temp.replace(path)


def batch(runs: Path, out: Path) -> None:
    for path in sorted(runs.rglob('*.jsonl')):
        target = out / (path.stem + '.json')
        stamp = out / (path.stem + '.size.json')
        size = path.stat().st_size
        if target.is_file() and stamp.is_file() and json.loads(stamp.read_text()) == size:
            continue
        write_json(target, convert(path))
        write_json(stamp, size)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('record', nargs='?', type=Path)
    parser.add_argument('--out', type=Path)
    parser.add_argument('--all', dest='runs', type=Path)
    parser.add_argument('--out-dir', type=Path)
    args = parser.parse_args()
    if args.runs:
        if not args.out_dir or args.record or args.out:
            parser.error('--all requires --out-dir and cannot be combined with record or --out')
        batch(args.runs, args.out_dir)
    else:
        if not args.record or args.out_dir:
            parser.error('provide a record, or --all with --out-dir')
        result = convert(args.record)
        if args.out:
            write_json(args.out, result)
        else:
            json.dump(result, sys.stdout, indent=2, allow_nan=False)
            print()


if __name__ == '__main__':
    main()
