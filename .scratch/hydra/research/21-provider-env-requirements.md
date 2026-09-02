# 21 - What each provider reads from the environment, and what it needs

Ticket: [21 - What environment each provider needs, and what it leaks](../issues/21-provider-env-requirements.md)
Type: research. This file reports measurements. It proposes lists but decides nothing.

Same rule as research/08 and research/12: names, paths, service names, error strings and
mechanisms only. No values. Nothing was decrypted, no keychain password was read, no provider
was given a prompt, and no model call was made on any provider.

Measured 2026-09-02 on this machine.

## What this changes for ticket 12

Nine things. Six sharpen the allowlist, three contradict what the ticket assumed.

1. **`PATH`, `HOME`, `TERM`, `LANG` is not enough for `claude`.** It also needs `USER`. The
   keychain account name is `process.env.USER`, so with `USER` absent `claude auth status` returns
   `loggedIn: false, authMethod: "none"` and exits 0. Ticket 12's four-name starting list loses the
   Anthropic login on the first run.

2. **A wrong `HOME` silently logs out three of the four providers, and the mechanism is not theirs.**
   `security list-keychains` returns only `/Library/Keychains/System.keychain` when `HOME` points
   anywhere but the real home, so the login keychain leaves the search list and every keychain-backed
   provider comes up unauthenticated with exit code 0. An *absent* `HOME` is safer than a wrong one:
   with `HOME` unset the search list is correct again. `agy` is the exception, it refuses to start
   without `HOME` at all.

3. **`GEMINI_API_KEY` is not the most dangerous name on the list.** `CURSOR_AUTH_TOKEN` and
   `CURSOR_API_KEY` are write-through. `cursor-agent` calls `setAuthentication()` with the value it
   read from the environment, so an inherited value overwrites the stored keychain login rather than
   overriding it for one run. Scrubbing them protects the user's login, not only the secret.

4. **The allowlist cannot be a spend control for `hermes`.** Driven with a completely empty
   environment, `hermes acp` still advertises `authMethods[0].id = "gemini"`, and
   `acp_adapter/auth.py:11` only emits that when a credential string actually resolved. Hermes reads
   the billed AI Studio key out of `~/.hermes/auth.json`, where it persisted a byte-identical copy of
   `GEMINI_API_KEY` at seed time. Scrubbing the environment changes nothing about what hermes can
   spend. D16's gate has to sit somewhere else for this provider.

5. **The allowlist has to cover two axes beyond secrets.** Code injection: `NODE_OPTIONS=--require
   <file>` executes arbitrary JavaScript inside `cursor-agent`, confirmed by running it.
   `PYTHONPATH` with a `sitecustomize.py` executes arbitrary Python inside hermes when the venv
   binary is invoked directly, also confirmed. Permission bypass: `HERMES_YOLO_MODE` and
   `HERMES_ACCEPT_HOOKS` turn hermes' approval gates off from the environment.

6. **Session identity leaks even when no secret does.** This shell exports
   `CLAUDE_CODE_MESSAGING_SOCKET`, `CLAUDE_CODE_MESSAGING_TOKEN`, `CLAUDE_CODE_SESSION_ID`,
   `CLAUDE_CODE_BRIDGE_SESSION_ID` and `CLAUDE_CODE_CHILD_SESSION`. A child `claude` writes
   `messagingSocketPath` into its own pid-file record from that variable, so an unscrubbed child
   joins the parent's messaging plane. That is a correctness bug, not only a hygiene one.

7. **Ticket 12's biggest open hole is closed by `agy`'s own changelog.** `GEMINI_API_KEY` cannot
   displace `agy`'s keyring OAuth on its own. The 1.1.13 entry says the direct Gemini API route
   turns on only when `settings.json` carries `modelProvider: "gemini"`. What *can* displace it is
   `AGY_ADC_AUTH`, which is a confirmed read: the binary carries "To log out, please run
   `unset AGY_ADC_AUTH` in your terminal and restart the CLI."

8. **Setting `CLAUDE_CONFIG_DIR` loses the Anthropic login, even when it points at the default
   path.** The keychain service name gains an eight-hex-character hash of the config dir whenever
   that variable is defined, so the lookup misses. Hydra must not set it.

9. **`LANG` is not read by any of the four in its own code.** `claude`, `cursor-agent` and `hermes`
   read `TERM`; none reads `LANG` or `LC_ALL`. Keep `LANG` for libc and Python encoding if you like,
   but do not describe it as a provider requirement.

## Method

Static first, then a probe battery that never sends a prompt.

```
strings -n 4 ~/.local/share/claude/versions/2.1.257 > claude-strings.txt      # 445,542 lines
strings -n 4 ~/.local/bin/agy                        > agy-strings.txt        # 823,441 lines

# names, claude (Bun single-file binary, minified JS in the string table)
grep -ohE 'process\.env\.[A-Za-z_][A-Za-z0-9_]*' claude-strings.txt | sed 's/process\.env\.//' \
  | sort | uniq -c | sort -rn                                                 # 449 distinct
grep -ohE 'process\.env\[["'\''][A-Za-z_][A-Za-z0-9_]*["'\'']\]' claude-strings.txt            # 1 more

# names, cursor-agent (webpack chunks, plain files on disk)
D=~/.local/share/cursor-agent/versions/2026.08.31-4057e58
grep -ohE 'process\.env\.[A-Za-z_][A-Za-z0-9_]*' $D/*.js | sed 's/process\.env\.//' \
  | sort -u                                                                   # 190 distinct

# names, hermes (Python source)
# walks *.py outside venv/node_modules for os.environ.get / os.getenv / os.environ[...] /
# environ.get / os.environ.setdefault, counts and records the first three files per name
python3 pyenv.py ~/.hermes/hermes-agent                                       # 763 distinct
```

Reading long minified lines needs a windowing helper rather than `grep -o`, because the whole
bundle is one line and the `grep` on this machine hits a complexity limit on wide context patterns.
The helper is 15 lines: open the file, run a regex over each line, print `line[start-N : end+M]`
around each match, stop after a hit count. Rewrite it, do not look for it.

`agy` is a 180 MB Mach-O Go binary with a large Google-internal string table, so a name appearing
in `strings` proves nothing on its own. Research/12 already flagged that. Two things fix it here:
`agy changelog` prints the vendor's own prose about several of the variables, and the strings that
matter sit inside user-facing sentences ("unset AGY_ADC_AUTH env var") rather than alone.

The probe battery uses `env -i` plus explicit assignments, and only these commands:

```
claude --version | auth status --json          # keychain read, no network model call
cursor-agent --version | status --format json  # keychain read, account lookup, no model call
agy --help | models | changelog                # models fetches the catalogue, no inference
hermes --version                               # local
hermes acp  + one JSON-RPC "initialize" line   # handshake only, no session, no prompt
```

`claude auth status --json`, `cursor-agent status --format json` and the `hermes acp` handshake are
the three credential-reachability probes. Each reports whether the provider resolved a credential
without asking it to use one. `agy` has no such command, so `agy models` stands in: it needs a
working login and lists the catalogue without running a model.

## Cross-cutting: three mechanisms that apply to more than one provider

### The macOS keychain search list is a function of HOME

This explains most of the silent-logout behaviour below, and it is not any provider's code.

```
$ env -i PATH=/usr/bin:/bin HOME=/Users/USER /usr/bin/security list-keychains
    "/Users/USER/Library/Keychains/login.keychain-db"
    "/Library/Keychains/System.keychain"

$ env -i PATH=/usr/bin:/bin HOME=<scratch dir> /usr/bin/security list-keychains
    "/Library/Keychains/System.keychain"

$ env -i PATH=/usr/bin:/bin /usr/bin/security list-keychains          # HOME unset
    "/Users/USER/Library/Keychains/login.keychain-db"
    "/Library/Keychains/System.keychain"
```

