"""Map recorded provider and model spellings to one canonical pair.

This was `integrations/puppets/models.py`. Build ticket 220 moved it beside `records.py`, the
one file that loads it. The table stays in `integrations/puppets/`, where `hydra-puppets` and
`hydra-grader` read it.
"""
import json
from pathlib import Path

DATA = Path(__file__).resolve().parents[1] / 'puppets'


def load_aliases(path=None):
    path = DATA / 'aliases.json' if path is None else Path(path)
    rows = json.loads(path.read_text())
    table = {}
    for row in rows:
        recorded = (row['provider'], row.get('recorded_model'))
        canonical = row['canonical']
        table[recorded] = (canonical['provider'], canonical.get('model'))
    return table


def canonical(provider, model, aliases=None):
    aliases = load_aliases() if aliases is None else aliases
    return aliases.get((provider, model), (provider, model))
