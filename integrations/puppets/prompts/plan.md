You are proposing a work plan for the following goal.

## Goal

{{GOAL}}

## Instructions

Break the goal into concrete tasks a coding agent can run in isolation. Each task should name
the files or areas it touches when that is knowable.

Respond with a short narrative, then end with exactly one fenced JSON block containing an array
of task objects. Use this shape for every task:

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

`deps` lists task ids that must finish before this one. `paths` lists files or directories the
task is expected to touch.

Do not include any other fenced JSON blocks.