A wrong `HOME` drops the login keychain. An unset `HOME` does not, because the search list falls
back to `getpwuid`. A direct item lookup confirms it, attributes only, no `-g`:

```
$ env -i PATH=/usr/bin:/bin HOME=/Users/USER security find-generic-password -s gemini -a antigravity
rc=0
$ env -i PATH=/usr/bin:/bin HOME=<scratch> security find-generic-password -s gemini -a antigravity
security: SecKeychainSearchCopyNext: The specified item could not be found in the keychain.
```

So for `claude`, `cursor-agent` and `agy` the rule is the same: pass the real `HOME` or pass none.
Never pass a different one. Hydra pointing a child at a sandbox home costs it every keychain login
on the machine, with no error to notice.

### Two providers execute code named by an environment variable

`cursor-agent` runs a real Node binary, so Node's own `NODE_OPTIONS` applies:

```
$ printf 'process.stderr.write("HYDRA-NODE-OPTIONS-INJECTION-RAN\\n");\n' > inject.js
$ env -i PATH=$PATH HOME=$HOME NODE_OPTIONS="--require $PWD/inject.js" cursor-agent --version
HYDRA-NODE-OPTIONS-INJECTION-RAN
2026.08.31-4057e58
```

`claude` is a Bun single-file binary and ignored the same variable, printing only its version.

`hermes` is Python, and its launcher defends against the equivalent. `~/.local/bin/hermes` is four
lines:

```bash
#!/usr/bin/env bash
unset PYTHONPATH
unset PYTHONHOME
exec "/Users/USER/.hermes/hermes-agent/venv/bin/hermes" "$@"
```

That scrub is load-bearing. Bypass the wrapper and the injection lands:

```
$ printf 'import sys\nsys.stderr.write("HYDRA-PY-SITECUSTOMIZE-RAN\\n")\n' > pyinj/sitecustomize.py
$ env -i PATH=$PATH HOME=$HOME PYTHONPATH=$PWD/pyinj ~/.local/bin/hermes --version         # 0 hits
$ env -i PATH=$PATH HOME=$HOME PYTHONPATH=$PWD/pyinj \
      ~/.hermes/hermes-agent/venv/bin/hermes --version                                     # 1 hit
```

`usercustomize.py` does not fire, because a venv sets `ENABLE_USER_SITE` false. `sitecustomize.py`
does. Hydra must launch `~/.local/bin/hermes` and not the venv entry point, and should still deny
`PYTHONPATH` and `PYTHONHOME` itself rather than trusting a wrapper it does not own.

### Three variables turn a provider's approval gate off

| variable | provider | effect | evidence |
|---|---|---|---|
| `HERMES_YOLO_MODE` | hermes | skips the dangerous-command approval prompt | `tools/approval.py:943` and `:1079`, `is_truthy_value(os.getenv("HERMES_YOLO_MODE"))` short-circuits both approval paths |
| `HERMES_ACCEPT_HOOKS` | hermes | auto-approves unseen shell hooks from `config.yaml` with no TTY prompt | `cli.py:867`; `--accept-hooks` is documented as equivalent |
| `IS_SANDBOX` | claude | with `IS_SANDBOX=1`, `--dangerously-skip-permissions` is allowed under uid 0 | binary: `process.getuid()===0 && process.env.IS_SANDBOX!=="1" && !CLAUDE_CODE_BUBBLEWRAP` guards the refusal |

Worth knowing for ticket 11 and 13: `hermes -z/--oneshot` sets both hermes variables itself.
`hermes_cli/oneshot.py:171-172` is literally `os.environ["HERMES_YOLO_MODE"] = "1"` and
`os.environ["HERMES_ACCEPT_HOOKS"] = "1"`. So the headless hermes path Hydra would use is
approval-free by construction, whatever the environment says. Scrubbing these two protects the
interactive path only.

`cursor-agent` and `agy` have no equivalent. Their bypasses are argv flags (`--force`, `--yolo`,
`--dangerously-skip-permissions`), which is the better design and the one D6 can reason about.

## claude 2.1.257

Binary: `~/.local/bin/claude` is a symlink to `~/.local/share/claude/versions/2.1.257`, a 199 MB
Mach-O arm64 Bun executable.

### What it reads

450 distinct names. Most belong to vendored SDKs (AWS, Azure Identity, Google auth, gRPC, OTel) and
to CI and terminal detection. The groups that matter are its own, and the binary conveniently
defines them as arrays.

Anthropic credential names, verbatim from the bundle as `BU`:

```js
BU=["ANTHROPIC_API_KEY","ANTHROPIC_AUTH_TOKEN","CLAUDE_CODE_OAUTH_TOKEN","AWS_BEARER_TOKEN_BEDROCK",
    "ANTHROPIC_FOUNDRY_API_KEY","ANTHROPIC_FOUNDRY_AUTH_TOKEN","ANTHROPIC_AWS_API_KEY"]
```

Claude Code's own token-bearing names, as `sZe` and `aZe`:

```js
sZe=["CLAUDE_CODE_OAUTH_TOKEN_FILE_DESCRIPTOR","CLAUDE_CODE_GATEWAY_TOKEN_FILE_DESCRIPTOR",
     "CLAUDE_CODE_API_KEY_FILE_DESCRIPTOR","CLAUDE_CODE_WEBSOCKET_AUTH_FILE_DESCRIPTOR"]
aZe=["CLAUDE_CODE_OAUTH_TOKEN",...sZe,"CLAUDE_CODE_ARTIFACTS_API_TOKEN","CLAUDE_CODE_SLACK_TAG_TOKEN",
     "CLAUDE_CODE_HFI_BEARER_TOKEN","CLAUDE_BRIDGE_OAUTH_TOKEN","CLAUDE_TRUSTED_DEVICE_TOKEN",
     "AGENT_PROXY_AUTH_TOKEN","CLAUDE_BG_AUTH_SNAPSHOT_PATH","CLAUDE_BG_SOCKET_TOKENS_PATH",
     "CLAUDE_BG_RV_AUTH","CLAUDE_BG_PTY_AUTH","CLAUDE_BG_CLAIM_AUTH"]
```

Third-party credential names it also treats as credentials, as `Xt`, `oUe` and `qt`:

```js
Xt=["AWS_ACCESS_KEY_ID","AWS_SECRET_ACCESS_KEY","AWS_SESSION_TOKEN"]
oUe=[...Xt,"AWS_PROFILE","AWS_CONFIG_FILE","AWS_SHARED_CREDENTIALS_FILE",
     "GOOGLE_APPLICATION_CREDENTIALS","GOOGLE_CLOUD_PROJECT"]
```

Path and config roots, from a 65-name array the settings layer uses: `CLAUDE_CONFIG_DIR`,
`CLAUDE_SECURESTORAGE_CONFIG_DIR`, `ANTHROPIC_CONFIG_DIR`, `HOME`, `XDG_CONFIG_HOME`,
`XDG_DATA_HOME`, `XDG_CACHE_HOME`, `XDG_STATE_HOME`, `XDG_RUNTIME_DIR`, `TMPDIR`, `TMP`, `TEMP`,
`CLAUDE_TMPDIR`, `CLAUDE_CODE_TMPDIR`, `CLAUDE_JOB_DIR`, `CLAUDE_CODE_MESSAGING_SOCKET`,
`CLAUDE_CODE_MESSAGING_TOKEN`, `USER_TYPE`, `GITHUB_ACTIONS`, `CLAUDE_CODE_SUBPROCESS_ENV_SCRUB`.

