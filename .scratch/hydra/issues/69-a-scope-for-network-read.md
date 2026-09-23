# 69 - Does `network_read` get a scope, and who writes the domain list?

Type: grilling
Status: resolved
Blocked by:
Opened: 2026-09-22, from map ticket 68's answer 4.

## Question

`network_read` carries no scope in either policy that grants it (`policies/standard.json`,
`policies/permissive.json`), and nothing in the repository lists a domain. Under `standard` a run
holds the three legs of the lethal trifecta: it reads untrusted text, it reads private data (the
checkout, and by shell the home directory, `SECURITY.md:32`), and it sends a request anywhere. The
first two are what a coding agent is for. The third is the one a policy can take away. Build ticket
77 makes the floor's Slack rules fire on `network_read` as well as `shell`, which closes one
destination and not the class.

1. **Is there a domain list at all?** An allowlist per policy in `policies/`, protected config #1,
   written by the human. Or none, and `SECURITY.md` says the trifecta is complete under `standard`.
2. **What the gate can see.** A `WebFetch` or `web_search` call carries its URL in the payload, so
   the gate can match a host. A `curl` inside a shell call is the shell interpreter problem again,
   and the gate does not interpret shell. So a list binds the fetch tools and leaves the shell, on
   every provider. Is a half that is honest about being a half worth building?
3. **Per provider.** `agy` and `codex` have no fetch tool in the table the way `claude` does; what
   they reach, they reach by shell. Cost the list per provider before choosing.
4. **The seatbelt.** `(allow default)` allows `network-outbound`. A profile deny with a host
   allowlist would bind the shell too, and it is exactly the kind of deny map ticket 67 said breaks
   providers: every vendor CLI talks to its own API. Measure before deciding.

## Answer

Resolved 2026-09-22 with the human: state the exposure, build nothing. A domain list binds the
fetch tools and leaves `curl`, because the gate does not interpret shell, and the seatbelt's
`network-outbound` filter keys on addresses rather than host names, so neither mechanism closes the
leg; a list that only binds one tool family is the claim-wider-than-code class the hiring panel
marked. `SECURITY.md` now says under `standard` the trifecta is complete, what the floor still
refuses (Slack's posting methods, by shell since before and by fetch URL since build 77), and which
policies grant no network. No build ticket.
