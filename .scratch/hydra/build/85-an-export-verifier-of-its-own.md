# 85 - The export verifier stops sharing the redactor's patterns

Type: build
Status: claimed
Blocked by:

## What

Panel finding F33, stated in `SECURITY.md`: `verify_export_line` (`src/export.rs:534`) knows the
same five secret classes the redactor knows, so a secret the redactor misses the verifier misses
too, and the second check adds nothing for that class. Give the verifier patterns of its own:
whole-value matching on the multi-line classes (a PEM block from its BEGIN to its END line), and a
length-and-entropy rule for long opaque tokens that no named class covers, tuned so the existing
export fixtures still pass. A verifier hit aborts the export as today.

## Done when

`tests/export.rs` plants a secret the redactor does not know (a high-entropy token with no known
prefix) and the export aborts; the existing fixtures still export. `SECURITY.md`'s line on the
verifier is the session's to amend. Edits `src/export.rs`, `src/export_preference.rs` if it shares
the verifier, and `tests/` only.

## Outcome
