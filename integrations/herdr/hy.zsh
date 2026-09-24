# hy [model] [policy]: a gated interactive claude session in Herdr, with the mascot above it.
# Source this file from ~/.zshrc. Build ticket 107; the mascot pane since build 122.
#
# Two panes to the right of the current one: the top one runs `hydra-mascot --watch`, the pet
# that reads every pane carrying a run id, and the bottom one runs the gated claude session.
# Hydra draws nothing inside the session itself (map decision D3, the vendor owns that terminal),
# which is why the hydra lives in a pane of its own.
hy() {
    if [ -z "$HERDR_ENV" ]; then
        echo "hy: not inside Herdr" >&2
        return 1
    fi
    local model="${1:-sonnet}"
    local policy="${2:-standard}"
    local bin="${HYDRA_BIN:-$HOME/hydra/target/release/hydra}"
    local here="$HOME/hydra/integrations/herdr"
    local top bottom
    top=$(herdr pane split "$HERDR_PANE_ID" --direction right --cwd "$PWD" --env "HYDRA_BIN=$bin" \
        | python3 -c 'import sys, json; print(json.load(sys.stdin)["result"]["pane"]["pane_id"])') || return 1
    bottom=$(herdr pane split "$top" --direction down --ratio 0.3 --cwd "$PWD" --env "HYDRA_BIN=$bin" \
        | python3 -c 'import sys, json; print(json.load(sys.stdin)["result"]["pane"]["pane_id"])') || return 1
    herdr pane run "$top" "python3 $here/bin/hydra-mascot --watch" >/dev/null
    herdr pane run "$bottom" "bash $here/bin/hydra-tui-in-pane --provider claude --model $model --policy $policy" >/dev/null
    echo "hy: mascot $top, claude $bottom"
}
