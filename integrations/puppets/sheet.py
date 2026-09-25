"""Corpus spec sheets and conservative, window-share rankings (stdlib only)."""
import argparse
from collections import Counter
from datetime import datetime, timedelta
import hashlib
import importlib.util
import json
import math
import os
from pathlib import Path
import statistics
import subprocess
import tempfile
from urllib.parse import quote

HERE = Path(__file__).resolve().parent
_models_spec = importlib.util.spec_from_file_location('puppets_models', HERE / 'models.py')
_models = importlib.util.module_from_spec(_models_spec)
_models_spec.loader.exec_module(_models)
canonical = _models.canonical
load_aliases = _models.load_aliases

POLICIES = ['judge', 'read-only', 'standard', 'permissive', 'curate']
CHECKLIST = set('secrets_check security_issues tests_affected self_consistency code_quality cross_platform changelog_accuracy approved_patch_unreachable severity_ceiling_table reviewer_failure_fails_closed panel_size_unshrunk protected_path_set admission_gate_order baseline_test_provenance env_allowlist_intact hook_config_written_fresh hook_dialect_coverage billed_spend_gate delegation_depth_cap curator_write_surface evidence_is_read_only worktree_containment record_append_only record_source_fields raw_frame_preserved export_losslessness provider_claim_dated no_new_crossing no_daemon spec_matches_code'.split())



def read_json(path, default=None):
    return json.loads(Path(path).read_text()) if Path(path).exists() else default


def write_json(path, value):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    temp = path.with_suffix(path.suffix + '.tmp')
    temp.write_text(json.dumps(value, indent=2, allow_nan=False) + '\n')
    temp.replace(path)


def filename(provider, model):
    # Reserve an unambiguous marker for an unknown model.
    return quote(provider, safe='') + '--' + ('~null' if model is None else quote(model, safe='').replace('~', '%7E')) + '.json'


def statistic(values):
    values = [v for v in values if isinstance(v, (int, float)) and not isinstance(v, bool) and math.isfinite(v)]
    return {'p50': statistics.median(values) if values else None, 'n': len(values)}


def judge_parsed(text):
    if not isinstance(text, str):
        return False
    text = text.strip()
    if text.startswith('```') and '\n' in text:
        tag, body = text[3:].split('\n', 1)
        if tag.strip().lower() in ('', 'json') and body.rstrip().endswith('```'):
            body = body.rstrip()[:-3]
            if '```' not in body:
                text = body.strip()
    try:
        obj = json.loads(text)
        return (isinstance(obj, dict) and {'verdict', 'findings'} <= set(obj)
                and obj['verdict'] in ('pass', 'block') and isinstance(obj['findings'], list)
                and all(isinstance(f, dict) and {'item', 'severity', 'note'} <= set(f)
                        and isinstance(f['item'], str) and f['item'] in CHECKLIST
                        and isinstance(f['severity'], str) and f['severity'] in ('critical', 'major', 'minor')
                        and isinstance(f['note'], str) for f in obj['findings']))
    except (ValueError, TypeError):
        return False


def utilization(raw):
    if not isinstance(raw, dict):
        return None
    if isinstance(raw.get('seven_day'), dict):
        value = raw['seven_day'].get('utilization')
        if isinstance(value, (float, int)) and not isinstance(value, bool) and math.isfinite(value):
            return value
    for value in raw.values():
        if isinstance(value, dict):
            found = utilization(value)
            if found is not None:
                return found
    return None


def parse_iso(ts):
    return datetime.fromisoformat(ts.replace('Z', '+00:00'))


def token_spend(counts, cache_read_weight=1.0):
    if not isinstance(counts, dict):
        return None
    raw = counts.get('raw_input')
    out = counts.get('output_tokens')
    if not isinstance(raw, (int, float)) or isinstance(raw, bool) or not math.isfinite(raw):
        return None
    if not isinstance(out, (int, float)) or isinstance(out, bool) or not math.isfinite(out):
        return None
    return raw * cache_read_weight + out


