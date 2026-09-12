#!/usr/bin/env bash
# The scripted session behind docs/demo/hydra.gif. Run through record.sh, not by hand.
export PATH="$HOME/hydra/target/release:$PATH"
cd "$HOME/hydra"
say() {  # fake typing, then run it
  printf '\033[1;32m$\033[0m '
  local s="$1"; for ((i=0;i<${#s};i++)); do printf '%s' "${s:i:1}"; sleep 0.028; done
  printf '\n'; sleep 0.4; eval "$1"; sleep "${2:-1.2}"
}
note() { printf '\033[2m# %s\033[0m\n' "$1"; sleep 1.1; }
sleep 0.6
note "one command, three vendor agents, every run recorded and gated"
say 'hydra status | head -1'
say 'hydra run --provider agy --policy standard --timeout 3m --task "Count the run records: run ls ~/hydra/runs | wc -l and report the number." 2>/dev/null | tee /tmp/hydra-demo.json | head -6' 1.5
say 'id=$(python3 -c "import json;print(json.load(open(\"/tmp/hydra-demo.json\"))[\"run_id\"])"); echo $id'
note "the vendor exited 0 and called it a success. the record disagrees:"
say 'python3 docs/demo/verdict.py $id' 4
say 'hydra show ${id:0:13} | head -12   # a prefix is enough' 4
