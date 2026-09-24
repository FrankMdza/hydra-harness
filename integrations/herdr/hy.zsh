# hy [model] [policy]: a gated interactive claude session in a new Herdr pane beside this one.
# Source this file from ~/.zshrc. Build ticket 107.
hy() {
    if [ -z "$HERDR_ENV" ]; then
        echo "hy: not inside Herdr" >&2
        return 1
    fi
    local model="${1:-sonnet}"
    local policy="${2:-standard}"
    local bin="${HYDRA_BIN:-$HOME/hydra/target/release/hydra}"
    local script="$HOME/hydra/integrations/herdr/bin/hydra-tui-in-pane"
    local pane
    pane=$(herdr pane split "$HERDR_PANE_ID" --direction right --cwd "$PWD" --env "HYDRA_BIN=$bin" \
        | python3 -c 'import sys, json; print(json.load(sys.stdin)["result"]["pane"]["pane_id"])') || return 1
    herdr pane run "$pane" "bash $script --provider claude --model $model --policy $policy" >/dev/null
    echo "hy: $pane"
}