def window_duration(vendor, provider):
    windows = vendor.get('windows')
    if isinstance(windows, str):
        text = windows.strip().lower()
        if text.endswith('d'):
            return timedelta(days=float(text[:-1]))
        if text.endswith('h'):
            return timedelta(hours=float(text[:-1]))
    if isinstance(windows, dict):
        if windows.get('days') is not None:
            return timedelta(days=float(windows['days']))
        if windows.get('hours') is not None:
            return timedelta(hours=float(windows['hours']))
    # No default. codex ran out on 2026-09-24 and came back three days later, so a guessed
    # span sums the wrong runs and writes a budget far too small. Map 76-H.
    raise ValueError(f'no window for {provider} in vendors.json; set its "windows" ("5h", "7d") '
                     'or pass --since')


def corpus_records(runs, home, hydra=None):
    """Load run metadata from a corpus; cache hydra show answers in home."""
    runs, home = Path(runs), Path(home)
    hydra = hydra or os.environ.get('HYDRA_BIN', 'hydra')
    if not runs.is_dir():
        raise ValueError(f'runs directory does not exist: {runs}')
    cache = read_json(home / 'sheet-cache.json', {})
    records = {}
    for path in sorted(runs.glob('*.jsonl')):
        data = path.read_bytes()
        lines = []
        for line in data.splitlines(keepends=True):
            if not line.endswith(b'\n'):
                break  # A writer has not committed this line yet.
            lines.append(json.loads(line))
        if not lines:
            continue
        first = lines[0]
        start = first.get('hydra', {})
        if not first.get('provider') or start.get('command') not in ('run', 'resume'):
            continue
        run_id = first['run_id']
        fingerprint = hashlib.sha256(data).hexdigest()
        entry = cache.get(run_id, {})
        if entry.get('fingerprint') != fingerprint:
            env = dict(os.environ, HYDRA_ROOT=str(runs.resolve().parent))
            # Hydra resolves records only in <HYDRA_ROOT>/runs. Custom directory
            # names use a temporary layout with a symlink to the supplied corpus.
            with tempfile.TemporaryDirectory() as tmp:
                if runs.name != 'runs':
                    Path(tmp, 'runs').symlink_to(runs.resolve(), target_is_directory=True)
                    env['HYDRA_ROOT'] = tmp
                result = subprocess.run([hydra, 'show', run_id, '--events'], env=env,
                                        capture_output=True, text=True, check=True)
            entry = {'fingerprint': fingerprint, 'answer': json.loads(result.stdout)}
            cache[run_id] = entry
        answer = entry['answer']
        projection = answer.get('projection') or {}
        usage = projection.get('usage') or {}
        end = next((v['hydra'] for v in reversed(lines) if v.get('hydra', {}).get('type') == 'run_end'), {})
        duration = None
        ended_at = end.get('ended_at')
        if ended_at:
            duration = (parse_iso(ended_at) - parse_iso(first['ts'])).total_seconds()
        rates = [utilization(v.get('raw')) for v in lines if isinstance(v.get('raw'), dict) and v['raw'].get('type') == 'rate_limit_event']
        rates = [v for v in rates if v is not None]
        policy = start.get('policy')
        records[run_id] = dict(run_id=run_id, provider=first['provider'], model=first.get('model'),
            parent_run_id=first.get('parent_run_id') or start.get('resumed_from'), ts=first['ts'],
            ended_at=ended_at, policy=policy, subscription=start.get('subscription'),
            provider_version=start.get('provider_version'), launch_mode=start.get('launch_mode'),
            duration_seconds=duration, outcome=projection.get('outcome'),
            usage_source=usage.get('usage_source', 'unavailable'), tokens=usage.get('counts') or {},
            denials=sum(e.get('type') == 'permission_decision' and e.get('decision') == 'deny'
                        for e in answer.get('events', [])),
            judge_parsed=judge_parsed(projection.get('result_text')) if policy == 'judge' else None,
            window_share=(rates[-1] - rates[0]) if len(rates) >= 2 and rates[-1] >= rates[0] else None)
    write_json(home / 'sheet-cache.json', cache)
    return records


