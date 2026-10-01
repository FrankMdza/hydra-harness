"""Record build, patches and score. Run with python3; no network command runs."""
import json
import sys
from record_queue import HERE, case


def record():
    cases=[]
    for name, rows in [
        ('float-underscores', [('changed','0_0.8')]),
        ('float-unicode', [('changed','０.２')]),
        ('brier-overflow', [('changed',1e200)]),
        ('normal', [('changed',.8),('as_written',.2),('changed',.6)]),
        ('nan', [('changed',float('nan')),('as_written',.3)]),
        ('ties', [('changed',.5),('as_written',.5)]),
        ('near-ties', [('changed',.5),('as_written',.5000000000000001)]),
        ('empty', []),('single-class',[('changed',1.0)]),
        ('boundaries', [('changed',0),('as_written',.2),('changed',.4),('as_written',.6),('changed',.8),('as_written',1)]),
    ]:
        text=''.join(json.dumps(dict(label=l,answers={'needs_correction':p}))+'\n' for l,p in rows)
        cases.append(case('jev_probe',name,['score','answers.jsonl'],{'answers.jsonl':text}))
    cases.append(case('jev_probe','no-answers',['score','answers.jsonl'],{'answers.jsonl':'{"label":"changed"}\n{"label":"as_written","answers":{}}\n{"label":"other","answers":{"needs_correction":0.5}}\n'}))
    for name, labels in [('patches-unicode',[{'ticket':1,'label':'changed','patch':'a.patch'},{'ticket':2,'label':'as_written','patch':'b.patch'}]),('patches-empty',[]),('patches-unknown',[{'ticket':1,'label':'unknown','patch':'a.patch'}])]:
        cases.append(case('jev_probe',name,['patches','labels.json','out.jsonl'],{'a.patch':'diff\n+café 😀\r\n','b.patch':'diff\n+two\n','labels.json':json.dumps(labels)}))
    seed={'src/a.rs':'fn a() {}\n'}
    bodies={
        200:'Status: resolved\n## Outcome\nrun 01234567-abcd session changed the diff.',
        201:'Status: resolved\n## Outcome\nrun 01234567-abcd applied as written.',
        202:'Status: open\n',203:'no status',204:'Status: resolved\nno outcome',
        205:'Status: resolved\n## Outcome\nrun 01234567-abcd session fixed the diff.',
        206:'Status: resolved\n## Outcome\nrun 01234567-abcd applied the diff as written.',
        111:'Status: resolved\n## Outcome\nrun 01234567-abcd session changed the diff.',
    }
    for n,body in bodies.items(): seed[f'.scratch/hydra/build/{n}-test.md']=body
    steps=[['init','-b','main'],['add','.'],['commit','-m','seed'],{'path':'src/a.rs','text':'fn a() { /* café */ }\n'},['add','.'],['commit','-m','Build 200: change'],{'path':'src/b.rs','text':'fn b() {}\n'},['add','.'],['commit','-m','Build 201: change'],{'path':'other','text':'ignored\n'},['add','.'],['commit','-m','Build 206: ignored']]
    cases.append(case('jev_probe','build-all-skips',['build','out.jsonl'],seed,steps))
    for i, outcome in enumerate([
        'run 01234567-abcd session changed it',
        'RUN 01234567-ABCD session fixed it',
        'run\t01234567-abcd session\nreworded it',
        'run 01234567-abcd added something to the diff',
        'run 01234567-abcd applied the diff as written',
        'run 01234567-abcd (applied as written).',
        'run 01234567-abcd misapplied as written',
        'run 01234567-abcd applied as writtenx',
        'outrun 01234567-abcd session changed it',
        'run 01234567-abc session changed it',
        'run "01234567-abcd" session changed it',
        'run 01234567-abcd session changedness',
        'run 01234567-abcd obsession changed it',
        'run 01234567-abcd added\x1c something\tto the\ndiff',
        'run 01234567-abcd applied as written; session fixed it',
        'run 01234567-abcd session changed it\n## Outcome\nignored',
        'run 01234567-abcd applied as written\n## Notes\nsession changed it',
    ]):
        source = {'src/a.rs':'before\n','.scratch/hydra/build/200-one.md':'Status: resolved\n## Outcome\n'+outcome}
        git = [['init','-b','main'],['add','.'],['commit','-m','seed'],{'path':'src/a.rs','text':'after\n'},['add','.'],['commit','-m','Build 200: change']]
        cases.append(case('jev_probe',f'outcome-{i}',['build','out.jsonl'],source,git))
    for i,args in enumerate([[],['wrong'],['build'],['patches'],['patches','labels'],['score'],['build','out','extra'],['score','x','--bad'],['score','--help=bad'],['--bad','score','x'],['build','--help'],['patches','--help'],['score','--help']]):
        cases.append(case('jev_probe',f'argv-{i}',args,{}))
    return cases


if __name__=='__main__':
    (HERE/'fixtures/jev.json').write_text(json.dumps(dict(version=sys.version,cases=record()),indent=2)+'\n')
