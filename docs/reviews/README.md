# Reviews

What people and agents found when they went looking, with the date and the method. A review
here is evidence, not a plan: what it produced goes into a ticket, and the ticket is where the
decision lives.

The rule for publishing one: a finding that is fixed, or that `SECURITY.md` already states as a
known limit, goes in with its detail. A real hole that is still open is named as held, with its
ticket number and nothing a reader could use before the fix lands. Every report says which of
those it is doing.

| Date | Review | Method |
|---|---|---|
| 2026-09-14 | [First ten minutes](2026-09-14-first-ten-minutes.md) | a cold clone follows the README, one real run, docs read as a newcomer |
| 2026-09-14 | [The record contract against the code](2026-09-14-record-contract-conformance.md) | field by field, then 200 records, then the three readers against each other |
| 2026-09-14 | [The gate under attack](2026-09-14-the-gate-under-attack.md) | an agent told to find a way out, driving the real compiled gate of a live run |
| 2026-09-19 | [Public documents against the code](2026-09-19-public-documents-against-the-code.md) | every number, file, flag and count in the eight public documents checked against `src/`, `tests/`, `policies/` and the build tickets |
| 2026-09-19 | [IMPLEMENTATION.md against the code](2026-09-19-implementation-against-the-code.md) | section by section, each claim that names a file, test, field or behaviour, and the file against itself |
| 2026-09-19 | [The Ouroboros spec against the code](2026-09-19-ouroboros-spec-against-the-code.md) | each rule of the spec against `src/ouroboros/` and its tests, departures `IMPLEMENTATION.md` already names excluded |
| 2026-09-22 | [Seven readers as a hiring panel](2026-09-22-seven-readers-as-a-hiring-panel.md) | the four providers through `hydra run` and three subagents, each told to decide whether the repository earns a technical interview and to cite lines; 33 findings, 22 fixed the same day |