def revise_budget(run_id, runs, vendors_path, hydra=None, home=None, cache_read_weight=1.0, aliases=None,
                  since=None):
    """Set window_budget_tokens from a rate_limited run's closed window; return a summary line."""
    runs = Path(runs)
    vendors_path = Path(vendors_path)
    home = Path(home) if home is not None else vendors_path.parent
    hydra = hydra or os.environ.get('HYDRA_BIN', 'hydra')
    if not math.isfinite(cache_read_weight) or cache_read_weight < 0:
        raise ValueError('cache_read_weight must be finite and nonnegative')
    records = corpus_records(runs, home, hydra)
    if run_id not in records:
        raise ValueError(f'no such run in corpus: {run_id}')
    trigger = records[run_id]
    if trigger.get('outcome') != 'rate_limited':
        raise ValueError(f'run {run_id} is not rate_limited')
    if trigger['provider'] == 'claude':
        raise ValueError('claude window budgets come from utilization, not token spend')
    if not trigger.get('ended_at'):
        raise ValueError(f'run {run_id} has no run_end')
    aliases = load_aliases() if aliases is None else aliases
    key = canonical(trigger['provider'], trigger.get('model'), aliases)
    vendors = read_json(vendors_path)
    vendor = next((v for v in vendors
                   if canonical(v['provider'], v.get('model'), aliases) == key), None)
    if vendor is None:
        raise ValueError(f'no vendor row for {key[0]!r} model {key[1]!r}')
    window_end = parse_iso(trigger['ended_at'])
    if since is not None:
        window_start = parse_iso(since)
    else:
        window_start = window_end - window_duration(vendor, trigger['provider'])
    run_ids = []
    total = 0
    for record in records.values():
        if canonical(record['provider'], record.get('model'), aliases) != key:
            continue
        if not record.get('ended_at'):
            continue
        started = parse_iso(record['ts'])
        ended = parse_iso(record['ended_at'])
        if ended < window_start or started > window_end:
            continue
        spend = token_spend(record['tokens'], cache_read_weight)
        if spend is None:
            continue
        run_ids.append(record['run_id'])
        total += spend
    run_ids.sort()
    if not run_ids:
        raise ValueError(f'no token spend in window for {run_id}')
    history = vendor.setdefault('budget_history', [])
    previous = vendor.get('window_budget_tokens')
    if (previous == total and history and history[-1].get('new') == total
            and sorted(history[-1].get('run_ids', [])) == run_ids):
        return None
    entry = dict(previous=previous, new=total, date=datetime.now(tz=window_end.tzinfo).date().isoformat(),
                 run_ids=run_ids, outside_spend_possible=True)
    history.append(entry)
    vendor['window_budget_tokens'] = total
    write_json(vendors_path, vendors)
    return (f"{trigger['provider']} window_budget_tokens: {previous} -> {total} "
            f"(runs: {', '.join(run_ids)}; outside_spend_possible: true)")


