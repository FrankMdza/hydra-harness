#!/bin/bash
set -euo pipefail

# Paths are taken from this file so the caller cwd does not matter.
TEST_DIR="$(cd "$(dirname "$0")" && pwd)"
PLUGIN_DIR="$(cd "$TEST_DIR/.." && pwd)"
REPO_ROOT="$(cd "$PLUGIN_DIR/../.." && pwd)"
HYDRA_IN_PANE="$PLUGIN_DIR/bin/hydra-in-pane"

OPENED_PANE=""
OPENED_BRIDGE=""

# Close only panes this script opened.
cleanup() {
    if [ -n "${OPENED_PANE:-}" ]; then
        herdr pane close "$OPENED_PANE" >/dev/null 2>&1 || true
        OPENED_PANE=""
    fi
    if [ -n "${OPENED_BRIDGE:-}" ]; then
        herdr plugin pane close "$OPENED_BRIDGE" >/dev/null 2>&1 || true
        OPENED_BRIDGE=""
    fi
}
trap cleanup EXIT

# Refuse unless a Herdr server is running.
if ! SERVER_STATUS="$(herdr status server 2>/dev/null)" || ! printf '%s\n' "$SERVER_STATUS" | grep -q '^status: running$'; then
    echo "smoke: herdr server is not running" >&2
    exit 1
fi

# Refuse unless HYDRA_BIN names an executable.
if [ -z "${HYDRA_BIN:-}" ] || [ ! -x "$HYDRA_BIN" ]; then
    echo "smoke: HYDRA_BIN does not name an executable" >&2
    exit 1
fi

echo "smoke: hydra $HYDRA_BIN"

LIST_JSON="$(herdr pane list)"

FIRST_PANE="$(printf '%s\n' "$LIST_JSON" | python3 -c 'import sys, json; panes=(json.load(sys.stdin).get("result") or {}).get("panes") or []; print(panes[0]["pane_id"] if panes else "")')"
if [ -z "$FIRST_PANE" ]; then
    echo "smoke: herdr pane list returned no panes" >&2
    exit 1
fi

# Open the bridge pane if none already has its cwd in the plugin directory.
BRIDGE_EXISTING="$(printf '%s\n' "$LIST_JSON" | python3 -c 'import sys, json; panes=(json.load(sys.stdin).get("result") or {}).get("panes") or [];
print(next((p.get("pane_id") or "" for p in panes if (p.get("cwd") or "").endswith("integrations/herdr")), ""))')"

if [ -z "$BRIDGE_EXISTING" ]; then
    OPEN_JSON="$(herdr plugin pane open --plugin hydra.bridge --entrypoint bridge --env "HYDRA_BIN=$HYDRA_BIN" --no-focus)"
    OPENED_BRIDGE="$(printf '%s\n' "$OPEN_JSON" | python3 -c 'import sys, json; print(json.load(sys.stdin)["result"]["plugin_pane"]["pane"]["pane_id"])')"
    echo "smoke: opened bridge $OPENED_BRIDGE"
fi

# Split a throwaway pane from the first pane Herdr listed.
SPLIT_JSON="$(herdr pane split "$FIRST_PANE" --direction down --cwd "$REPO_ROOT" --env "HYDRA_BIN=$HYDRA_BIN" --no-focus)"
OPENED_PANE="$(printf '%s\n' "$SPLIT_JSON" | python3 -c 'import sys, json; print(json.load(sys.stdin)["result"]["pane"]["pane_id"])')"
echo "smoke: opened pane $OPENED_PANE"

# Run hydra-in-pane in the new pane.
TASK='Answer in one line: what is 2+2? Do not read any file.'
herdr pane run "$OPENED_PANE" "bash '$HYDRA_IN_PANE' --provider claude --model haiku --policy read-only --task '$TASK'" >/dev/null

# Poll until the idle label is completed or time runs out.
DEADLINE=$((SECONDS + 120))
LAST_RUN_ID=""
LAST_IDLE=""
PASSED=0
while [ "$SECONDS" -lt "$DEADLINE" ]; do
    GET_JSON="$(herdr pane get "$OPENED_PANE" 2>/dev/null || true)"
    PARSE="$(printf '%s\n' "$GET_JSON" | python3 -c 'import sys, json; run_id=""; idle=""
try:
    pane=(json.load(sys.stdin).get("result") or {}).get("pane") or {}; tokens=pane.get("tokens") or {}; labels=pane.get("state_labels") or {}; run_id=tokens.get("run_id") or ""; idle=labels.get("idle") or ""
except Exception:
    pass
print(run_id + "\t" + idle)')"
    LAST_RUN_ID="${PARSE%%$'\t'*}"
    LAST_IDLE="${PARSE#*$'\t'}"
    echo "smoke: poll run_id=${LAST_RUN_ID:-none} idle=${LAST_IDLE:-none}"
    if [ -n "$LAST_RUN_ID" ] && [ "$LAST_IDLE" = "completed" ]; then
        PASSED=1
        break
    fi
    sleep 3
done

if [ "$PASSED" -eq 1 ]; then
    echo "smoke: passed run_id=$LAST_RUN_ID idle=$LAST_IDLE"
    exit 0
fi

echo "smoke: failed run_id=${LAST_RUN_ID:-none} idle=${LAST_IDLE:-none}" >&2
exit 1