Worth naming for its own sake: Claude Code already ships a subprocess environment scrub, gated on
`CLAUDE_CODE_SUBPROCESS_ENV_SCRUB` or on `CLAUDE_CODE_ENTRYPOINT === "local-agent"`, plus an
`CLAUDE_CODE_MCP_ALLOWLIST_ENV` gate for MCP servers and `CLAUDE_CODE_DONT_INHERIT_ENV` for the
shell snapshot. It also carries a name heuristic for anything key-shaped:

```js
Gt=["TOKEN","SECRET","PASSWORD","PASSWD","PASSPHRASE","KEY","AUTH","COOKIE","PAT","DSN","WEBHOOK",
    "CREDENTIAL","CREDENTIALS","CREDS","APIKEY","ACCESSKEY","SECRETKEY","ACCOUNTKEY","PRIVATEKEY",
    "AUTHKEY","SSHKEY","SIGNINGKEY","MASTERKEY","DEPLOYKEY","ENCRYPTIONKEY","PGPASSWORD","SSHPASS"]
```

Ticket 19 wants a heuristic list at export time. This is a vendor-maintained one, already shipped.

### What it needs

Three variables, measured. `claude auth status --json` reports the resolved credential without
using it, so the table below is a direct read of "can this process reach its own login".

| environment | `loggedIn` | `authMethod` |
|---|---|---|
| the full parent environment | true | `claude.ai` |
| `env -i` | false | none |
| `PATH` + `HOME` | false | none |
| `PATH` + `HOME` + `USER=frank` | true | `claude.ai` |
| `PATH` + `HOME` + `LOGNAME=frank` | false | none |
| `PATH` + `HOME` + `USER=nobodyxyz` | false | none |
| `PATH` + `USER=frank`, no `HOME` | true | `claude.ai` |
| `PATH` + `HOME=/tmp` + `USER=frank` | false | none |
| `PATH=/nonexistent` + `HOME` + `USER` | false | none |
| `PATH=/usr/bin` + `HOME` + `USER` | true | `claude.ai` |
| no `PATH` at all, `HOME` + `USER` | true | `claude.ai` |
| `CLAUDE_CONFIG_DIR=/Users/USER/.claude` added to the working set | false | none |

The mechanism, from the bundle:

```js
import{homedir as o,userInfo as u}from"os";
var g5="-credentials";
function Ix(n=""){let e=process.env.CLAUDE_SECURESTORAGE_CONFIG_DIR,
  t=e!==void 0?!e:!process.env.CLAUDE_CONFIG_DIR,
  r=e!==void 0?e.normalize("NFC"):Se(),
  c=t?"":`-${a("sha256").update(r).digest("hex").substring(0,8)}`;
  return`Claude Code${Kt().OAUTH_FILE_SUFFIX}${n}${c}`}
var s=/^[a-zA-Z0-9._-]+$/;
function yv(){let n;try{n=process.env.USER||u().username}catch{n="claude-code-user"}
  if(!s.test(n))return"claude-code-user";return n}
...
R={name:"keychain",read(){ ... let n=Ix(g5),o=yv(),
  s=DQ(`security find-generic-password -a "${o}" -w -s "${n}"`,{timeout:2000}); ... }}
```

So the service name is `Claude Code-credentials` exactly, and only while neither `CLAUDE_CONFIG_DIR`
nor `CLAUDE_SECURESTORAGE_CONFIG_DIR` is defined. Define either and `c` becomes an eight-character
hash suffix and the lookup misses. That is why setting `CLAUDE_CONFIG_DIR` to its own default value
still logs the process out.

The account name is `process.env.USER`. The documented `u().username` fallback does not save it
here. Both halves are confirmed against the keychain directly:

```
$ security find-generic-password -a "frank"            -s "Claude Code-credentials"  ; rc=0
$ security find-generic-password -a "claude-code-user" -s "Claude Code-credentials"  ; rc=44
```

`USER=` set to the empty string behaves like unset, false in both cases. Why `os.userInfo()` does
not produce `frank` inside this Bun build is unresolved. It does not matter for the allowlist: pass
`USER`.

`PATH` matters only because the keychain read shells out to `security` by bare name. `/usr/bin` on
`PATH` is sufficient; `PATH` unset works because libc supplies a default; `PATH=/nonexistent` loses
the login with no error. Hydra needs a real `PATH` for the Bash tool anyway.

### What carries a secret

The seven names in `BU`, the thirteen in `aZe`, and the AWS trio in `Xt`. None of them is set on
this machine today. Two binary strings spell out the precedence Hydra has to respect: "Unset
ANTHROPIC_API_KEY and run /login to sign in with your claude.ai account", and the same sentence for
`apiKeyHelper`. `--bare` restates it from the other side: "Anthropic auth is strictly
ANTHROPIC_API_KEY or apiKeyHelper via --settings (OAuth and keychain are never read)".

### What happens when something is missing

Silent degrade, in every case measured. `claude auth status` exits 0 and reports
`loggedIn: false, authMethod: "none"` whether `USER` is absent, `HOME` is wrong, `PATH` cannot reach
`security`, or `CLAUDE_CONFIG_DIR` is set. No warning on stderr. A run started this way would fail
later at the API, and research/12 already collected that vocabulary: "401", "Invalid API key",
"Please run /login". Hydra's pre-flight check should call `claude auth status --json` rather than
infer anything from a process start.

### Proxy, CA and telemetry

Read, with both cases: `HTTP_PROXY`/`http_proxy`, `HTTPS_PROXY`/`https_proxy`, `NO_PROXY`/`no_proxy`,
`ALL_PROXY`/`all_proxy`, `grpc_proxy`, `no_grpc_proxy`. The git helper reinjects whatever it finds as
`-c http.proxy=`, so dropping the proxy variables breaks `git` inside a run on a proxied network too.

CA: `NODE_EXTRA_CA_CERTS` is read in three places and can also be *written* by claude from its own
settings. `SSL_CERT_FILE` appears once, in the pairing `if(process.env.HTTPS_PROXY &&
process.env.SSL_CERT_FILE)`, which is the branch that forwards a proxy environment to child CLIs
(`gcloud`, `bq`, `gsutil`). `NODE_TLS_REJECT_UNAUTHORIZED` and `NODE_USE_SYSTEM_CA` are present.

Telemetry: `CLAUDE_CODE_ENABLE_TELEMETRY`, `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC`,
`DISABLE_TELEMETRY`, `DISABLE_ERROR_REPORTING`, `DO_NOT_TRACK`, `DISABLE_AUTOUPDATER`, and the whole
`OTEL_*` family including `OTEL_EXPORTER_OTLP_HEADERS`, which carries a bearer token in normal use.
Claude Code is the only one of the four with real opt-out switches. If Hydra wants a quiet child,
these are the levers, and `OTEL_EXPORTER_OTLP_HEADERS` belongs on the deny list either way.

### Minimum viable environment

```
HOME=<the real home>          required, or omit; never a different path
USER=<the macOS account>      required
PATH=<must contain /usr/bin>  required for the keychain read and for every shell tool
TERM                          read 10 times; needed for the pty rendering path
SHELL                         read for shell classification; CLAUDE_CODE_SHELL overrides the executor
TMPDIR                        read; claude falls back on its own if absent
```

Pass through on purpose when the network needs it: `HTTPS_PROXY`, `HTTP_PROXY`, `NO_PROXY` and their
lowercase twins, `NODE_EXTRA_CA_CERTS`.

