"""Record triage commands with an empty environment and scratch files, CPython 3.14.7."""
import importlib.util
import json
import subprocess
import sys
import tempfile
from pathlib import Path
HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
SCRIPT = REPO / 'integrations/grader/triage.py'

def dump(rows):
    return ''.join(json.dumps(r) + '\n' for r in rows)

def case(name, argv, seed):
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        for path, text in seed.items():
            target = root / path
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(text)
        env = dict(HYDRA_ROOT='root', HYDRA_GRADER_HOME='home', PYTHONDONTWRITEBYTECODE='1')
        ran = subprocess.run([sys.executable, str(SCRIPT), *argv], cwd=root, env=env, stdin=subprocess.DEVNULL, capture_output=True)
        left = {str(p.relative_to(root)): p.read_text() for p in sorted(root.rglob('*')) if p.is_file()}
        return dict(name=name, argv=argv, seed=seed, env=env, code=ran.returncode, stdout=ran.stdout.decode(), stderr=ran.stderr.decode(), left=left)

def record(out, repo, scans):
    spec = importlib.util.spec_from_file_location('triage_tests', repo / 'integrations/grader/tests/test_triage.py')
    tests = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(tests)
    cases = []
    combined = {}
    rows = []
    # Harvest the actual test shapes rather than translating them by hand.
    for name in sorted(n for n in dir(tests.TriageTests) if n.startswith('test_')):
        test = tests.TriageTests(name)
        test.setUp()
        getattr(test, name)()
        seed = {('root/runs/' + p.name): p.read_text() for p in test.runs.iterdir()}
        seed['home/dataset.jsonl'] = (test.home / 'dataset.jsonl').read_text()
        cases.append(case(name, ['run'], seed))
        ident = name
        combined['root/runs/' + ident + '.jsonl'] = seed['root/runs/r1.jsonl']
        row = json.loads(seed['home/dataset.jsonl'])
        rows.append(dict(row, run_id=ident, provider='codex' if len(rows) % 2 else 'claude'))
        test.doCleanups()
    for i, (name, data) in enumerate(scans[:12]):
        ident = 'fixture-' + str(i)
        seed = {'root/runs/' + ident + '.jsonl': data.decode(), 'home/dataset.jsonl': dump([dict(run_id=ident, outcome='failed', provider='cursor')])}
        cases.append(case(name, ['run'], seed))
        combined.update({k:v for k,v in seed.items() if k.startswith('root/')})
        rows.append(dict(run_id=ident, outcome='failed', provider='cursor'))
    # Rule precedence, empty/missing records, time boundaries, torn tails and long evidence.
    extra = [
        ('empty', [], 'failed'), ('missing', None, 'failed'),
        ('completed', [], 'completed'), ('fallback-collision', [], 'skill_collision'),
        ('fallback-quota', [], 'rate_limited'),
        ('collision-before-quota', [{'raw': {'api_error_status':429}}, {'hydra':{'type':'run_end','refusal':'skill_collision'}}], 'failed'),
        ('signal-code', [{'raw':{}}, {'hydra':{'type':'run_end','exit_code':128}}], 'failed'),
        ('exact-600', [{'raw':{},'ts':'2026-09-24T00:00:00Z'}, {'hydra':{'type':'run_end'},'ts':'2026-09-24T00:10:00Z'}], 'failed'),
        ('microsecond-over', [{'raw':{},'ts':'2026-09-24T00:00:00Z'}, {'hydra':{'type':'run_end'},'ts':'2026-09-24T00:10:00.000001Z'}], 'failed'),
        ('long-evidence', [{'hydra':{'type':'run_end','note':'é😀'*300}}], 'failed'),
        ('mixed-zones', [{'raw':{},'ts':'2026-09-24T00:00:00'}, {'hydra':{'type':'run_end'},'ts':'2026-09-24T00:10:01Z'}], 'failed'),
    ]
    for name, lines, outcome in extra:
        seed = {'home/dataset.jsonl':dump([dict(run_id=name, outcome=outcome)])}
        if lines is not None:
            seed['root/runs/'+name+'.jsonl'] = dump(lines)
        cases.append(case(name, ['run'], seed))
    combined['home/dataset.jsonl'] = dump(rows)
    run = case('combined', ['run'], combined)
    cases.append(run)
    seed = run['left']
    cases.append(case('table', ['table'], seed))
    for s in ['s', 'x']:
        for n in [0, 1, 5, 100, -1]:
            cases.append(case('sample-'+s+'-'+str(n), ['sample','--seed',s,'--n',str(n)], seed))
    for argv in [[], ['bad'], ['run','extra'], ['table','--n','1'], ['sample','--n'], ['sample','--n','bad'], ['sample','--n=1_0'], ['sample','--n','０'], ['sample','--s','x'], ['sample','--help'], ['--help'], ['sample','--seed'], ['sample','--help=x'], ['sample','--n','-1'], ['table'], ['sample']]:
        cases.append(case('argv-'+repr(argv), argv, {}))
    cases.extend(extra_cases())
    (out/'triage.json').write_text(json.dumps(dict(version=sys.version, cases=cases), indent=1)+'\n')
    print('triage:', len(cases), 'cases', file=sys.stderr)


