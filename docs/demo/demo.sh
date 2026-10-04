#!/usr/bin/env bash
# The scripted session behind docs/demo/hydra.gif. Run through record.sh, not by hand.
#
# It launches two real runs, so it spends quota: one `agy` run on the CLI's default
# model and one `claude` run on Haiku. The cockpit part launches nothing.
#
# The checkout is wherever this script lives, so a clone records itself. The data
# root is separate: Hydra reads HYDRA_ROOT when it is set and ~/hydra when it is
# not, whatever directory the binary was built in (build ticket 52).
#
# Run it from the real checkout. `hydra run` refuses a launch from under
# worktrees/ (build ticket 99), and a seatbelt does not nest inside a run.
CHECKOUT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export PATH="$CHECKOUT/target/release:$PATH"
cd "$CHECKOUT"
say() {  # fake typing, then run it
  printf '\033[1;32m$\033[0m '
  local s="$1"; for ((i=0;i<${#s};i++)); do printf '%s' "${s:i:1}"; sleep 0.028; done
  printf '\n'; sleep 0.4; eval "$1"; sleep "${2:-1.2}"
}
note() { printf '\033[2m# %s\033[0m\n' "$1"; sleep 1.1; }
# `hydra run` prints one JSON object on stdout. This reads its run_id.
run_id() { python3 -c 'import json,sys; print(json.load(sys.stdin)["run_id"])'; }
launched() {  # stop the recording when a launch printed no run id
  [ -n "$1" ] && return
  echo "the launch printed no run id. Type the same command without 2>/dev/null to read why."
  exit 1
}

# The cockpit is a full-screen view and has no snapshot mode, so it runs in a
# detached tmux session of its own server (-L hydra-demo, no config file, so the
# author's tmux is not touched). A background loop sends the keys with
# `tmux send-keys`, and `tmux attach` puts the session on the recorded terminal.
# The last key is q, the session ends with the cockpit, and attach returns.
cockpit() {
  local tm=(tmux -L hydra-demo -f /dev/null)
  "${tm[@]}" kill-server 2>/dev/null
  "${tm[@]}" start-server \; set -g status off \; set -g default-terminal screen-256color \; \
    new-session -d -s cockpit -x 118 -y 30 "hydra cockpit ${HYDRA_DEMO_GOAL:+--goal $HYDRA_DEMO_GOAL}"
  (
    key() { sleep "$1"; "${tm[@]}" send-keys -t cockpit "$2"; }
    # Focus starts on the task table. Four Tabs reach the fourth lane, `ended not ok`,
    # where the gated run is the newest card.
    key 4 Tab; key 0.5 Tab; key 0.5 Tab; key 0.5 Tab
    key 1.5 Enter    # that run as a full page: outcome, tokens, the refused call
    key 5 Escape
    key 1.5 k        # one lane per agent
    key 4 k          # back to the four lanes by state
    key 1.5 t        # the shown goal's task table, full page, with its panel votes
    key 6 t
    key 1.5 '?'      # every key the cockpit answers
    key 4 Escape
    key 1.5 q
    sleep 2; "${tm[@]}" kill-server 2>/dev/null   # in case q did not land
  ) &
  "${tm[@]}" attach -t cockpit
  wait
}

sleep 0.6
note "what the copilot types: two agents, each in its own worktree. --timeout 0 returns at once"
say 'a=$(hydra run --provider agy --policy standard --timeout 0 --task "Count the run records: run ls ~/hydra/runs | wc -l and report the number." 2>/dev/null | run_id); echo $a'
launched "$a"
say 'b=$(hydra run --provider claude --model haiku --policy read-only --timeout 0 --task "Read README.md and say in one sentence what Hydra does." 2>/dev/null | run_id); echo $b'
launched "$b"
note "one blocking command waits for both: id, provider, model, outcome, tokens"
say 'hydra wait $a $b --timeout 5m' 3
note "what the vendor said about the first run, and what the record says:"
say 'python3 docs/demo/verdict.py $a' 4
note "the cockpit: runs by state, one run full page, lanes per agent, the last goal's task table"
sleep 1
cockpit
