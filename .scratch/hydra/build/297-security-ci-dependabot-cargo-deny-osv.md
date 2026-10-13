# 297 - Security in CI for free: Dependabot, cargo-deny and OSV-Scanner on Linux, actions pinned by SHA

Type: build
Status: resolved
Blocked by:
Opened: 2026-10-13, by the session, from `~/cerebro/research/security-and-compile.md` section 1,
"Recommended minimal stack", which the human put on Hydra's list on 2026-10-12.

## What

The repo is private and on a personal account, so CodeQL, secret scanning and Scorecard are not
available at any price until it goes public. What is free today: Dependabot for advisories and
version bumps, cargo-deny on every PR for advisories, licences, duplicates and sources, and
OSV-Scanner's PR workflow for "what did this PR introduce". None is a SaaS reading the repo. All
of it runs on `ubuntu-latest`: a macOS minute costs ten Linux minutes, and nothing here needs a
Mac. Every action is pinned to a commit SHA with the tag in a comment, because Trivy's tags were
replaced by malware on 2026-03-19 and a tag is not an identity. The same ticket makes
`CARGO_INCREMENTAL=0` explicit in `ci.yml`: a CI build is from scratch and incremental state
there is pure cost.

## Do

1. `.github/dependabot.yml`:

   ```yaml
   version: 2
   updates:
     - package-ecosystem: cargo
       directory: /
       schedule:
         interval: weekly
       open-pull-requests-limit: 5
       groups:
         cargo-minor-and-patch:
           update-types: [minor, patch]
     - package-ecosystem: github-actions
       directory: /
       schedule:
         interval: weekly
   ```

2. `.github/workflows/security.yml`, SHAs resolved on 2026-10-12:

   ```yaml
   name: security
   on:
     pull_request:
     push:
       branches: [master]
     schedule:
       - cron: "17 6 * * 1"
   permissions:
     contents: read
   jobs:
     cargo-deny:
       runs-on: ubuntu-latest
       strategy:
         fail-fast: false
         matrix:
           checks:
             - advisories
             - bans licenses sources
       steps:
         - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
         - uses: EmbarkStudios/cargo-deny-action@3c6349835b2b7b196a839186cb8b78e02f7b5f25 # v2
           with:
             command: check ${{ matrix.checks }}
             arguments: --workspace --all-features
             rust-version: "<the workspace's rust-version from Cargo.toml>"
     osv-pr:
       if: github.event_name == 'pull_request'
       permissions:
         contents: read
         actions: read
       uses: google/osv-scanner-action/.github/workflows/osv-scanner-reusable-pr.yml@b1d267aea5211fa648d9d442fa64b90dbdcbdd36 # v2.7.0
       with:
         scan-args: --lockfile=Cargo.lock
         upload-sarif: false
         fail-on-vuln: true
   ```

3. `deny.toml` at the repo root:

   ```toml
   [graph]
   targets = ["aarch64-apple-darwin", "x86_64-unknown-linux-gnu"]
   all-features = true

   [advisories]
   db-urls = ["https://github.com/rustsec/advisory-db"]
   unmaintained = "workspace"
   yanked = "deny"
   ignore = []

   [licenses]
   allow = ["Apache-2.0", "MIT", "BSD-2-Clause", "BSD-3-Clause", "ISC", "Unicode-3.0", "Zlib", "MPL-2.0"]
   confidence-threshold = 0.9

   [bans]
   multiple-versions = "warn"
   wildcards = "deny"

   [sources]
   unknown-registry = "deny"
   unknown-git = "deny"
   ```

   `cargo tree -d` already shows `hashbrown` twice, so `multiple-versions` stays `warn`. Run
   `cargo deny check` if the tool and the network are reachable from where you run; if either is
   refused, say so in the report and the session runs it after applying. Expect the licence list
   to need an entry or two; add what the first run names, and only that.
4. `ci.yml`: add `env: CARGO_INCREMENTAL: 0` at the job level, and pin its three actions
   (`actions/checkout`, `dtolnay/rust-toolchain`, `Swatinem/rust-cache`) to the SHA their tag
   points at today, tag in a comment. If you cannot resolve a SHA from where you run, leave that
   line and name it in the report.
5. One paragraph in `IMPLEMENTATION.md` under whatever heading covers CI: what runs where and
   what is still off because the repo is private. Do not touch repository settings; enabling
   Dependency graph and Dependabot alerts in GitHub's Code security page is the human's and is on
   his list.

## Done when

- The three files exist with the content above adapted to the first `cargo deny check`; `ci.yml`
  carries the env and the pins; `IMPLEMENTATION.md` says so; fmt, clippy and the suite untouched
  and green.

## Outcome

Built by `codex` `gpt-6.1-sol` on Hydra run `01a13405-c36a-7e30-b509-174ee94e03fa` under
`standard`, 209k tokens, applied to master on 2026-10-13 after the diff was read whole. The three
files are as the ticket gave them; `ci.yml` carries `CARGO_INCREMENTAL: 0` and three pins. The
session checked every pin against GitHub with `gh api`: five were right, and the one the builder
wrote for `Swatinem/rust-cache` named a commit that does not exist in that repository. It was
replaced with the commit the `v2` tag points at on 2026-10-13, `6323deb`, release 2.9.2. A pin a
builder resolves "by network" from inside a run is not verified until the session resolves it
again. `cargo deny check` on master, with cargo-deny 0.20.2 installed by brew: advisories, bans,
licenses and sources all ok; four duplicate-version warnings (`hashbrown`, `syn`, `thiserror`,
`thiserror-impl`) and two licences in the allow list that no crate uses. The licence list was not
changed. Enabling Dependency graph and Dependabot alerts in the repository settings is still the
human's.