Never pass: everything in `BU`, `aZe` and `Xt`; `CLAUDE_CONFIG_DIR`;
`CLAUDE_SECURESTORAGE_CONFIG_DIR`; `ANTHROPIC_BASE_URL` and the six other `ANTHROPIC_*_BASE_URL`
endpoints with their `CLAUDE_CODE_USE_*` selectors and `CLAUDE_CODE_SKIP_*_AUTH` companions;
`ANTHROPIC_CUSTOM_HEADERS`; every `CLAUDE_CODE_*` and `CLAUDE_*` session variable this harness
exports; `OTEL_EXPORTER_OTLP_HEADERS`; `IS_SANDBOX`.

## cursor-agent 2026.08.31-4057e58

`~/.local/bin/cursor-agent` is a symlink to a 37-line bash launcher next to a bundled `node` and a
9.5 MB `index.js` with 70-odd webpack chunks beside it.

### What it reads

The launcher reads five, before any JavaScript runs:

```bash
export CURSOR_INVOKED_AS="$(basename "$0")"
if [ -z "${NODE_COMPILE_CACHE:-}" ] && [ -n "${HOME:-}" ]; then
  if [[ "${OSTYPE:-}" == darwin* ]]; then export NODE_COMPILE_CACHE="$HOME/Library/Caches/cursor-compile-cache"
  else export NODE_COMPILE_CACHE="${XDG_CACHE_HOME:-$HOME/.cache}/cursor-compile-cache"; fi
fi
should_skip_system_ca() { case "${AGENT_CLI_CREDENTIAL_STORE:-}" in file) return 0;; esac; return 1; }
if ! should_skip_system_ca && "$NODE_BIN" --use-system-ca --version >/dev/null 2>&1; then
  exec -a "$0" "$NODE_BIN" --use-system-ca "$SCRIPT_DIR/index.js" "$@"
fi
```

The bundles read 190 more. Auth-relevant: `CURSOR_API_KEY`, `CURSOR_AUTH_TOKEN`,
`CURSOR_API_ENDPOINT`, `CURSOR_API_BASE_URL`, `CURSOR_WEBSITE_URL`, `AGENT_CLI_CREDENTIAL_STORE`,
`CURSOR_ASKPASS_SOCKET`, `CURSOR_ASKPASS_SECRET`, `CURSOR_LOCAL_AGENT_HEADERS`,
`CURSOR_LOCAL_AGENT_ALLOW_CURSOR_HOST`, `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`,
`NO_OPEN_BROWSER`, `AGENT_CLI_BYPASS_AUTH_FOR_BOOT_SMOKE`, `CURSOR_AGENT_CLI_AUTHLESS_MODE`.
Config roots: `CURSOR_CONFIG_DIR`, `CURSOR_DATA_DIR`, `CURSOR_WORKTREES_ROOT`,
`CURSOR_SANDBOX_POLICY_DIR`. The rest is CI detection (three dozen `*_COMMIT_SHA` names), Sentry,
OTel and terminal detection.

### What it needs

Almost nothing, and this surprised me.

| environment | `status` | `isAuthenticated` |
|---|---|---|
| full parent environment | authenticated | true |
| `PATH` + `HOME` | authenticated | true |
| `PATH` only, no `HOME` | authenticated | true |
| `env -i`, nothing at all | authenticated | true |
| the resolved binary directly under `env -i` | authenticated | true |
| `PATH` + `HOME=/tmp` | unauthenticated | false |
| `PATH` + `HOME` + `AGENT_CLI_CREDENTIAL_STORE=file` | unauthenticated | false |
| `PATH` + `HOME` + `AGENT_CLI_CREDENTIAL_STORE=memory` | unauthenticated | false |
| `PATH=` (empty string) | `env: bash: No such file or directory` | rc 127 |

`env -i` works because bash supplies a compiled-in default `PATH` when the variable is unset, and
because the keychain items use the constant account `cursor-user` rather than the login name.
`PATH` set to an empty string is different and fails loudly, since the `#!/usr/bin/env bash` shebang
cannot resolve. So `PATH` must either contain a shell or be absent, never be empty.

`AGENT_CLI_CREDENTIAL_STORE` is a three-way switch and a trap:

```js
"./src/utils/credential-store.ts"(e,t,n){
  function r({env:e=process.env}={}){const t=e.AGENT_CLI_CREDENTIAL_STORE;
    return"file"===t?"file":"memory"===t?"memory":"default"}
```

`file` also suppresses `--use-system-ca` in the launcher, so an inherited value changes both the
credential source and the trust store in one move.

### What carries a secret, and the write-through problem

`CURSOR_API_KEY` and `CURSOR_AUTH_TOKEN` both carry one, and both are persisted rather than merely
used. From `./src/api-key-auth.ts`, exported as `T` and `D`:

```js
function l(e,n){                                            // T, the API-key path
  const l=n.apiKey??process.env.CURSOR_API_KEY, u=!n.apiKey&&!!process.env.CURSOR_API_KEY;
  if(!l)return{isAuthenticated:!1,usingApiKeyFromEnv:!1};
  ... h=yield m.loginWithApiKey(l,{endpoint:n.endpoint});
  if(h?.accessToken&&h.refreshToken){ const{accessToken:n,refreshToken:t}=h;
    yield a.jQ(()=>e.setAuthentication(n,t,l),"auth.apiKey.persistFailed"); ... }
  uQ(1,`⚠ Warning: The provided API key is invalid.`+(u?"The API key was loaded from the CURSOR_API_KEY environment variable.\n":"")...)}

function u(e,n){                                            // D, the auth-token path
  const r=n.authToken??process.env.CURSOR_AUTH_TOKEN, s=!n.authToken&&!!process.env.CURSOR_AUTH_TOKEN;
  return r?(o.Nw(r),yield a.jQ(()=>e.setAuthentication(r,r),"auth.authToken.persistFailed"),
            {isAuthenticated:!0,usingAuthTokenFromEnv:s}):{isAuthenticated:!1,usingAuthTokenFromEnv:!1}}
```

`setAuthentication` writes the credential store. So `CURSOR_AUTH_TOKEN` overwrites the stored
access and refresh tokens with the environment value unconditionally, before any validation. This
was not tested, deliberately: proving it means clobbering the user's real login.

Boot order, from the main chunk, is auth token first, API key second, stored login third:

```js
const e=yield c.D(C,{authToken:o.authToken}); Ee=e.usingAuthTokenFromEnv;
const t=e.isAuthenticated?{isAuthenticated:!1,usingApiKeyFromEnv:!1}
                         :yield c.T(C,{apiKey:o.apiKey,endpoint:me});
if(Ie=t.usingApiKeyFromEnv,!(Se||Me||e.isAuthenticated||t.isAuthenticated||(yield j.h4(C,j.hT.LOGIN)))){ ... }
```

Two more secret-adjacent pairs. `CURSOR_ASKPASS_SOCKET` and `CURSOR_ASKPASS_SECRET` are a sudo
askpass capability: `cursor-askpass.js` refuses to run unless its parent is `sudo` or `doas`, setuid
and root-owned, then uses the secret to talk to the socket. `CURSOR_LOCAL_AGENT_HEADERS` is parsed
as newline-separated `Name: Value` pairs and attached to agent requests, so it can inject an
`Authorization` header. `AWS_ACCESS_KEY_ID` and `AWS_SECRET_ACCESS_KEY` are read only inside
`cursor-agent bedrock` when `--from-env` is passed, not on the normal path.

### What happens when something is missing

Two shapes, both clear for once. No credential at all, in print mode:

```
Error: Authentication required. Please run 'cursor-agent login' first, or set CURSOR_API_KEY environment variable.
Authentication required to use Cursor Agent. Please run 'cursor-agent login' to authenticate.
Error: Authentication required. Run 'cursor-agent login', pass --api-key/--auth-token, or set CURSOR_API_KEY/CURSOR_AUTH_TOKEN.
```

