# hy [provider model policy] | [model] [policy]: a gated interactive session in Herdr, with the mascot above it.
# Source this file from ~/.zshrc. Build ticket 107; provider and model prompts since 129.
#
# Two panes to the right of the current one: the top one runs `hydra-mascot --watch`, the pet
# that reads every pane carrying a run id, and the bottom one runs the gated session.
# Hydra draws nothing inside the session itself (map decision D3, the vendor owns that terminal),
# which is why the hydra lives in a pane of its own.
_hy_measured_providers() {
    grep -v '^#' "$1" | sed '/^[[:space:]]*$/d'
}

_hy_provider_measured() {
    _hy_measured_providers "$2" | grep -Fxq "$1"
}

_hy_is_hydra_provider() {
    case "$1" in
        claude|cursor-agent|agy|codex) return 0 ;;
        *) return 1 ;;
    esac
}

_hy_refuse_unmeasured() {
    echo "hy: $1 has no measured interactive gate yet; build ticket 130 adds it." >&2
}

_hy_pick_provider() {
    local providers_file="$1"
    local n=0 p
    echo "hy: pick a provider with a measured interactive gate:" >&2
    while IFS= read -r p; do
        n=$((n + 1))
        printf '  %d) %s\n' "$n" "$p" >&2
    done < <(_hy_measured_providers "$providers_file")
    if [ "$n" -eq 0 ]; then
        echo "hy: no measured interactive providers in $providers_file" >&2
        return 1
    fi
    local choice=""
    read -r choice
    if [ -z "$choice" ]; then
        choice=1
    fi
    if ! [[ "$choice" =~ ^[0-9]+$ ]] || [ "$choice" -lt 1 ] || [ "$choice" -gt "$n" ]; then
        echo "hy: invalid provider choice: $choice" >&2
        return 1
    fi
    _hy_measured_providers "$providers_file" | sed -n "${choice}p"
}

_hy_pick_model() {
    local vendors_file="$1" provider="$2"
    local default="sonnet"
    local n=0 m
    echo "hy: pick a $provider model:" >&2
    while IFS= read -r m; do
        n=$((n + 1))
        printf '  %d) %s\n' "$n" "$m" >&2
    done < <(python3 - "$provider" "$vendors_file" <<'PY'
import json, sys
provider, path = sys.argv[1], sys.argv[2]
with open(path) as f:
    rows = json.load(f)
for row in rows:
    if row.get("provider") == provider and row.get("model"):
        print(row["model"])
PY
)
    echo "  or type a model name" >&2
    local prompt="model"
    if [ "$provider" = "claude" ]; then
        prompt="model [$default]"
    fi
    local choice=""
    printf '%s: ' "$prompt" >&2
    read -r choice
    if [ -z "$choice" ] && [ "$provider" = "claude" ]; then
        choice="$default"
    elif [ -z "$choice" ]; then
        echo "hy: $provider has no default model; type one" >&2
        return 1
    elif [[ "$choice" =~ ^[0-9]+$ ]] && [ "$choice" -ge 1 ] && [ "$choice" -le "$n" ]; then
        choice=$(python3 - "$provider" "$vendors_file" "$choice" <<'PY'
import json, sys
provider, path, idx = sys.argv[1], sys.argv[2], int(sys.argv[3]) - 1
with open(path) as f:
    rows = json.load(f)
models = [r["model"] for r in rows if r.get("provider") == provider and r.get("model")]
print(models[idx])
PY
)
    fi
    printf '%s' "$choice"
}

_hy_list_policies() {
    python3 - "$1" <<'PY'
import os, sys
d = sys.argv[1]
for name in sorted(
    os.path.splitext(f)[0]
    for f in os.listdir(d)
    if f.endswith(".json") and f != "floor.json"
):
    print(name)
PY
}

_hy_valid_policy_names() {
    _hy_list_policies "$1" | tr '\n' ' ' | sed 's/ $//'
}

