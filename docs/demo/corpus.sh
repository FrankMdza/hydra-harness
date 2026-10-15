#!/usr/bin/env bash
# The scripted session behind docs/demo/corpus.gif: reading what Hydra already recorded.
# Launches nothing and spends no quota. Run through record.sh (`bash docs/demo/record.sh corpus`).
#
# It needs a corpus: at least one record under ~/hydra/runs, and one of them gated. The
# first argument is the run id (a prefix is enough) of a gated run to show; without it the
# script picks the newest record whose outcome is `gated`, which takes a few seconds.
CHECKOUT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export PATH="$CHECKOUT/target/release:$PATH"
cd "$CHECKOUT"
say() {  # fake typing, then run it
  printf '\033[1;32m$\033[0m '
  local s="$1"; for ((i=0;i<${#s};i++)); do printf '%s' "${s:i:1}"; sleep 0.028; done
  printf '\n'; sleep 0.4; eval "$1"; sleep "${2:-1.4}"
}
note() { printf '\033[2m# %s\033[0m\n' "$1"; sleep 1.1; }

GATED="${1:-}"
if [ -z "$GATED" ]; then
  for f in $(ls -t ~/hydra/runs/*.jsonl | head -200); do
    id="$(basename "$f" .jsonl)"
    if hydra show "$id" 2>/dev/null | grep -q '"outcome": "gated"'; then GATED="$id"; break; fi
  done
fi
[ -n "$GATED" ] || { echo "corpus.sh: no gated record found; pass a run id" >&2; exit 1; }
SHORT="${GATED:0:8}"

sleep 0.6
note "no daemon, no database: the records under ~/hydra/runs are the whole state"
say 'hydra status | head -6'
note "one record, normalized. The vendor exited 0; the record derives its own outcome"
say "hydra show $SHORT --events | grep -m4 -E '\"(launch_mode|policy|provider_version|subscription)\"'" 2
say "hydra show $SHORT | grep -E '\"(exit_code|outcome|outcome_source)\"'" 2.5
note "the refused calls are on the record, with the gate's reason"
say "hydra show $SHORT --events | grep -o '\"decision\": \"[a-z]*\"' | sort | uniq -c" 2.5
say "hydra show $SHORT --events | grep -m2 -o '\"reason\": \"[^\"]*\"'" 2.5
note "every record on this machine against the contract"
say 'hydra doctor | tail -8' 2.5
note "the build is tickets, and each resolved one names the run that built it"
say 'hydra-puppets frontier build | head -5' 2
note "the models each vendor offers, from the last refresh; launches nothing"
say 'hydra models | head -8' 2.5