An invalid key exits 1 and names the environment variable it came from. But a *wrong* `HOME` or an
inherited `AGENT_CLI_CREDENTIAL_STORE` degrades silently: `status` exits 0 and says
`"unauthenticated"`.

### Proxy, CA and telemetry

`HTTPS_PROXY`/`https_proxy`, `HTTP_PROXY`/`http_proxy`, `NO_PROXY`/`no_proxy` and
`GLOBAL_AGENT_ENVIRONMENT_VARIABLE_NAMESPACE` are read. `NODE_TLS_REJECT_UNAUTHORIZED` appears 15
times, more than anything except `CURSOR_API_KEY` itself, which is worth a raised eyebrow.

CA handling is in the launcher, not the bundle: `node --use-system-ca` trusts the macOS system
roots. `NODE_EXTRA_CA_CERTS` is not referenced anywhere in the JavaScript, but Node honours it in
the runtime regardless, so it is still the variable to pass on a TLS-inspecting network.

There is no telemetry opt-out variable. The `SENTRY_*` names are the Sentry SDK's own defaults and
`CURSOR_STATSIG_OVERRIDES` is a hidden dev flag.

### Minimum viable environment

```
PATH=<must resolve bash; may be unset, must not be empty>
HOME=<the real home>     or omit; needed for ~/.cursor and for the keychain search list
TERM                     read 4 times, for rendering
```

Pass through on purpose: `HTTPS_PROXY`, `HTTP_PROXY`, `NO_PROXY` and lowercase,
`NODE_EXTRA_CA_CERTS`.

Never pass: `CURSOR_API_KEY`, `CURSOR_AUTH_TOKEN`, `AGENT_CLI_CREDENTIAL_STORE`, `NODE_OPTIONS`,
`NODE_TLS_REJECT_UNAUTHORIZED`, `CURSOR_API_ENDPOINT`, `CURSOR_API_BASE_URL`,
`CURSOR_LOCAL_AGENT_HEADERS`, `CURSOR_ASKPASS_SOCKET`, `CURSOR_ASKPASS_SECRET`, `CURSOR_CONFIG_DIR`,
`CURSOR_DATA_DIR`, `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`.

## agy 1.1.23

A 180 MB Mach-O arm64 Go binary, Google's `google3/third_party/jetski` tree.

### What it reads

Static enumeration is unreliable here and I am labelling it as such. What the string table offers,
with the qualification research/12 already made, is a list of candidates:

```
AGY_ADC_AUTH  AGY_BROWSER_WS_URL  AGY_BROWSER_ACTIVE_PORT_FILE  AGY_BUSINESS_PAYGO_TIER
AGY_CLI_CMD_OUTPUT_PERCENTAGE  AGY_CLI_DISABLE_AUTO_UPDATE  AGY_CLI_DISABLE_ESCAPE_SEQUENCE_OPTIMIZATIONS
AGY_CLI_DISABLE_INPUT_MODE_REASSERT  AGY_CLI_DISABLE_LATEX  AGY_CLI_DISABLE_MERMAID_ASCII
AGY_CLI_EXPERIMENTAL_RENDERING  AGY_CLI_FORCE_OSC8  AGY_CLI_HIDE_ACCOUNT_INFO  AGY_CLI_HIDE_LOGO
AGY_CLI_INTERACTIVE_HEADLESS  AGY_CLI_LOGO_STYLE  AGY_CLI_NEW_HARNESS  AGY_ENABLE_HUB
GEMINI_API_KEY  GOOGLE_API_KEY  GOOGLE_GEMINI_BASE_URL  GOOGLE_APPLICATION_CREDENTIALS
GOOGLE_CLOUD_PROJECT  GOOGLE_LOG_DIR  GOOGLE_LOGTOSTDERR  GOOGLE_ALSOLOGTOSTDERR  GOOGLE_STDERRTHRESHOLD
JETSKI_ALLOW_SANDBOX  JETSKI_APP_DATA_DIR  JETSKI_OAUTH_TOKEN  JETSKI_TEST_GAIA_TOKEN
JETSKI_BROWSER_HEADLESS  JETSKI_BROWSER_PORT  JETSKI_BROWSER_USER_DATA_DIR  JETSKI_BROWSER_NETLOG_ENABLED
JETSKI_GRPC_MAX_MESSAGE_SIZE  EDITOR  HTTP_PROXY  HTTPS_PROXY  NO_PROXY (and lowercase)
```

Names in that list are normalised. The Go linker packs string literals end to end, so `strings`
returns `AGY_CLI_HIDE_ACCOUNT_INFOT`, `AGY_CLI_DISABLE_AUTO_UPDATEF` and
`AGY_CLI_FORCE_OSC8CLI_CLIPBOARD_FILES` with the next literal's first characters attached. The
trailing letters are not part of the names.

`GEMINI_CLI_HOME` is **not** in the binary, zero occurrences. That variable belongs to Google's
`gemini` CLI, which research/12 covered, and ticket 12's answer names it in the right place. It has
no effect on `agy`.

Five of these are confirmed reads rather than candidates, because `agy` documents them itself.
`agy changelog` prints the vendor's release notes out of the binary:

> 1.1.13: Added support for `GEMINI_API_KEY`, so the CLI can run against the Gemini API directly
> without signing in. Set `modelProvider: "gemini"` in `settings.json`, export `GEMINI_API_KEY`, and
> point `GOOGLE_GEMINI_BASE_URL` at a custom endpoint if you need one. The banner and `/help` show
> `Gemini API key` as the credential, and `/logout` explains that it comes from the environment
> rather than appearing to end a session.

> 1.1.19: Added the `AGY_CLI_HIDE_LOGO` environment variable ... Added
> `AGY_CLI_DISABLE_ESCAPE_SEQUENCE_OPTIMIZATIONS` to bypass the renderer's dirty-rectangle and
> diffing optimizations.

> 1.0.15: Added `ctrl+g` on the artifact view to open $EDITOR.

> 1.0.4: Set `AGY_CLI_DISABLE_LATEX` environment variable to turn off LaTeX rendering globally.

> 1.0.2: Added `AGY_CLI_HIDE_ACCOUNT_INFO` environment variable to hide email and plan tier from the
> header.

That first entry answers the question research/12 listed as its biggest hole. An inherited
`GEMINI_API_KEY` cannot displace `agy`'s keyring OAuth by itself, because the direct Gemini route is
gated behind a `settings.json` key. `~/.gemini/antigravity-cli/settings.json` on this machine holds
only `trustedWorkspaces`, no `modelProvider`, so the gate is shut. I did not test the positive case,
because opening the gate is exactly the run D16 forbids.

`AGY_ADC_AUTH` is confirmed a different way, by user-facing sentences in the binary:

```
To log out, please run `unset AGY_ADC_AUTH` in your terminal and restart the CLI.
  3. Or unset AGY_ADC_AUTH env var to use other sign in methods.
  2. Or check GOOGLE_APPLICATION_CREDENTIALS env var.
```

Read together: when `AGY_ADC_AUTH` is set, `agy` authenticates by Application Default Credentials,
that choice outranks the keyring, and the only way back is to unset the variable. This is a real
auth-plane switch reachable from the environment, and it is the `agy` equivalent of
`ANTHROPIC_API_KEY` overriding a claude.ai login. Setting it with a scratch `HOME` produced no
observable difference here, because both runs failed at the same earlier step, so the behaviour is
read from the strings rather than watched.

Two more error strings name variables that are read:

```
GEMINI_API_KEY environment variable not set and no API key override provided
LLM Monitor: GEMINI_API_KEY environment variable not set; client will rely on per-monitor API key overrides
```

