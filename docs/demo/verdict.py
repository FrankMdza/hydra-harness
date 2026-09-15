#!/usr/bin/env python3
"""Print what the vendor claimed and what the record derived, for one run id."""
import json, subprocess, sys, os
run_id = sys.argv[1]
hydra = sys.argv[2] if len(sys.argv) > 2 else "hydra"
# The same root the binary reads, and for the same reason: a clone that is not
# ~/hydra still writes its records wherever HYDRA_ROOT says (build ticket 52).
root = os.environ.get("HYDRA_ROOT") or os.path.expanduser("~/hydra")
rec = os.path.join(root, "runs", f"{run_id}.jsonl")
rows = [json.loads(l) for l in open(rec)]
def vendor_claim():
    """The vendor's own verdict, whatever it calls it: agy says status, claude says outcome."""
    for r in reversed(rows):
        raw = r.get("raw")
        if isinstance(raw, dict):
            for k in ("outcome", "status", "is_error", "subtype"):
                if k in raw:
                    return f"{k}={raw[k]!r}"
            # agy nests it: {"event": ..., "result": {"status": "SUCCESS"}}
            for k, v in raw.items():
                if isinstance(v, dict):
                    for kk in ("status", "state", "outcome"):
                        if kk in v:
                            return f"{k}.{kk}={v[kk]!r}"
    return "no verdict field"
shown = json.loads(subprocess.run([hydra, "show", run_id, "--events"], capture_output=True, text=True).stdout)
denies = [e for e in shown["events"] if e.get("type") == "permission_decision" and e.get("decision") == "deny"]
print(f"vendor says:  exit_code={shown['projection'].get('exit_code')}  {vendor_claim()}")
print(f"record says:  outcome={shown['projection']['outcome']!r}  ({len(denies)} tool call(s) refused by the gate)")
for e in denies[:2]:
    print(f"  refused:    {e['reason'][:110]}")