_hy_policy_valid() {
    local policies_dir="$1" policy="$2"
    # A name, never a path: `../config/panels` would otherwise pass the file test.
    [ -n "$policy" ] && [ "$policy" != "floor" ] && [[ "$policy" != */* ]] && [ -f "$policies_dir/$policy.json" ]
}

_hy_refuse_unknown_policy() {
    local policy="$1" valid
    valid=$(_hy_valid_policy_names "$2")
    echo "hy: unknown policy $policy; valid names: $valid" >&2
}

_hy_pick_policy() {
    local policies_dir="$1"
    local n=0 p
    echo "hy: pick a policy:" >&2
    while IFS= read -r p; do
        n=$((n + 1))
        printf '  %d) %s\n' "$n" "$p" >&2
    done < <(_hy_list_policies "$policies_dir")
    local choice=""
    printf 'policy [standard]: ' >&2
    read -r choice
    if [ -z "$choice" ]; then
        choice="standard"
    elif [[ "$choice" =~ ^[0-9]+$ ]] && [ "$choice" -ge 1 ] && [ "$choice" -le "$n" ]; then
        choice=$(_hy_list_policies "$policies_dir" | sed -n "${choice}p")
    fi
    if ! _hy_policy_valid "$policies_dir" "$choice"; then
        _hy_refuse_unknown_policy "$choice" "$policies_dir"
        return 1
    fi
    printf '%s' "$choice"
}

hy() {
    if [ -z "$HERDR_ENV" ]; then
        echo "hy: not inside Herdr" >&2
        return 1
    fi
    local here="${HYDRA_HERDR:-$HOME/hydra/integrations/herdr}"
    local providers_file="$here/interactive-gate-providers.txt"
    local policies_dir="$here/../../policies"
    local vendors_file="$here/../puppets/vendors.json"
    local bin="${HYDRA_BIN:-$HOME/hydra/target/release/hydra}"
    local provider model policy
    local top bottom

    if [ $# -eq 0 ]; then
        provider=$(_hy_pick_provider "$providers_file") || return 1
        model=$(_hy_pick_model "$vendors_file" "$provider") || return 1
        policy=$(_hy_pick_policy "$policies_dir") || return 1
    elif [ $# -ge 3 ] && _hy_is_hydra_provider "$1"; then
        if ! _hy_provider_measured "$1" "$providers_file"; then
            _hy_refuse_unmeasured "$1"
            return 1
        fi
        provider="$1"
        model="$2"
        policy="$3"
        if ! _hy_policy_valid "$policies_dir" "$policy"; then
            _hy_refuse_unknown_policy "$policy" "$policies_dir"
            return 1
        fi
    else
        if _hy_is_hydra_provider "$1" && ! _hy_provider_measured "$1" "$providers_file"; then
            _hy_refuse_unmeasured "$1"
            return 1
        fi
        provider="claude"
        model="${1:-sonnet}"
        policy="${2:-standard}"
        if ! _hy_policy_valid "$policies_dir" "$policy"; then
            _hy_refuse_unknown_policy "$policy" "$policies_dir"
            return 1
        fi
    fi

    if ! _hy_provider_measured "$provider" "$providers_file"; then
        _hy_refuse_unmeasured "$provider"
        return 1
    fi

    # The session gets a worktree of the repo it starts in. Outside any repo, Hydra's own
    # checkout is that repo, so `hy` works from any directory once ~/.zshrc sources this file.
    local dir="$PWD"
    if ! git -C "$dir" rev-parse --show-toplevel >/dev/null 2>&1; then
        dir="$HOME/hydra"
        echo "hy: $PWD is not inside a git repo; the session works on $dir" >&2
    fi
    top=$(herdr pane split "$HERDR_PANE_ID" --direction right --cwd "$dir" --env "HYDRA_BIN=$bin" \
        | python3 -c 'import sys, json; print(json.load(sys.stdin)["result"]["pane"]["pane_id"])') || return 1
    bottom=$(herdr pane split "$top" --direction down --ratio 0.3 --cwd "$dir" --env "HYDRA_BIN=$bin" \
        | python3 -c 'import sys, json; print(json.load(sys.stdin)["result"]["pane"]["pane_id"])') || return 1
    herdr pane run "$top" "python3 $here/bin/hydra-mascot --watch" >/dev/null
    herdr pane run "$bottom" "bash $here/bin/hydra-tui-in-pane --provider $provider --model $model --policy $policy" >/dev/null
    echo "hy: mascot $top, $provider $bottom"
}
