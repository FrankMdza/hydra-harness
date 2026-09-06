//! Ouroboros: the loops that read what the router recorded.
//!
//! `docs/ouroboros-spec.md` is the design. Rung one, the curator, is built and
//! lives in [`curate`] over the corpus selection in [`corpus`]. Rung two, reviewed
//! core evolution, is specified in that document's section 4 and is not built:
//! `hydra evolve`, the admission gate, the panel of three and the `ApprovedPatch`
//! type are all absent, and `IMPLEMENTATION.md` says so.
//!
//! **Neither rung may call `hydra reap`, `hydra prune` or `hydra export`, and the
//! gate is not what stops it.** Those are a binary reached through `shell`, so a
//! rule against them is an argv match with the pattern floor's failure mode:
//! silent permission on the spelling nobody wrote down. What holds them is that no
//! code path under this directory calls one, which is a property of Hydra's own
//! source rather than of any run. `evidence_is_read_only` in the review checklist
//! is the item that keeps it true under a self-edit, and it is the only mechanism
//! there is, so a reviewer may not defer to the gate on it.
//!
//! Everything Ouroboros is lives under this one directory for that reason. The
//! property is checkable by reading it, and `tests/curate.rs` reads it.
//!
//! **What crosses is the run record and the skill store, and nothing else** (D5).
//! This module reads the corpus through [`crate::normalize`] and writes a branch in
//! the store's own repository. No shared module, no database, no socket, no
//! scheduler.

pub mod corpus;
pub mod curate;
