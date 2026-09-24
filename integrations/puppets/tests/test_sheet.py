import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

MODULE = Path(__file__).resolve().parents[1] / 'sheet.py'
spec = importlib.util.spec_from_file_location('sheet', MODULE)
sheet = importlib.util.module_from_spec(spec)
spec.loader.exec_module(sheet)


class SheetTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.runs = self.root / 'synthetic'
        self.runs.mkdir()
        self.home = self.root / 'home'
        self.table = {}
        self.stub = self.root / 'hydra'
        self.stub.write_text('#!' + sys.executable + '\n' + '''import json, os, pathlib, sys
root = pathlib.Path(__file__).parent
assert sys.argv[1] == 'show' and sys.argv[3] == '--events'
assert (pathlib.Path(os.environ['HYDRA_ROOT']) / 'runs' / (sys.argv[2] + '.jsonl')).exists()
with (root / 'calls').open('a') as f: f.write(sys.argv[2] + '\\n')
print(json.dumps(json.loads((root / 'table.json').read_text())[sys.argv[2]]))
''')
        self.stub.chmod(0o755)
        self.vendors = [dict(provider='claude', model='sonnet', window_budget_tokens=None),
                        dict(provider='codex', model=None, window_budget_tokens=None)]

    def record(self, ident, provider='claude', model='sonnet', parent=None,
               command='run', policy='standard', ended=True, rates=(), outcome='completed', text=None):
        envelope = dict(run_id=ident, parent_run_id=parent, provider=provider, model=model,
                        ts='2026-09-24T00:00:00Z')
        rows = [dict(envelope, hydra=dict(type='run_start', command=command, policy=policy,
                subscription='test', provider_version='1', launch_mode='recorded'))]
        for rate in rates:
            rows.append(dict(envelope, raw={'type':'rate_limit_event', 'rate_limit_info':{
                'unifiedWindows':{'seven_day':{'utilization':rate}}}}))
        if ended:
            rows.append(dict(envelope, hydra=dict(type='run_end', ended_at='2026-09-24T00:00:10Z', exit_code=0)))
        (self.runs / (ident + '.jsonl')).write_text(''.join(json.dumps(v)+'\n' for v in rows))
        self.table[ident] = {'projection':{'outcome':outcome if ended else None,
             'result_text':text, 'usage':{'usage_source':'provider', 'counts':{
             'raw_input':100, 'cache_read_tokens':80, 'output_tokens':20}}},
             'events':[{'type':'permission_decision', 'decision':'deny'}]}
        (self.root / 'table.json').write_text(json.dumps(self.table))

    def build(self):
        return sheet.build(self.runs, self.home, self.vendors, str(self.stub))

    def test_aliases_merge_spellings(self):
        self.record('a', model='sonnet')
        self.record('b', model='Sonnet 5')
        self.record('c', model='claude-sonnet-5')
        sheets = self.build()
        claude = [s for s in sheets if s['provider'] == 'claude' and s['model'] == 'sonnet']
        self.assertEqual(len(claude), 1)
        self.assertEqual(claude[0]['sample_size'], 3)
        self.assertEqual(claude[0]['recorded_models'], ['Sonnet 5', 'claude-sonnet-5', 'sonnet'])

    def test_corpus_cache_and_rollup(self):
        self.record('a', rates=(.1,.3), outcome='gated')
        self.record('b', provider='codex', model=None, policy='judge', text='```json\n{"verdict":"pass","findings":[]}\n```')
        self.record('c', parent='a', command='resume', rates=(.3,.4))
        self.record('d', ended=False)
        self.record('excluded', provider=None, command='evolve')
        self.record('curator', command='curate')
        sheets = self.build()
        claude, codex = sheets
        self.assertEqual(claude['sample_size'], 3)
        self.assertEqual(claude['independent_tasks'], 2)
        self.assertEqual(claude['outcomes'], {'gated':1,'completed':1,'unknown':1})
        self.assertEqual(claude['duration_seconds'], {'p50':10,'n':2})
        self.assertAlmostEqual(claude['cost']['p50'], .15)
        self.assertEqual(claude['cost']['n'], 2)
        self.assertEqual(claude['denials'], {'p50':1,'n':3})
        self.assertEqual(codex['cost'], {'p50':None,'n':0})
        self.assertIsNone(codex['model'])
        self.assertIsNone(codex['vendor']['plan'])
        self.assertEqual(codex['judge'], {'parsed':1,'n':1})
        self.assertEqual(codex['usage_sources'], {'provider':1})
        self.build()
        self.assertEqual(len((self.root / 'calls').read_text().splitlines()), 4)
        self.record('d', ended=True)
        self.build()
        self.assertEqual(len((self.root / 'calls').read_text().splitlines()), 5)

    def test_cost_formula_missing_usage_and_reset(self):
        self.vendors[1]['window_budget_tokens'] = 1000
        self.record('a', rates=(.9,.1))
        self.record('b', provider='codex', model=None)
        result = sheet.build(self.runs, self.home, self.vendors, str(self.stub), .5)
        self.assertEqual(result[1]['cost'], {'p50':.07,'n':1})
        self.assertIsNone(result[0]['cost']['p50'])
        self.table['b']['projection']['usage'] = {'usage_source':'unavailable'}
        (self.root / 'table.json').write_text(json.dumps(self.table))
        (self.home / 'sheet-cache.json').unlink()
        self.assertEqual(self.build()[1]['tokens']['raw_input']['n'], 0)

    def test_prior_ranking_and_two_verdicts(self):
        self.record('a', rates=(.1,.2))
        self.record('b', rates=(.2,.3))
        sheets = self.build()
        self.assertEqual(len(sheet.rank(sheets, 'rust', 'medium')), 1)
        self.assertEqual(sheet.rank(sheets, 'rust', 'high'), [])
        verdicts = [dict(task=i, run_id=i, result='applied') for i in ('a','b')]
        after = sheet.rank(sheets, 'rust', 'high', verdicts)
        self.assertEqual(len(after), 1)
        self.assertEqual(after[0]['sample_size'], 2)
        self.assertAlmostEqual(after[0]['bound'], .1 ** (1/7), places=10)
        self.assertEqual(sheet.rank(sheets, 'rust', 'agi', verdicts), [])
        self.assertEqual(sheet.rank(sheets, 'rust', 'high', verdicts * 2), after)
        sheets[0]['available'] = False
        self.assertEqual(sheet.rank(sheets, 'rust', 'low', verdicts), [])
        self.assertEqual(sheet.rank(sheets[1], 'security_review', 'low', policy='read-only'), [])
        self.assertEqual(sheet.rank(sheets[1], 'security_review', 'low')[0]['cost'], None)

    def test_resume_is_not_first_pass_and_classes(self):
        self.record('a')
        self.record('b', parent='a', command='resume')
        agents = self.build()
        verdicts = [dict(task='task',run_id='b',result='applied')]
        ranked = sheet.rank(agents, 'rust', 'low', verdicts)
        self.assertEqual(ranked, [])  # Beta(5,2) lower bound misses .5.
        verdicts[0]['task_class'] = 'other'
        self.assertEqual(sheet.rank(agents, 'rust', 'medium', verdicts)[0]['sample_size'], 0)

    def test_judge_parser(self):
        for text in ('{}', 'no', '{"verdict":"pass","findings":[{"item":"unknown","severity":"minor","note":"x"}]}', '{"verdict":[],"findings":[]}'):
            self.assertFalse(sheet.judge_parsed(text))
        self.assertTrue(sheet.judge_parsed('{"verdict":"block","findings":[{"item":"security_issues","severity":"critical","note":"x"}]}'))
        # serde ignores additional keys in the current checkout.
        self.assertTrue(sheet.judge_parsed('{"verdict":"pass","findings":[],"extra":true}'))

    def test_cli(self):
        self.record('a')
        env = dict(os.environ, PATH=str(self.root) + os.pathsep + os.environ['PATH'])
        def cli(*args):
            return subprocess.run([sys.executable, str(MODULE), *args, '--home', str(self.home)],
                                  env=env, check=True, capture_output=True, text=True)
        self.assertGreater(json.loads(cli('build','--runs',str(self.runs)).stdout)['agents'], 0)
        self.assertEqual(json.loads(cli('show','claude','sonnet').stdout)['sample_size'], 1)
        self.assertTrue(json.loads(cli('rank','--class','rust','--grade','medium','--provider','claude').stdout))

    def test_goal_event_format_and_class_filter(self):
        self.record('a')
        self.record('b')
        agents = self.build()
        log = [{'event':'task','id':'one','class':'rust_launch_path'},
               {'event':'task','id':'two','class':'docs'},
               {'event':'human_verdict','task':'one','run_id':'a','result':'applied'},
               {'event':'human_verdict','task':'two','run_id':'b','result':'rejected'}]
        ranked = sheet.rank(agents, 'rust_launch_path', 'medium', log)
        self.assertEqual(ranked[0]['counts'], {'applied':1})
        self.assertEqual(ranked[0]['sample_size'], 1)


if __name__ == '__main__':
    unittest.main()