### What it needs

One variable, and it fails loudly without it.

| environment | result |
|---|---|
| full parent environment | rc 0, catalogue printed |
| `env -i` | rc 1, `$HOME is not defined` |
| `PATH` only | rc 1, `$HOME is not defined` |
| `HOME` only, no `PATH` | rc 0, catalogue printed |
| `PATH=/nonexistent` + `HOME` | rc 0, catalogue printed |
| `PATH` + `HOME=<scratch>` | rc 1, `Error: Please sign in to view available models.` |
| `PATH` + `HOME=<scratch with .gemini symlinked to the real one>` | rc 1, same |
| `agy --help` under `env -i` | rc 0 |

The no-`HOME` failure is six error lines and a Go stack trace, which is the most useful failure of
the four providers:

```
E main.go:623] Failed to redirect output for CLI: resolving log directory: getting home directory: $HOME is not defined
E main.go:178] failed to resolve CLI settings path for model backend selection: getting home directory: $HOME is not defined
E server.go:1470] Failed to get AppDataDir for crash reporter: $HOME is not defined
W server.go:410]  failed to load secplugins config at startup: $HOME is not defined
E main.go:779] CLI failed to start - $HOME is not defined
  | google3/third_party/jetski/fs/local/local.GeminiDir
  |     third_party/jetski/fs/local/local.go:28
```

`PATH` is irrelevant to `agy`'s auth because `zalando/go-keyring` calls `/usr/bin/security` by
absolute path. Symlinking `.gemini` into a scratch home does not restore the login, which fits the
keychain search list explanation above rather than a config-file one.

### What carries a secret

`GEMINI_API_KEY`, `GOOGLE_API_KEY`, `GOOGLE_APPLICATION_CREDENTIALS` (a path to one),
`JETSKI_OAUTH_TOKEN`, `JETSKI_TEST_GAIA_TOKEN`. `AGY_ADC_AUTH` carries no secret itself but selects
the plane that uses one. `ANTIGRAVITY_CSRF_TOKEN` and `ANTIGRAVITY_SIDECAR_UI_TOKEN` appear in the
string table; they read as the IDE sidecar's local IPC, not as provider credentials, and I did not
confirm that.

### What happens when something is missing

`HOME` absent, hard failure with a stack trace, rc 1. Credential unreachable, one clean line and
rc 1: `Error: Please sign in to view available models. Launch the CLI without arguments to sign in.`
Both are better than the other three providers manage. Research/12's finding that `agy` carries a
typed `AuthResult_HeadlessAuthRequired` state is consistent with this: `agy` treats a missing login
as a first-class outcome rather than an improvised prompt.

### Proxy, CA and telemetry

`HTTP_PROXY`, `HTTPS_PROXY`, `NO_PROXY` and the lowercase forms are in the binary, which is what
`golang.org/x/net/http/httpproxy` puts there; Go's default transport reads them once and caches the
result. `SSL_CERT_FILE` and `SSL_CERT_DIR` are also present, but they sit adjacent to `AWS_CA_BUNDLE`
in the string blob, which points at a vendored AWS SDK rather than at `agy`'s own TLS setup, and Go's
`crypto/x509` does not consult them on darwin. Treat `SSL_CERT_FILE` as unlikely to work here.

`GOOGLE_LOG_DIR`, `GOOGLE_LOGTOSTDERR`, `GOOGLE_ALSOLOGTOSTDERR` and `GOOGLE_STDERRTHRESHOLD` are
glog's. They redirect `agy`'s log output, which matters for ticket 09: an inherited
`GOOGLE_LOGTOSTDERR` would push Google's log lines into the stream Hydra tees. There is no telemetry
opt-out variable.

### Minimum viable environment

```
HOME=<the real home>    required, hard error without it, silent logout with the wrong one
TERM                    for the TUI; print mode was not tested
PATH                    not needed for auth, needed for agy's own shell and codesearch tools
```

Pass through on purpose: `HTTPS_PROXY`, `HTTP_PROXY`, `NO_PROXY` and lowercase.

Never pass: `AGY_ADC_AUTH`, `GEMINI_API_KEY`, `GOOGLE_API_KEY`, `GOOGLE_APPLICATION_CREDENTIALS`,
`GOOGLE_GEMINI_BASE_URL`, `GOOGLE_CLOUD_PROJECT`, `JETSKI_OAUTH_TOKEN`, `JETSKI_TEST_GAIA_TOKEN`,
`JETSKI_APP_DATA_DIR`, `GOOGLE_LOGTOSTDERR`, `GOOGLE_ALSOLOGTOSTDERR`, `GOOGLE_LOG_DIR`, `EDITOR`.

`EDITOR` is on that list for a reason. `agy` spawns it from the artifact view on `ctrl+g`, and a
detached delegated child that opens an editor on a pty nobody is watching hangs.

## hermes 0.14.0

Not on `~/.local/bin` as a program. `~/.local/bin/hermes` is the four-line bash wrapper quoted
above; the code is at `~/.hermes/hermes-agent/`, 1,833 Python files outside the venv, running on the
venv's Python 3.11.15.

### What it reads

763 distinct names, by far the most of the four, and 194 of them are `HERMES_*`. Thirty-four are
provider API keys, from `hermes_cli/auth.py`:

```
AI_GATEWAY_API_KEY  ALIBABA_CODING_PLAN_API_KEY  ANTHROPIC_API_KEY  ANTHROPIC_TOKEN
ARCEEAI_API_KEY  AZURE_FOUNDRY_API_KEY  CLAUDE_CODE_OAUTH_TOKEN  COPILOT_GITHUB_TOKEN
DASHSCOPE_API_KEY  DEEPSEEK_API_KEY  GEMINI_API_KEY  GH_TOKEN  GITHUB_TOKEN  GLM_API_KEY
GMI_API_KEY  GOOGLE_API_KEY  HF_TOKEN  KILOCODE_API_KEY  KIMI_API_KEY  KIMI_CN_API_KEY
KIMI_CODING_API_KEY  LM_API_KEY  MINIMAX_API_KEY  MINIMAX_CN_API_KEY  NVIDIA_API_KEY
OLLAMA_API_KEY  OPENCODE_GO_API_KEY  OPENCODE_ZEN_API_KEY  STEPFUN_API_KEY  TOKENHUB_API_KEY
XAI_API_KEY  XIAOMI_API_KEY  ZAI_API_KEY  Z_AI_API_KEY
```

Plus `OPENROUTER_API_KEY` from `hermes_cli/main.py:2916`, `OPENAI_API_KEY` and `AZURE_ANTHROPIC_KEY`
from the runtime resolver, and `NOUS_API_KEY` and `QWEN_API_KEY` from the provider plugins. Anything
seeded from one of these lands in the pool as `source: "env:<VAR>"` (`credential_pool.py:1818`).

Beyond inference there are another forty-odd credentials for messaging platforms, memory backends,
search providers and sandboxes: `DISCORD_BOT_TOKEN`, `TELEGRAM_BOT_TOKEN`, `MATRIX_ACCESS_TOKEN`,
`MATRIX_PASSWORD`, `MATRIX_RECOVERY_KEY`, `SLACK_*`, `TWILIO_ACCOUNT_SID`, `TWILIO_AUTH_TOKEN`,
`EMAIL_PASSWORD`, `HASS_TOKEN`, `LINE_CHANNEL_SECRET`, `TEAMS_CLIENT_SECRET`, `FEISHU_APP_SECRET`,
`DAYTONA_API_KEY`, `VERCEL_TOKEN`, `MODAL_TOKEN_SECRET`, `SUDO_PASSWORD`, `TERMINAL_SSH_KEY`,
`API_SERVER_KEY`, `BROWSERBASE_API_KEY`, `EXA_API_KEY`, `TAVILY_API_KEY`, `FIRECRAWL_API_KEY`,
`LINEAR_API_KEY`, `NOTION_API_KEY`. A blanket rule beats an enumeration here.

