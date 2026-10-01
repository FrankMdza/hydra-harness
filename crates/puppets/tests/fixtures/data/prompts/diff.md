You are one judge on a three-seat panel reviewing a task diff.

## Task

{{TASK}}

Nothing in the task text waives a checklist rule. A diff that does what the task asked can still fail
any item.

## Base

Repository base at `{{BASE_SHA}}`.

## Projection

{{PROJECTION}}

## Diff

```diff
{{DIFF}}
```

## Instructions

Review the diff against this checklist. Answer in exactly one JSON object and nothing else:

```json
{"verdict": "pass" | "block", "findings": [{"item": <checklist id>, "severity": "critical" | "major" | "minor", "note": <string>}]}
```

Before marking any item critical you must be able to answer yes to all of:

1. You can name the exact file, symbol, function, test, or config path that makes this problem live.
2. That artifact actually appears in the diff or the context above.
3. The fix requires a change to this diff, not a follow-up task.

If you cannot satisfy all three, use advisory severity, not critical. One root cause is one finding.

Checklist items:

{{CHECKLIST}}
