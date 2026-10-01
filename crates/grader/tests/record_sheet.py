"""Record sheet.py before its cutover. Run from the checkout root with python3.
Only sheet/scenarios.json is written. Python is never started by replay tests.
"""
import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
CHILD = '''import importlib.util, sys
from pathlib import Path
spec = importlib.util.spec_from_file_location('sheet', sys.argv[1])
m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m)
m.HERE = Path('data')
m.load_aliases = lambda: m._models.load_aliases(Path('data/aliases.json'))
sys.argv = ['sheet.py', *sys.argv[2:]]
m.main()
'''
# The same shell stub as collect, including recorded arguments and environment.
spec = importlib.util.spec_from_file_location('record_collect', HERE / 'record_collect.py')
collect = importlib.util.module_from_spec(spec)
spec.loader.exec_module(collect)
STUB = collect.HYDRA_STUB

def normalize(text, root):
    text = text.replace(str(root), '<root>')
    text = re.sub(r'env: HYDRA_ROOT=(?!<root>)[^\n]+', 'env: HYDRA_ROOT=<tmp>', text)
    text = re.sub(r'"date": "\d{4}-\d{2}-\d{2}"', '"date": "<date>"', text)
    return text

def record(out=HERE / 'fixtures', repo=REPO):
    data = {name: (HERE / 'fixtures/data' / name).read_text() for name in ('aliases.json',)}
    data['vendors.json'] = (repo / 'integrations/puppets/vendors.json').read_text()
    cases = []
    def case(name, records=(), steps=None, vendors=None, extra=None, corpus='root/runs'):
        files = {'bin/hydra': STUB, **{f'data/{k}': v for k, v in data.items()}}
        if vendors is not None:
            files['data/vendors.json'] = json.dumps(vendors) + '\n'
        for i, options in enumerate(records):
            options = dict(options)
            ident = options.pop('id', f'run-{i}')
            provider = options.pop('provider', 'claude')
            model = options.pop('model', 'sonnet')
            parent = options.pop('parent', None)
            policy = options.pop('policy', 'standard')
            start = {'run_id': ident, 'provider': provider, 'model': model,
                     'parent_run_id': parent, 'ts': options.pop('ts', '2026-09-24T10:00:00Z'),
                     'hydra': {'type': 'run_start', 'command': options.pop('command', 'run'),
                               'policy': policy, 'subscription': 'test', 'provider_version': '1', 'launch_mode': 'recorded'}}
            lines = [start]
            for rate in options.pop('rates', []):
                lines.append({'raw': {'type': 'rate_limit_event', 'nested': {'seven_day': {'utilization': rate}}}})
            if options.pop('ended', True):
                lines.append({'hydra': {'type': 'run_end', 'ended_at': options.pop('end', '2026-09-24T10:00:10Z')}})
            files[f'{corpus}/{ident}.jsonl'] = ''.join(json.dumps(v) + '\n' for v in lines) + options.pop('tail', '')
            answer = {'projection': {'outcome': options.pop('outcome', 'completed'),
                      'result_text': options.pop('text', None), 'usage': options.pop('usage', {'usage_source': 'provider', 'counts': {'raw_input': 100, 'output_tokens': 20, 'cache_read_tokens': 80}})},
                      'events': [{'type': 'permission_decision', 'decision': 'deny'}]}
            files[f'bin/table/{ident}'] = json.dumps(answer) + '\n'
            assert not options, options
        files.update(extra or {})
        if steps is None:
            steps = [['build', '--runs', corpus], ['build', '--runs', corpus], ['show', 'claude', 'sonnet']]
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp).resolve()
            (root / corpus).mkdir(parents=True)
            for path, text in files.items():
                target = root / path; target.parent.mkdir(parents=True, exist_ok=True); target.write_text(text)
            (root / 'bin/hydra').chmod(0o755)
            env = {'HYDRA_ROOT': str(root / 'root'), 'HYDRA_PUPPETS_HOME': str(root / 'home'),
                   'HYDRA_PUPPETS_DATA': str(root / 'data'), 'HYDRA_BIN': str(root / 'bin/hydra'), 'PATH': '/usr/bin:/bin'}
            recorded = []
            for step in steps:
                argv = step['argv'] if isinstance(step, dict) else step
                put = step.get('put', {}) if isinstance(step, dict) else {}
                for path, text in put.items():
                    target = root / path; target.parent.mkdir(parents=True, exist_ok=True); target.write_text(text)
                    files.setdefault(path, text)
                done = subprocess.run([sys.executable, '-c', CHILD, str(repo / 'integrations/puppets/sheet.py'), *argv], cwd=root, env=env, stdin=subprocess.DEVNULL, capture_output=True, text=True)
                stderr = done.stderr
                if done.returncode == 1:
                    errors = [line for line in stderr.splitlines() if re.match(r'^[\w.]+(?:Error|Exception):', line)]
                    stderr = errors[-1].split(':')[0].split('.')[-1] + ':'
                elif done.returncode == 2:
                    stderr = stderr.rstrip().splitlines()[-1].replace('sheet.py', 'hydra-grader sheet')
                snapshot = {str(p.relative_to(root)): normalize(p.read_text(), root) for p in root.rglob('*') if p.is_file() and (str(p.relative_to(root)) not in files or str(p.relative_to(root)).startswith('home/') or str(p.relative_to(root)) == 'data/vendors.json')}
                recorded.append({'argv': argv, 'put': put, 'code': done.returncode, 'stdout': normalize(done.stdout, root), 'stderr': normalize(stderr, root), 'files': snapshot})
            cases.append({'name': name, 'corpus': corpus, 'input': files, 'steps': recorded})
    vendors = [{'provider': 'claude', 'model': 'sonnet'}, {'provider': 'codex', 'model': None, 'windows': '5h', 'window_budget_tokens': 1000}]
    case('rollup and cache', [{'id': 'a', 'rates': [.1, .3], 'outcome': 'gated'}, {'id': 'b', 'provider': 'codex', 'model': None, 'policy': 'judge', 'text': '```json\n{"verdict":"pass","findings":[]}\n```'}, {'id': 'c', 'parent': 'a', 'command': 'resume', 'rates': [.3, .4]}, {'id': 'd', 'ended': False, 'outcome': None}, {'id': 'excluded', 'command': 'curate'}], vendors=vendors)
    case('aliases, reset, tail and stale sheets', [{'model': 'Sonnet 5', 'rates': [.9, .1]}, {'model': 'claude-sonnet-5', 'tail': '{"torn":'}, {'provider': 'codex', 'model': None, 'usage': {'usage_source': 'unavailable'}}], vendors=vendors, extra={'home/agents/stale.json': '{}', 'home/agents/keep.txt': 'keep'})
    refreshed = json.dumps({'run_id': 'run-0', 'provider': 'claude', 'model': 'sonnet', 'ts': '2026-09-24T10:00:00Z', 'hydra': {'command': 'run', 'policy': 'standard'}}) + '\n'
    case('live refresh on a torn tail', [{}], vendors=vendors, steps=[['build'], {'argv': ['build'], 'put': {'root/runs/run-0.jsonl': refreshed + '{"torn":'}}, ['build']])
    case('weighted cost', [{'provider': 'codex', 'model': None}], vendors=vendors, steps=[['build', '--cache-read-weight', '.5'], ['show', 'codex']])
    case('custom corpus', [{'rates': [.1, .2]}], vendors=vendors, corpus='corpus')
    case('empty corpus', vendors=vendors)
    case('filename quoting and cycles', [{'provider': 'x/y', 'model': '~null é', 'id': 'a', 'parent': 'b'}, {'provider': 'x/y', 'model': '~null é', 'id': 'b', 'parent': 'a'}], vendors=[], steps=[['build'], ['show', 'x/y', '~null é']])
    for text in ['{}', 'no', '{"verdict":[],"findings":[]}', '{"verdict":"block","findings":[{"item":"security_issues","severity":"critical","note":"x"}]}', '{"verdict":"pass","findings":[{"item":"unknown","severity":"minor","note":"x"}]}']:
        case('judge ' + text, [{'policy': 'judge', 'text': text}], vendors=vendors)
    limited = [{'id': 'run-a', 'provider': 'codex', 'model': None}, {'id': 'run-b', 'provider': 'codex', 'model': None, 'outcome': 'rate_limited', 'end': '2026-09-24T12:00:00Z'}]
    case('budget and idempotence', limited, vendors=vendors, steps=[['budget', 'run-b', '--vendors', 'data/vendors.json'], ['budget', 'run-b', '--vendors', 'data/vendors.json']])
    case('budget since', limited, vendors=[{'provider': 'codex', 'model': None}], steps=[['budget', 'run-b', '--vendors', 'data/vendors.json'], ['budget', 'run-b', '--vendors', 'data/vendors.json', '--since', '2026-09-24T11:00:00Z'], ['budget', 'run-b', '--vendors', 'data/vendors.json', '--since', '2026-09-24T09:00:00Z']])
    for windows in ['0.25d', {'hours': 5}, {'days': 1}, 'bad', -1]:
        case('budget window ' + repr(windows), limited, vendors=[{'provider': 'codex', 'model': None, 'windows': windows}], steps=[['budget', 'run-b', '--vendors', 'data/vendors.json', '--cache-read-weight', '.5']])
    for name, records, vendor_rows, ident in [
        ('missing run', limited, vendors, 'missing'),
        ('not limited', limited, vendors, 'run-a'),
        ('claude budget', [{'id': 'run-a', 'outcome': 'rate_limited'}], vendors, 'run-a'),
        ('no vendor', limited, [], 'run-b'),
        ('invalid spend', [{'id': 'run-b', 'provider': 'codex', 'model': None, 'outcome': 'rate_limited', 'usage': {'counts': {'raw_input': True, 'output_tokens': 20}}}], vendors, 'run-b'),
    ]:
        case(name, records, vendors=vendor_rows, steps=[['budget', ident, '--vendors', 'data/vendors.json']])
    case('budget overlaps and boundary', limited + [
        {'id': 'old', 'provider': 'codex', 'model': None, 'ts': '2026-09-24T06:00:00Z', 'end': '2026-09-24T06:59:59Z'},
        {'id': 'boundary', 'provider': 'codex', 'model': None, 'ts': '2026-09-24T06:00:00Z', 'end': '2026-09-24T07:00:00Z'},
        {'id': 'future', 'provider': 'codex', 'model': None, 'ts': '2026-09-24T12:00:01Z', 'end': '2026-09-24T13:00:00Z'},
        {'id': 'unfinished', 'provider': 'codex', 'model': None, 'ended': False},
    ], vendors=vendors, steps=[['budget', 'run-b', '--vendors', 'data/vendors.json']])
    case('null model and literal null marker', [{'provider': 'x', 'model': None}, {'provider': 'x', 'model': '~null'}], vendors=[], steps=[['build'], ['show', 'x'], ['show', 'x', '~null']])
    case('nonfinite answer refuses cache', [{'usage': {'counts': {'raw_input': float('nan'), 'output_tokens': 1}}}], vendors=vendors, steps=[['build']])
    case('bad arguments', vendors=vendors, steps=[[], ['--help'], ['build', '--help'], ['build', '--cache-read-weight', 'nan'], ['budget'], ['show'], ['show', 'none'], ['build', '--cache-read-weight', '-1'], ['build', '--cache-read-weight', 'x'], ['build', '--runs'], ['build', 'extra'], ['build', '--ho=home'], ['--', 'build']])
    for name, extra in [('malformed record', {'root/runs/a.jsonl': 'no\n'}), ('wrong first line', {'root/runs/a.jsonl': '[]\n'}), ('failed show', {'bin/table/run-0.exit': '3\n'}), ('bad timestamp', {})]:
        records = [{'end': 'soon'}] if name == 'bad timestamp' else [{}]
        case(name, records, vendors=vendors, extra=extra, steps=[['build']])
    target = out / 'sheet'; target.mkdir(exist_ok=True)
    goal = [{'seq': 1, 'ts': '2026-09-24T00:00:00Z', 'event': 'goal', 'text': 'test', 'repo': '/repo', 'base_sha': 'abc'},
            {'event': 'task', 'id': 'accepted', 'class': 'rust_launch_path', 'grade': 'medium', 'text': 'accepted work'},
            {'event': 'human_verdict', 'task': 'accepted', 'run_id': 'a', 'result': 'applied'},
            {'event': 'task', 'id': 'next', 'class': 'rust_launch_path', 'grade': 'medium', 'text': 'next work'}]
    spec = importlib.util.spec_from_file_location('rank_reference', repo / 'integrations/puppets/sheet.py')
    reference = importlib.util.module_from_spec(spec); spec.loader.exec_module(reference)
    sheets = [json.loads(text) for path, text in cases[0]['steps'][0]['files'].items() if path.startswith('home/agents/')]
    priors = json.loads((repo / 'integrations/puppets/priors.json').read_text())
    ranked = reference.rank(sheets, 'rust_launch_path', 'medium', goal, priors=priors)
    (target / 'assignment.json').write_text(json.dumps({'case': 0, 'goal': goal, 'priors': priors, 'ranked': ranked}, indent=1) + '\n')
    (target / 'scenarios.json').write_text(json.dumps({'cases': cases}, indent=1) + '\n')

if __name__ == '__main__':
    record(repo=Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else REPO)
