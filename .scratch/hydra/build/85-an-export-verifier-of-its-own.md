# 85 - The export verifier stops sharing the redactor's patterns

Type: build
Status: resolved
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

`verify_export_line` no longer calls the redactor. It decodes each line, scans keys and values, and
aborts on a PEM header or one of the known prefixes (`scan_text`); it returns `EntropyWarning`s,
field path, class and length and never the value, for tokens of 32 bytes or more whose entropy is at
least 0.80 of log2(min(len, 64)) and that hold a 24-character run with no `/`, `.`, `_` or `-`, after
UUIDs, `sha*-` integrity strings and data-URI payloads are masked. `hydra export` and the preference
export print the warnings after writing. The ticket said a verifier hit aborts as today, and the
human changed that on measurement: an aborting entropy rule stopped 108 of 321 real records, mostly
on Cursor `contentBlobId` digests, so entropy warns and prefixes and PEM abort. The run's first
version held a fixed 5-bit threshold that a 32-byte token cannot reach and a PEM check that
reduced to `contains`; the Opus review measured both. A 40-byte base64 secret whose `/` characters
break every 24-character run is missed, pinned by a test. Measured on master: export 20, lib 195
passed; clippy and fmt clean. Built by `codex` under `standard`, run 01a0cc32-3831, 1.2M tokens;
an Opus subagent reworked the rule over three rounds with the human's decision in between.
