# 21 - What environment each provider needs, and what it leaks

Type: research
Status: resolved
Blocked by: 12

## Question

Ticket 12 decided that Hydra builds each child's environment from an allowlist, so no provider
secret reaches an agent by inheritance. The allowlist is written as `PATH`, `HOME`, `TERM`, `LANG`
"plus whatever a given provider genuinely needs", and that last clause is unmeasured. Scrub too
hard and a provider fails to launch or silently loses its login. Scrub too little and D6 is a claim
rather than a fact.

Settle, for `claude` 2.1.257, `cursor-agent` 2026.08.31-4057e58, `agy` 1.1.23 and `hermes` 0.14.0:

- Which environment variables each binary reads, and which of those it needs to start, to reach its
  own credential, and to run a task
- Which variables carry a secret, so the scrub has a complete target list rather than
  `GEMINI_API_KEY` alone
- What each one does when a needed variable is missing: a clear error, a silent degrade, or a
  fallback to a different auth path
- Whether any of them read a proxy, CA or telemetry variable that Hydra should pass through on
  purpose rather than by accident
- The minimum viable environment for each provider, stated as a list

The answer becomes the allowlist in the harness spec and feeds ticket 13's launch sequence.

## Answer

**The allowlist is `HOME`, `USER`, `PATH` and `TERM`, plus `SHELL` and `TMPDIR` for `claude`.**
Measurements in [research/21-provider-env-requirements.md](../research/21-provider-env-requirements.md),
942 lines. Six findings sharpen ticket 12's decision and three contradict it.

### The list

Required by at least one provider:

```
HOME    the real home, or omit entirely. Never a different path. agy hard-fails without it
USER    claude's keychain account name is process.env.USER. Without it, claude is logged out
PATH    must contain /usr/bin for the keychain read, and every shell tool the providers call
TERM    rendering, read by claude, cursor-agent and hermes
SHELL   claude only, for shell classification
TMPDIR  claude only, and it falls back on its own if absent
```

Passed through on purpose when the network needs it: `HTTPS_PROXY`, `HTTP_PROXY`, `NO_PROXY` and
their lowercase twins, `NODE_EXTRA_CA_CERTS` for the two Node providers, `SSL_CERT_FILE` and
`REQUESTS_CA_BUNDLE` for hermes.

`LANG` comes off the list as a provider requirement. None of the four reads it. Keep it for libc
and Python encoding if you like, but do not call it a dependency.

### `HOME` is the rule to get right

`security list-keychains` returns only the system keychain when `HOME` points anywhere but the real
home, so the login keychain drops out of the search list and every keychain-backed provider comes up
unauthenticated with exit code 0. Three of the four fail this way, silently. An absent `HOME` is
safer than a wrong one, because the search list falls back to `getpwuid`. `agy` refuses to start
without it at all.

So Hydra either passes the real `HOME` or omits it, and never invents one. A sandbox that redirects
`HOME` to a scratch directory would log the user out of three providers and report success.

### What the scrub protects beyond secrets

**A stolen login, not only a leaked key.** `CURSOR_AUTH_TOKEN` and `CURSOR_API_KEY` are
write-through: `cursor-agent` passes the environment value to `setAuthentication()`, so an inherited
value overwrites the stored keychain login rather than overriding it for one run.

**Arbitrary code execution.** `NODE_OPTIONS=--require <file>` runs attacker-named JavaScript inside
`cursor-agent`, and `PYTHONPATH` with a `sitecustomize.py` runs Python inside hermes. Both confirmed
by running them. The allowlist is an injection boundary, not only hygiene.

**Approval bypass.** `HERMES_YOLO_MODE` and `HERMES_ACCEPT_HOOKS` turn hermes' gates off from the
environment.

**Session cross-talk.** This shell exports `CLAUDE_CODE_MESSAGING_SOCKET`,
`CLAUDE_CODE_MESSAGING_TOKEN`, `CLAUDE_CODE_SESSION_ID` and two more. An unscrubbed child `claude`
writes `messagingSocketPath` from that variable into its own pid record and joins the parent's
messaging plane. That is a correctness bug on top of a hygiene one.

### Three names Hydra must never set

`CLAUDE_CONFIG_DIR` loses the Anthropic login even when set to its own default value, because the
keychain service name gains a hash of the config dir the moment the variable is defined.
`AGY_ADC_AUTH` is the real auth switch for Google, not `GEMINI_API_KEY`, which cannot displace
`agy`'s keyring OAuth unless `settings.json` carries `modelProvider: "gemini"`. And `EDITOR` stays
off the list for `agy`, which spawns it from the artifact view, so a detached child would hang on an
editor nobody is watching.

### What this could not decide

Research decides nothing, and one finding needs a decision. The allowlist cannot gate spend on
hermes, because `~/.hermes/.env` loads with `override=True` at `hermes_cli/env_loader.py:214` and
beats anything Hydra passes in. D16 has no mechanism for that provider. Ticket 22 owns it.

**Corrected 2026-09-02.** The first version of this section said hermes resolves the billed AI
Studio key from `~/.hermes/auth.json`. It does not. The `gemini` entry in `credential_pool` carries
a 1-character `access_token`, ticket 08's junk manual entry, and `GOOGLE_API_KEY` in
`~/.hermes/.env` is declared with an empty value. Hermes advertises `gemini` auth because one junk
character passes a truthiness check, not because it holds a key. The override path is real and the
hole is latent. Nothing can be spent through hermes today.

One more correction to the never-pass list above: `HERMES_HOME` should not be on it. It redirects
both `auth.json` and `.env`, which makes it the one lever that gates this provider. Ticket 22
decides whether Hydra sets it.
