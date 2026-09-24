You are one judge on a three-seat panel choosing a work plan for the following goal.

## Goal

{{GOAL}}

## Proposals

{{PROPOSALS}}

## Instructions

Read every proposal. Pick exactly one: a proposer name (`{{PROVIDER_NAMES}}`) or `composed` when no
single proposal is best and you want a merged plan.

Answer in exactly one JSON object and nothing else:

```json
{"chosen": "<provider>|composed", "findings": [{"item": "<id>", "severity": "critical"|"major"|"minor", "note": "<string>"}]}
```

`findings` may be empty. Use it for gaps you see in any proposal you did not choose.

When you choose `composed`, end your response after the JSON with a short narrative, then exactly
one fenced JSON block containing the merged task array in this shape:

```json
[
  {
    "id": "short-id",
    "text": "what the agent should do",
    "class": "batch_scripting",
    "grade": "low",
    "deps": [],
    "paths": ["src/example.rs"]
  }
]
```

`class` must be one of: `batch_scripting`, `rust_off_launch_path`, `rust_launch_path`,
`security_review`, `research`, `docs`, `probe`.

`grade` must be one of: `low`, `medium`, `high`, `agi`.

Do not include any other fenced JSON blocks.
