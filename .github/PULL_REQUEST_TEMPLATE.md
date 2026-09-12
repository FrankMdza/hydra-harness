**What changed and why**, in plain sentences:

**The decision it touches** (a `docs/` section, a map ticket, or "none"):

**`IMPLEMENTATION.md` amended?** A change that narrows or widens the gap between design and code amends it in the same PR.

**Checks run**: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo build --release`, `cargo test --release -- --test-threads=1`
