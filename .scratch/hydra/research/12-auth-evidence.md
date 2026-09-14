# 12 - Auth evidence

Ticket: [12 - Two auth planes, and what v1 ships](../issues/12-auth-planes.md)

**Withdrawn before this repository was published, 2026-09-12.** This document recorded how each
provider's credential was found on the machine Hydra was built on, and named the keychain items
and environment variables involved. Its conclusions are in ticket 12 and in the map's D4 and D6:
every vendor CLI owns its own token, Hydra never reads or forwards one, and the child's
environment is built from an allowlist so a key exported in a shell profile never reaches a run.

The method it used is worth keeping in one sentence, because it is the reason the conclusions
are trustworthy: every check read attributes only, never a secret's value, so
`security find-generic-password -s <service>` was run without `-g` throughout.