### What it needs

Nothing. That is the finding.

| environment | `authMethods` from the ACP `initialize` handshake |
|---|---|
| `PATH HOME USER TERM LANG SHELL` | `['gemini', 'hermes-setup']` |
| `PATH` + `HOME` | `['gemini', 'hermes-setup']` |
| `PATH` only | `['gemini', 'hermes-setup']` |
| `env -i`, nothing at all | `['gemini', 'hermes-setup']` |
| `PATH` + `HOME=<scratch>` | `['hermes-setup']` |
| `PATH` + `HOME` + `HERMES_HOME=<scratch>/.hermes` | `['hermes-setup']` |

`hermes-setup` is always advertised. The `gemini` entry appears only when a credential resolved,
and `acp_adapter/auth.py:11` `detect_provider()` proves it:

```python
runtime = resolve_runtime_provider()
api_key = runtime.get("api_key"); provider = runtime.get("provider")
...
is_string_key = isinstance(api_key, str) and api_key.strip()
if is_string_key or is_callable_provider:
    return provider.strip().lower()
```

So under a completely empty environment hermes still resolved a non-empty API key string, from
`~/.hermes/auth.json` where research/12 found it byte-identical to the shell's `GEMINI_API_KEY`.
`hermes --version` works under `env -i` too.

Two reasons the environment cannot control hermes. First, `hermes_cli/env_loader.py:213` loads
`~/.hermes/.env` with `override=True`, so that file beats anything Hydra passes:

```python
if user_env.exists():
    _load_dotenv_with_fallback(user_env, override=True)
```

Second, `_apply_external_secret_sources()` runs right after and can pull further secrets from
Bitwarden Secrets Manager into `os.environ` before any credential lookup happens.

One relief: the "project" `.env` is `Path(__file__).parent / '.env'`, the hermes install directory,
not the working directory. `~/.hermes/hermes-agent/.env` does not exist. A repository `.env` in the
directory an agent is working in is not read.

Config beats environment for the provider choice, `runtime_provider.py:426`:

```python
def resolve_requested_provider(requested=None):
    if requested and requested.strip(): return requested.strip().lower()
    model_cfg = _get_model_config(); cfg_provider = model_cfg.get("provider")
    if isinstance(cfg_provider, str) and cfg_provider.strip(): return cfg_provider.strip().lower()
    env_provider = os.getenv("HERMES_INFERENCE_PROVIDER", "").strip().lower()
    if env_provider: return env_provider
    return "auto"
```

`config.yaml` pins `gemini`, so an inherited `HERMES_INFERENCE_PROVIDER` cannot flip it here.

### What happens when something is missing

Silent degrade, and it degrades in the direction that matters least. A wrong `HOME` or a redirected
`HERMES_HOME` costs hermes its credential and it says so only by dropping the `gemini` entry from
`authMethods`. Exit code 0, no warning. Nothing in the environment can cost hermes its *billed*
credential, because that one is on disk.

### Proxy, CA and telemetry

`SSL_CERT_FILE` is read in three places and *written* by `gateway/run.py:597` when the system does
not expose CA certs to Python, falling back to `certifi.where()`. `agent/model_metadata.py:38`
checks `HERMES_CA_BUNDLE`, `REQUESTS_CA_BUNDLE` and `SSL_CERT_FILE` in that order, with a comment
explaining that `requests` and the stdlib `ssl` module read different names. `HTTPS_PROXY`,
`NO_PROXY` and `ALL_PROXY` are read in the gateway platform adapters, and httpx `trust_env=True`
picks up the standard names underneath.

`PYTHONIOENCODING` and `PYTHONUTF8` are set by hermes itself, in `hermes_bootstrap.py` and
`hermes_cli/stdio.py`. `NO_COLOR` and `TERM` are read for colouring. There is no telemetry variable.
Hermes does check for updates on every start and prints the result to stderr, seen live as
"Update available: 172 commits behind", which is a network call Hydra pays for on every launch.

### Minimum viable environment

```
(nothing is required)
HOME=<the real home>    or omit; a wrong one costs the credential silently
PATH                    needed for gh, rg, node, ffmpeg and every terminal tool hermes shells out to
TERM                    read for colouring
```

Pass through on purpose: `HTTPS_PROXY`, `HTTP_PROXY`, `NO_PROXY`, `SSL_CERT_FILE`,
`REQUESTS_CA_BUNDLE`.

Never pass: the 34 `api_key_env_vars` above plus `OPENROUTER_API_KEY`, `OPENAI_API_KEY`,
`AZURE_ANTHROPIC_KEY`, `NOUS_API_KEY` and `QWEN_API_KEY`; `HERMES_HOME`; `HERMES_YOLO_MODE`;
`HERMES_ACCEPT_HOOKS`; `HERMES_EXEC_ASK`; `PYTHONPATH`; `PYTHONHOME`; and, as a rule rather than a
list, any name matching Claude Code's own `Gt` heuristic.

## The consolidated lists

### The common allowlist, and why each name is on it

| name | claude | cursor-agent | agy | hermes |
|---|---|---|---|---|
| `HOME` | required or absent | required or absent | required | required or absent |
| `USER` | **required** | no | no | no |
| `PATH` | must reach `/usr/bin` | must resolve `bash` | tools only | tools only |
| `TERM` | read | read | probably | read |
| `LANG` | not read | not read | unclear | not read |

`LANG` earns its place only as a libc and Python encoding lever, not as anything a provider asks
for. Keeping it is harmless. Calling it a requirement is wrong.

### Pass through on purpose, not by accident

`HTTPS_PROXY`, `HTTP_PROXY`, `NO_PROXY` and the three lowercase twins reach all four. `ALL_PROXY`
and `all_proxy` reach claude and hermes. `NODE_EXTRA_CA_CERTS` reaches the two Node-based providers.
`SSL_CERT_FILE` and `REQUESTS_CA_BUNDLE` reach hermes. `agy` is the odd one out on CA bundles: Go
on darwin uses the system trust store and there is no environment lever.

If Hydra passes a proxy it should pass the CA bundle with it. Claude's own code makes the same
assumption, forwarding a proxy environment to child CLIs only in the branch where `HTTPS_PROXY` and
`SSL_CERT_FILE` are both set.

### The deny list, grouped by what the name does

1. **Credentials.** The provider lists above, roughly 60 names. Backstop with the `Gt` substring
   heuristic rather than trusting the enumeration to stay complete.
2. **Endpoint redirection.** `ANTHROPIC_BASE_URL` and its six siblings, `CURSOR_API_ENDPOINT`,
   `CURSOR_API_BASE_URL`, `GOOGLE_GEMINI_BASE_URL`, `OPENAI_BASE_URL`, `GEMINI_BASE_URL`. Redirecting
   a provider at another host is exfiltration with a different spelling.
3. **Auth-plane switches.** `AGY_ADC_AUTH`, `AGENT_CLI_CREDENTIAL_STORE`, `CLAUDE_CODE_USE_BEDROCK`
   and the other five `CLAUDE_CODE_USE_*`, the six `CLAUDE_CODE_SKIP_*_AUTH`.
4. **Config-root redirection.** `CLAUDE_CONFIG_DIR`, `CLAUDE_SECURESTORAGE_CONFIG_DIR`,
   `ANTHROPIC_CONFIG_DIR`, `CURSOR_CONFIG_DIR`, `CURSOR_DATA_DIR`, `HERMES_HOME`,
   `JETSKI_APP_DATA_DIR`. Each one silently costs the provider its login.