def build(runs, home, vendors=None, hydra=None, cache_read_weight=1.0, aliases=None):
    """Build sheets; cache by run id and content hash so live records refresh."""
    runs, home = Path(runs), Path(home)
    if not math.isfinite(cache_read_weight) or cache_read_weight < 0:
        raise ValueError('cache_read_weight must be finite and nonnegative')
    vendors = read_json(HERE / 'vendors.json') if vendors is None else vendors
    aliases = load_aliases() if aliases is None else aliases
    records = corpus_records(runs, home, hydra)
    groups = {}
    recorded_models = {}
    for v in vendors:
        key = canonical(v['provider'], v.get('model'), aliases)
        groups.setdefault(key, [])
        recorded_models.setdefault(key, set())
    for record in records.values():
        root, seen = record, {record['run_id']}
        while root.get('parent_run_id') in records and root['parent_run_id'] not in seen:
            seen.add(root['parent_run_id'])
            root = records[root['parent_run_id']]
        record['task_id'] = root.get('parent_run_id') or root['run_id']
        key = canonical(record['provider'], record.get('model'), aliases)
        groups.setdefault(key, []).append(record)
        recorded_models.setdefault(key, set()).add(record.get('model'))
    sheets = []
    for (provider, model), rows in sorted(groups.items(), key=lambda pair: (pair[0][0], pair[0][1] or '')):
        vendor = next((v for v in vendors
                       if canonical(v['provider'], v.get('model'), aliases) == (provider, model)), {})
        facts = {k: vendor.get(k) for k in ('as_of', 'source_url', 'context_tokens', 'thinking_levels', 'plan', 'windows', 'window_budget_tokens')}
        budget = facts['window_budget_tokens']
        for row in rows:
            if provider != 'claude':
                tokens = row['tokens']
                row['window_share'] = ((tokens['raw_input'] * cache_read_weight + tokens['output_tokens']) / budget
                    if budget is not None and budget > 0 and 'raw_input' in tokens and 'output_tokens' in tokens else None)
        spellings = sorted(recorded_models.get((provider, model), ()), key=lambda m: m or '')
        sheet = dict(provider=provider, model=model, recorded_models=spellings, vendor=facts,
            available=vendor.get('available', True),
            policies=vendor.get('policies', [p for p in POLICIES if not (provider == 'codex' and p == 'read-only')]),
            cache_read_weight=cache_read_weight, runs=rows, sample_size=len(rows),
            independent_tasks=len({r['task_id'] for r in rows}),
            duration_seconds=statistic([r['duration_seconds'] for r in rows]),
            cost=statistic([r['window_share'] for r in rows]),
            denials=statistic([r['denials'] for r in rows]),
            tokens={k: statistic([r['tokens'].get(k) for r in rows]) for k in ('raw_input', 'output_tokens', 'cache_read_tokens')},
            usage_sources=dict(Counter(r['usage_source'] for r in rows)),
            outcomes=dict(Counter(r['outcome'] or 'unknown' for r in rows)),
            judge={'n': sum(r['judge_parsed'] is not None for r in rows), 'parsed': sum(r['judge_parsed'] is True for r in rows)})
        write_json(home / 'agents' / filename(provider, model), sheet)
        sheets.append(sheet)
    # Only delete stale sheets owned by this builder.
    names = {filename(s['provider'], s['model']) for s in sheets}
    for path in (home / 'agents').glob('*.json'):
        if path.name not in names:
            path.unlink()
    return sheets


def beta_lower(a, b, probability=0.1):
    """10th percentile of integer-parameter Beta via its binomial identity."""
    n = a + b - 1
    def cdf(x):
        return sum(math.exp(math.lgamma(n + 1) - math.lgamma(j + 1) - math.lgamma(n - j + 1)
                            + j * math.log(x) + (n - j) * math.log1p(-x)) for j in range(a, n + 1))
    lo, hi = 0.0, 1.0
    for _ in range(60):
        mid = (lo + hi) / 2
        if cdf(mid) < probability:
            lo = mid
        else:
            hi = mid
    return (lo + hi) / 2


