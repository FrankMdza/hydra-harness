# Orchestrator cost bench

Map 107, D5 C: a fixed set of five goals in three sizes, each run three times through Master of
Puppets. Orchestrator tokens against the agents of that run, then the median and the spread.
Real sessions sit beside the set as a check that it is typical.

The command is `hydra cost`, in `src/cost.rs`. It walks the run records in a Layout the way
`hydra usage` and `hydra show` do. It does not live under `crates/puppets`: that binary reads
goal logs and does not depend on hydra. Running a goal is the session's, one a night. This
directory holds only the goals and the tiny repository they run against.

## The repository

`repo/` is a twelve-line Rust library, `tally`. Copy it to a scratch directory that is not
this checkout, `cd` there, and start the goal with `hydra-puppets new` on the goal file's
body. A fresh copy per repetition so one run's agents do not leak into the next. Three
repetitions per goal.

## The goals

| File | Size | What it asks |
|---|---|---|
| `goals/s1-readme-typo.md` | small | one typo in the README |
| `goals/s2-one-test.md` | small | one unit test for an existing function |
| `goals/m1-chars.md` | medium | a new function, its tests, a README line |
| `goals/m2-saturating-bump.md` | medium | a wrapping bug across the library and tests |
| `goals/l1-cli.md` | large | a CLI, a flag, tests, and a README section |

## Report

After a night's three repetitions, with the orchestrator run ids in hand:

```
hydra cost <orch-1> <orch-2> <orch-3>
```

`--check` takes real Master of Puppets sessions and prints them beside the set:

```
hydra cost <orch-1> <orch-2> <orch-3> --check <session-a> <session-b>
```

Each row is orchestrator `total_tokens` over the sum of the agents that started after it in
the same repository. The summary line is n, median, min, max, over the rows that had agents.
The command launches nothing.