5. **Code injection.** `NODE_OPTIONS`, `PYTHONPATH`, `PYTHONHOME`, `PYTHONSTARTUP`,
   `DYLD_INSERT_LIBRARIES`, `LD_PRELOAD`.
6. **Approval bypass.** `HERMES_YOLO_MODE`, `HERMES_ACCEPT_HOOKS`, `HERMES_EXEC_ASK`, `IS_SANDBOX`.
7. **Header and TLS injection.** `ANTHROPIC_CUSTOM_HEADERS`, `CURSOR_LOCAL_AGENT_HEADERS`,
   `OTEL_EXPORTER_OTLP_HEADERS`, `NODE_TLS_REJECT_UNAUTHORIZED`.
8. **Session identity.** Every `CLAUDE_CODE_*` and `CLAUDE_*` name this harness exports, plus
   `CLAUDECODE` and `AI_AGENT`. `CLAUDE_CODE_MESSAGING_SOCKET` and `CLAUDE_CODE_MESSAGING_TOKEN` are
   the sharp ones: a child inherits the parent's messaging plane. `CURSOR_INVOKED_AS` needs no rule,
   because cursor's launcher overwrites it on line 3 of the script.
9. **Log redirection.** `GOOGLE_LOG_DIR`, `GOOGLE_LOGTOSTDERR`, `GOOGLE_ALSOLOGTOSTDERR`,
   `GOOGLE_STDERRTHRESHOLD`, `SELF_HOSTED_RUNNER_LOG_FILE`, `AUTOMODE_DECISION_LOG`. These change
   what lands in the stream ticket 09 records.
10. **Editor spawn.** `EDITOR`, `GIT_EDITOR`, `VISUAL`. A detached child that opens an editor hangs.

An allowlist makes most of this list unnecessary by construction. It is written down anyway, because
the allowlist will grow under pressure from the first provider that will not start, and these are
the names that must never be added to it when it does.

## Confirmed against inferred

Confirmed by running something:

- `claude` needs `USER`, and the account it looks up is `frank` while `claude-code-user` does not
  exist in the keychain. Twelve environment variations plus two direct `security` lookups.
- Setting `CLAUDE_CONFIG_DIR`, even to `/Users/USER/.claude`, loses the login.
- A wrong `HOME` removes the login keychain from `security list-keychains`; an unset `HOME` does not.
- `cursor-agent` reaches its credential under `env -i` and loses it under
  `AGENT_CLI_CREDENTIAL_STORE=file` or `=memory` or a wrong `HOME`.
- `cursor-agent` executes `NODE_OPTIONS=--require <file>`; `claude` ignores the same variable.
- `hermes` executes `PYTHONPATH` `sitecustomize.py` when the venv binary is called directly, and does
  not through `~/.local/bin/hermes`.
- `hermes` resolves a real API key with an empty environment, and loses it only when `HOME` or
  `HERMES_HOME` moves.
- `agy` refuses to start without `HOME` and prints a stack trace naming `local.go:28`.
- `agy` needs no `PATH` to reach its keyring credential.
- `claude auth status` did not write `~/.claude.json`: the SHA-256 prefix and mtime are unchanged
  across all probes.

Confirmed by reading source or a vendor document, not by watching behaviour:

- `cursor-agent` persists `CURSOR_AUTH_TOKEN` and `CURSOR_API_KEY` into the credential store via
  `setAuthentication`. Read from `./src/api-key-auth.ts` in the bundle. Deliberately not tested.
- `agy`'s `GEMINI_API_KEY` path requires `modelProvider: "gemini"` in `settings.json`. From `agy`'s
  own 1.1.13 changelog, printed by the binary.
- `AGY_ADC_AUTH` selects Application Default Credentials and outranks other sign-in methods. From
  three user-facing strings in the binary.
- `HERMES_YOLO_MODE` and `HERMES_ACCEPT_HOOKS` bypass approvals. From `tools/approval.py` and
  `cli.py`.
- `~/.hermes/.env` overrides the process environment. From `env_loader.py:213`, `override=True`.
- `IS_SANDBOX=1` lifts claude's root guard on `--dangerously-skip-permissions`.

Inference, labelled:

- **`TERM` is needed for the pty path.** Every provider reads it, none was tested without a terminal,
  and none was run interactively.
- **What each provider needs to run a task, as opposed to start and authenticate.** No task was run
  on any provider. `PATH`, `SHELL` and `TMPDIR` are the obvious candidates from the tool code, and
  claude's shell-snapshot spawn sets `SHELL` and `GIT_EDITOR` itself.
- **`agy`'s `SSL_CERT_FILE`.** The literal is in the binary next to `AWS_CA_BUNDLE`, which suggests a
  vendored AWS SDK rather than `agy`'s own TLS. Go's `crypto/x509` ignores it on darwin.
- **`NODE_EXTRA_CA_CERTS` actually working for `cursor-agent`.** Pointing it at a missing file
  produced no warning on `--version`, because Node validates lazily on first TLS use. The bundle
  never references the name, so this rests on Node's documented behaviour alone.
- **The `ANTIGRAVITY_*` names.** Most of that list is protocol-buffer enum names, not environment
  variables. Only the five the changelog documents should be treated as real.
- **Why `os.userInfo()` does not rescue `claude` when `USER` is unset.** The fallback is in the
  source and did not fire. The cause is unresolved and does not change the allowlist.

Not attempted, and why:

- Any task run on any provider. Money and quota, per D16 and this ticket's own limits.
- `CURSOR_AUTH_TOKEN` with a value. Proving the write-through means overwriting the real login.
- `agy` with `modelProvider: "gemini"` and a key. That is the billed path.
- `hermes` with a prompt. Its configured provider is `gemini` on the billed key.
- Whether an inherited variable survives into a *delegated* child. Ticket 11 owns the spawn path and
  there is no spawn path yet to test.

## Side effects of this pass

Disclosed because the probes were not free.

- `agy models` ran seven times against the live catalogue. No inference, no spend. The keychain item
  `svce=gemini, acct=antigravity` has `mdat` 2026-09-02 19:36:43 UTC, which falls inside that window,
  so `agy` refreshed its stored OAuth token. Nothing under `~/.gemini` changed on disk in the hour.
- `agy` created a throwaway config tree under the session scratch directory during the wrong-`HOME`
  tests: `.gemini/config/`, `.gemini/antigravity-cli/` with an empty conversation database, and
  `Library/Caches/`. It is in the scratch directory and goes away with the session.
- `hermes acp` was started six times and killed after one JSON-RPC line each. No session, no prompt,
  no model call. It writes a handful of INFO lines to stderr per start and whatever its logger writes;
  `~/.hermes/logs` shows no file modified in the last hour.
- `claude auth status --json` ran roughly twenty times. `~/.claude.json` is byte-identical by
  SHA-256 prefix and mtime, and the `Claude Code-credentials` keychain item's `mdat` predates the
  probes.
- `cursor-agent status --format json` ran nine times. Nothing under `~/.cursor` was modified in the
  hour.
- `security find-generic-password` was called with attributes only. No `-g`, no `-w` against a real
  item except once with output discarded to a null device to test a return code.

## Secret-material check

`grep -inE "sk-|AIza|gh[pousr]_|github_pat_|ya29|eyJ|-----BEGIN"` over this file matches only the
type-name literals quoted from provider source. The claude precedence probes were run with invented
placeholder strings that were never valid credentials anywhere; those strings are not reproduced
here. No value, fragment, prefix or length of any live credential appears in this document.