def extra_cases():
    cases=[]
    def run(name,lines=None,row=None,record=None):
        seed={'home/dataset.jsonl': dump([row if row is not None else {'run_id':'r','outcome':'failed'}])}
        if lines is not None: seed['root/runs/r.jsonl']=dump(lines)
        if record is not None: seed['root/runs/r.jsonl']=record
        cases.append(case(name,['run'],seed))
    end={'hydra':{'type':'run_end'}}
    run('torn-tail',record=dump([{'raw':{}},end])+'{"raw":')
    run('garbled-interior',record='not json\n'+dump([end]))
    run('non-object-frame',[[],end])
    run('non-object-dataset',[],row=[])
    run('missing-run-id',[],row={'outcome':'failed'})
    run('outcome-list',[{'raw':{}},end],row={'run_id':'r','outcome':['failed']})
    run('invalid-time',[{'raw':{},'ts':'bad'},dict(end,ts=7)])
    run('naive-stall',[{'raw':{},'ts':'2026-09-24T00:00:00'},dict(end,ts='2026-09-24T00:10:01')])
    run('offset-gap',[{'raw':{},'ts':'2026-09-24T00:00:00+00:01'},dict(end,ts='2026-09-24T00:09:01Z')])
    run('truthy-refusal',[{'hydra':{'type':'run_end','refusal':['denied']}}])
    run('float-signal-code',[{'raw':{}},{'hydra':{'type':'run_end','exit_code':129.0}}])
    run('huge-signal-code',[{'raw':{}},{'hydra':{'type':'run_end','exit_code':10**50}}])
    run('last-run-end',[{'raw':{}},{'hydra':{'type':'run_end','refusal':'skill_collision'}},end])
    run('quota-before-gate',[{'raw':{'api_error_status':429}},{'hydra':{'type':'run_end','refusal':'policy'}}])
    # A gated outcome keeps an earlier refusal even after a later success.
    a={'raw':{'type':'assistant','message':{'content':[{'type':'tool_use','id':'a','name':'Bash'},{'type':'tool_use','id':'b','name':'Read'}]}}}
    b={'raw':{'type':'user','message':{'content':[{'type':'tool_result','tool_use_id':'a','is_error':True,'content':'hydra/capability: no'}]}}}
    c={'raw':{'type':'user','message':{'content':[{'type':'tool_result','tool_use_id':'b','content':'ok'}]}}}
    run('gated-after-success',[a,b,c,end],row={'run_id':'r','outcome':'gated'})
    run('new-call-after-refusal',[a,b,a,end])
    # Distinct keys with equal string forms and equal numeric keys with distinct string forms.
    rows=[dict(run_id=str(i), outcome=o,cause=c,evidence='é\nline') for i,(o,c) in enumerate([(None,None),('None','None'),(True,1),(1,True),(1.0,1.0),('failed','quota')])]
    dataset=[dict(run_id=str(i),provider=p) for i,p in enumerate([None,'None',True,1,1.0,'é'])]
    seed={'home/triage.jsonl':dump(rows),'home/dataset.jsonl':dump(dataset)}
    cases.append(case('table-key-equality',['table'],seed))
    cases.append(case('table-unhashable',['table'],{'home/triage.jsonl':dump([dict(run_id='r',cause=[])]),'home/dataset.jsonl':''}))
    cases.append(case('table-no-dataset',['table'],{'home/triage.jsonl':dump([dict(run_id='r',outcome=None,cause=None)])}))
    for argv in [['sample','--seed','s'],['sample','--n',str(10**50)],['sample','--n','-'+str(10**50)],['sample','--n','+2'],['sample','--n',' ２ '],['--foo','table'],['--','table'],['sample','--n','--help'],['sample','--seed','--'],['sample','--','--n','1']]:
        cases.append(case('extra-argv-'+repr(argv),argv,{'home/triage.jsonl':dump(rows)}))
    return cases
