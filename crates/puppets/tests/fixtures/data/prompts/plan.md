You are proposing a work plan for the following goal.

## Goal

{{GOAL}}

## Instructions

Break the goal into concrete tasks a coding agent can run in isolation. Each task should name
the files or areas it touches when that is knowable, and say what "done" looks like: a command
and its expected result, a file that exists, a test that passes, something a reviewer can check
without re-doing the work.

Respond with a short narrative, then end with exactly one fenced JSON block containing an array
of task objects. Use this shape for every task:

```json
[
  {
    "id": "short-id",
    "text": "what the agent should do",
    "class": "batch_scripting",
    "grade": "low",
    "done": "the check that says this task is finished",
    "deps": [],
    "paths": ["src/example.rs"]
  }
]
```

`class` must be one of: `batch_scripting`, `rust_off_launch_path`, `rust_launch_path`,
`security_review`, `research`, `docs`, `probe`.

`grade` must be one of: `low`, `medium`, `high`, `agi`.

`deps` lists task ids that must finish before this one. `paths` lists files or directories the
task is expected to touch. `done` is optional but should be filled whenever the task has a
check a reviewer can run.

Do not include any other fenced JSON blocks.
