"""Record queue command lines under CPython 3.14, with an empty child environment.

Run: python3 crates/grader/tests/record_queue.py
The checkout's path in a traceback is written as `<repo>` (build 220), so a recording made
in another checkout gives the same bytes. The Rust replay reads only the exception's class.
"""
import csv
import io
import json
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
CHILD = '''import importlib.util,sys
script=sys.argv[1]
sys.argv=[script.rsplit('/',1)[-1],*sys.argv[2:]]
spec=importlib.util.spec_from_file_location('reference',script)
m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
raise SystemExit(m.main())
'''


def case(module, name, argv, seed, git=None):
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp).resolve()
        for path, text in seed.items():
            target = root / path
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(text.encode())
        env = {'HYDRA_ROOT': 'unused-root', 'HYDRA_GRADER_HOME': 'home'}
        if git:
            env.update(GIT_ENV)
            for step in git:
                if isinstance(step, dict):
                    (root / step['path']).write_text(step['text'])
                else:
                    subprocess.run([GIT, *step], cwd=root, env=env, stdin=subprocess.DEVNULL, capture_output=True, check=True)
        ran = subprocess.run([sys.executable, '-c', CHILD, str(REPO / 'integrations/grader' / (module + '.py')), *argv], cwd=root, env=env, stdin=subprocess.DEVNULL, capture_output=True)
        left = {str(p.relative_to(root)): p.read_bytes().decode() for p in sorted(root.rglob('*')) if p.is_file() and '.git' not in p.parts}
        return dict(name=name, argv=argv, seed=seed, git=git or [], env=env, code=ran.returncode, stdout=ran.stdout.decode(), stderr=ran.stderr.decode().replace(str(REPO), '<repo>'), left=left)


GIT = '/usr/bin/git'
GIT_ENV = dict(GIT_AUTHOR_DATE='2026-10-01T00:00:00Z', GIT_COMMITTER_DATE='2026-10-01T00:00:00Z', GIT_AUTHOR_NAME='test', GIT_AUTHOR_EMAIL='test@example.com', GIT_COMMITTER_NAME='test', GIT_COMMITTER_EMAIL='test@example.com', GIT_CONFIG_NOSYSTEM='1', GIT_CONFIG_GLOBAL='/dev/null')
ID = '01234567-abcd-7000-8000-000000000001'
DATA = json.dumps(dict(run_id=ID, provider='codex', model=None, human_verdict=None))+'\n'


def record():
    cases = []
    for i, value in enumerate(['comma,field', 'quote"field', 'CR\rfield', 'LF\nfield', 'CRLF\r\nfield', 'café 😀', '']):
        data = json.dumps(dict(run_id=ID, provider=value, model=value, human_verdict=None))+'\n'
        body = 'Status: resolved\n## Outcome\nRun '+ID+' '+value
        cases.append(case('baseline', f'csv-{i}', ['label-queue', 'queue.csv', '--tickets', 'build', '--dataset', 'dataset.jsonl'], {'dataset.jsonl':data, 'build/200-one.md':body}))
    for name, body in [
        ('no-outcome', 'Status: resolved\nRun '+ID),
        ('two-outcomes', 'Status: resolved\n## Outcome\nRun '+ID+'\n## Outcome\nother 12345678'),
        ('outcome-at-end', 'Status: resolved\n## Outcome'),
        ('ambiguous', 'Status: resolved\n## Outcome\n01234567 ffffffff'),
        ('duplicate-mentions', 'Status: resolved\n## Outcome\n'+ID+' '+ID),
        ('not-resolved', 'Status: open\n## Outcome\n'+ID),
        ('unicode-boundary', 'Status: resolved\n## Outcome\né01234567 _01234567 01234567z 01234567'),
        ('long-outcome', 'Status: resolved\n## Outcome\n'+ID+' é'*500),
    ]:
        data = DATA + (DATA.replace(ID, ID[:-1]+'2') if name=='ambiguous' else '')
        cases.append(case('baseline', name, ['label-queue','queue.csv','--tickets=build','--dat=dataset.jsonl'], {'dataset.jsonl':data, 'build/200-one.md':body}))
    for name, text in [
        ('blank-header', '\nrun_id,verdict\nx,fixed\n'),
        ('missing-run-cell', 'verdict,run_id\nfixed\n'),
        ('field-limit', 'run_id,verdict\n'+'a'*131073+',fixed\n'),
        ('missing-verdict', 'run_id\r\n'+ID+'\r\n'),
        ('missing-run', 'verdict\r\nfixed\r\n'),
        ('missing-run-refusal', 'verdict\r\nBAD\r\n'),
        ('invalid-verdict', 'run_id,verdict\r\n'+ID+',good\r\n'),
        ('unknown', 'run_id,verdict\r\n'+ID+',unknown\r\n'),
        ('duplicate', 'run_id,verdict\r\n'+ID+',applied\r\n'+ID+',fixed\r\n'),
        ('not-in-dataset', 'run_id,verdict\r\nother,fixed\r\n'),
        ('empty-file', ''),
        ('short-row', 'run_id,verdict\r\n'+ID+'\r\n'),
        ('extra-column', 'run_id,verdict\r\n'+ID+',fixed,extra\r\n'),
        ('strip-lower', 'run_id,verdict\r\n'+ID+', FIXED \r\n'),
    ]:
        cases.append(case('baseline',name,['label-import','queue.csv','--dataset','dataset.jsonl','--home','home'],{'queue.csv':text,'dataset.jsonl':(json.dumps(dict(run_id='x',human_verdict=None))+'\n' if name=='blank-header' else DATA)}))
    for i, field in enumerate(['a,b','a"b','a\rb','a\nb','a\r\nb','é😀','']):
        buf=io.StringIO(newline=''); writer=csv.writer(buf);writer.writerow(['run_id','verdict','note']);writer.writerow([field,'fixed',field])
        cases.append(case('baseline',f'import-csv-{i}',['label-import','queue.csv','--dataset','dataset.jsonl'],{'queue.csv':buf.getvalue(),'dataset.jsonl':DATA}))
    for i, args in enumerate([['label-queue'],['label-import'],['label-queue','q','--tickets'],['label-import','q','--dataset'],['label-queue','q','--bad'],['label-import','q','extra'],['label-queue','q','--h'],['label-import','q','--h'],['label-queue','--','-q','--dataset','data'],['label-import','--help=bad'],['label-queue','--help'],['label-import','--help']]):
        cases.append(case('baseline',f'argv-{i}',args,{}))
    return cases


if __name__ == '__main__':
    (HERE/'fixtures/queue.json').write_text(json.dumps(dict(version=sys.version, cases=record()), ensure_ascii=True, indent=2)+'\n')
