# 382 - A signed round manifest: what comes back from the cloud is checked before it loads

Type: build
Status: open
Blocked by: map 139

## Objective

Map 137, ticket 3. Every round in the cloud writes a manifest of SHA-256 hashes of what it made
and an ML-DSA-65 (FIPS 204) signature over it; the Mac refuses a flipped byte, a wrong key or a
missing signature before anything loads. The crate is chosen by NIST's known-answer vectors.

Blocked by map 139 because the human asked for a post-quantum system of the lab's own, and the
manifest's format, key custody and crate come from that answer. The limit GPT stated on map 137
stays in scope: a key the training code can reach signs whatever that code produces.

## Done when

Tests refuse the three cases; round one's adapter, replayed through it, loads.

## Outcome
