"""Map recorded provider and model spellings to one canonical pair."""
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent


def load_aliases(path=None):
    path = HERE / 'aliases.json' if path is None else Path(path)
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