def rank(sheet, task_class, grade, verdicts=None, *, policy=None, priors=None):
    """Rank one sheet or a list; optional verdicts are {task, run_id, result}."""
    if grade == 'agi':
        return []
    threshold = {'low': .5, 'medium': .6, 'high': .7}[grade]
    priors = read_json(HERE / 'priors.json') if priors is None else priors
    policy = policy or ('judge' if task_class in ('probe', 'security_review') else 'standard')
    sheets = [sheet] if isinstance(sheet, dict) else sheet
    result = []
    verdicts = list(verdicts or [])
    task_classes = {e['id']: e['class'] for e in verdicts
                    if e.get('event', e.get('type')) == 'task' and 'id' in e and 'class' in e}
    for agent in sheets:
        if not agent.get('available', False) or policy not in agent.get('policies', []):
            continue
        name = (agent.get('model') or '').lower().replace(' ', '-')
        prior = priors['default']
        for rule in priors['rules']:
            if agent['provider'] == rule['provider'] and rule.get('model_contains', '') in name:
                prior = rule.get('classes', {}).get(task_class, rule['prior'])
                break
        a, b = prior
        counts = Counter()
        runs = {r['run_id']: r for r in agent.get('runs', [])}
        seen = set()
        for event in verdicts:
            if event.get('event', event.get('type', 'human_verdict')) != 'human_verdict':
                continue
            run = runs.get(event.get('run_id'))
            if run is None or event.get('result') not in ('applied', 'fixed', 'rejected'):
                continue
            if event.get('task_class', task_classes.get(event.get('task'), task_class)) != task_class:
                continue
            task = str(event.get('task') or run['task_id'])
            if task in seen or run['task_id'] in seen:
                continue
            seen.update((task, run['task_id']))
            # A resumed acceptance required another attempt, so is not first-pass.
            verdict = 'fixed' if run.get('parent_run_id') and event['result'] == 'applied' else event['result']
            counts[verdict] += 1
        bound = beta_lower(a + counts['applied'], b + counts['fixed'] + counts['rejected'])
        if bound < threshold:
            continue
        cost = agent['cost']['p50']
        result.append(dict(provider=agent['provider'], model=agent.get('model'), bound=bound,
            cost=cost, cost_sample_size=agent['cost']['n'], sample_size=sum(counts.values()),
            score=cost / bound if cost is not None else None, counts=dict(counts)))
    return sorted(result, key=lambda r: (r['score'] is None, r['score'] if r['score'] is not None else 0, -r['bound'], r['provider'], r['model'] or ''))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest='command', required=True)
    default_root = Path(os.environ.get('HYDRA_ROOT', Path.home() / 'hydra'))
    default_home = Path(os.environ.get('HYDRA_PUPPETS_HOME', default_root / 'puppets'))
    for command in ('build', 'budget', 'rank', 'show'):
        p = commands.add_parser(command)
        p.add_argument('--home', type=Path, default=default_home)
        if command == 'build':
            p.add_argument('--runs', type=Path, default=default_root / 'runs')
            p.add_argument('--cache-read-weight', type=float, default=1.0)
        elif command == 'budget':
            p.add_argument('run_id')
            p.add_argument('--runs', type=Path, default=default_root / 'runs')
            p.add_argument('--vendors', type=Path, default=HERE / 'vendors.json')
            p.add_argument('--since', help='window start, ISO 8601, when vendors.json names no window')
            p.add_argument('--cache-read-weight', type=float, default=1.0)
        elif command == 'rank':
            p.add_argument('--class', dest='task_class', required=True)
            p.add_argument('--grade', choices=['low', 'medium', 'high', 'agi'], required=True)
            p.add_argument('--provider')
            p.add_argument('--policy', choices=POLICIES)
            p.add_argument('--verdicts', type=Path, help='Goal log JSONL')
        else:
            p.add_argument('provider')
            p.add_argument('model', nargs='?')
    args = parser.parse_args()
    if args.command == 'build':
        if not math.isfinite(args.cache_read_weight) or args.cache_read_weight < 0:
            parser.error('cache-read-weight must be finite and nonnegative')
        sheets = build(args.runs, args.home, cache_read_weight=args.cache_read_weight)
        print(json.dumps({'agents': len(sheets)}))
    elif args.command == 'budget':
        if not math.isfinite(args.cache_read_weight) or args.cache_read_weight < 0:
            parser.error('cache-read-weight must be finite and nonnegative')
        line = revise_budget(args.run_id, args.runs, args.vendors, home=args.home,
                             cache_read_weight=args.cache_read_weight, since=args.since)
        if line:
            print(line)
    elif args.command == 'show':
        path = args.home / 'agents' / filename(args.provider, args.model)
        if not path.exists():
            parser.error('no sheet for that provider/model; build first')
        print(path.read_text(), end='')
    else:
        sheets = [read_json(p) for p in sorted((args.home / 'agents').glob('*.json'))]
        if args.provider:
            sheets = [s for s in sheets if s['provider'] == args.provider]
        verdicts = [json.loads(line) for line in args.verdicts.read_text().splitlines() if line.strip()] if args.verdicts else []
        print(json.dumps(rank(sheets, args.task_class, args.grade, verdicts, policy=args.policy), indent=2))


if __name__ == '__main__':
    main()
