#!/usr/bin/python3
"""Hydra's permission gate, harness spec 7.5.

The shebang is absolute on purpose. `/usr/bin/env python3` resolves through the
child's own PATH, and the child's PATH is inherited: a venv on it, or a Homebrew
Python that has moved, and the gate does not start. On `claude` a hook that fails
to run fails **open** (6.6), so a PATH lookup is the one dependency this script
cannot afford. `/usr/bin/python3` is what macOS ships, it is 3.9.6 on this
machine as of 2026-09-05, and nothing here needs anything newer.

Generated per spawn with the policy compiled in. This file is the template; the
compiler in `src/policy/mod.rs` replaces the `HYDRA_CONFIG` line below with the
run's own policy and writes the result to `~/hydra/spawn/<run-id>/gate`.

**Nothing here reads a policy file.** 7.5 rules that out: anything read at fire
time is a file that may have changed since the previous tool call. All three
providers snapshot their hook config at process start, but the config is a
pointer and the script is its target, and the target is read fresh on every
invocation. The policy is in this file, and the gate denies writes to this file
under every policy.

Python because the gate needs JSON and path matching, macOS ships it, and the
user's own 418 lines of PreToolUse guards are already Python, so one language
covers both sets. No third-party imports: this runs inside the child's
environment, which is an allowlist that carries no PYTHONPATH.

The response dialect is argv[1] and is never inferred. `claude` ignores a
response in a shape it does not recognise and allows the call, so a script that
guesses wrong is an open door on exactly one provider.
"""

import json
import os
import shlex
import sys

HYDRA_CONFIG = None  # HYDRA_COMPILED_POLICY

DIALECTS = ("claude", "cursor", "agy")


# --------------------------------------------------------------------------
# Answering
# --------------------------------------------------------------------------


def respond(dialect, allowed, reason):
    """Writes the decision in the provider's own shape.

    `cursor` gets Cursor's own shape rather than Claude's nested one. 6.6 notes
    that Cursor accepts Claude's dialect on `preToolUse`, and it does, through a
    translation the bundle applies to that one event and to no other.
    `beforeReadFile`, which this gate also answers on Cursor, gets no
    translation, so the nested shape there would be an invalid response. One
    shape per provider is the rule that survives both events: `permission`
    plus `user_message`, which research 30's rig answered in and Cursor obeyed
    on 2026.09.02-c22c1a3, and which a `beforeReadFile` deny was measured
    obeying on the same build on 2026-09-06. `agent_message` carries the same
    text so the model reads the reason too, which is what `claude` does with
    its own field.

    An explicit allow is emitted rather than silence. On `claude` the two are
    the same thing, because 6.6 measured that a hook is a veto and never a key:
    an allow is still checked against the permission rules and cannot suppress
    another hook's deny. Saying it out loud is what lets a test tell a decision
    from a crash, and a crash on `claude` fails open.
    """
    decision = "allow" if allowed else "deny"
    if dialect == "claude":
        out = {
            "hookSpecificOutput": {
                "hookEventName": "PreToolUse",
                "permissionDecision": decision,
                "permissionDecisionReason": reason,
            }
        }
    elif dialect == "cursor":
        out = {
            "permission": decision,
            "user_message": reason,
            "agent_message": reason,
        }
    elif dialect == "agy":
        out = {"decision": decision, "reason": reason}
    else:
        raise AssertionError("unreachable: dialect checked in main")
    sys.stdout.write(json.dumps(out))
    sys.stdout.write("\n")


def deny_without_dialect(reason):
    """The one answer that cannot be spoken in the caller's dialect.

    Nothing said who is asking, so the script answers in the two shapes that
    share a line and exits non-zero as well. `claude` reads the JSON; `agy`
    fails closed on a non-zero exit, so the exit code covers it whatever it
    makes of the body; `cursor` sees two objects where it expects one, which is
    an invalid response, and the hook config Hydra writes says `failClosed`, so
    it blocks the call and loses the reason. Unverified against a running
    provider on purpose, and a wrong guess here fails toward a run that cannot
    start.
    """
    sys.stderr.write("hydra gate: {}\n".format(reason))
    respond("claude", False, reason)
    respond("agy", False, reason)
    sys.exit(2)


# --------------------------------------------------------------------------
# Paths
# --------------------------------------------------------------------------


def fold_ascii(text):
    """Lowercase ASCII letters only.

    Python's str.casefold() follows Unicode equivalence, which disagrees with
    Rust's to_lowercase on characters like ß and ﬁ. Harness spec 15.2 asks for
    macOS filesystem case-insensitivity, which is ASCII-only.
    """
    return "".join(c.lower() if c.isascii() else c for c in text)


def resolve(path, cwd):
    """An absolute, symlink-resolved, case-folded path.

    `realpath` rather than `normpath`, because a symlink inside the worktree
    pointing at `~/.claude` is the cheapest way out of a scoped policy, and
    realpath resolves the existing prefix of a path whose tail does not exist
    yet, which is every file a run is about to create.

    Case-folded because 15.2 says so: `write_file(path="bible.md")` on macOS
    writes `BIBLE.md`, and an exact-case membership test misses it.
    """
    if not os.path.isabs(path):
        path = os.path.join(cwd, path)
    return fold_ascii(os.path.realpath(path))


def inside(path, root):
    """True when `path` is `root` or sits under it. Both already resolved."""
    return path == root or path.startswith(root.rstrip(os.sep) + os.sep)


def target_path(tool, tool_input, cwd):
    """The path a tool call acts on, or None when the table names no field."""
    field = HYDRA_CONFIG["path_fields"].get(tool)
    if field is None:
        return None
    raw = tool_input.get(field)
    if not isinstance(raw, str) or not raw:
        return None
    return resolve(raw, cwd)


def protected_root(path):
    """The denied path this one falls under, or None. Already resolved.

    7.5's list has two kinds of entry since ticket 40, and this is where they
    meet: a path is protected when it is inside a denied path and inside none of
    the paths this spawn owns. The rule behind it is one sentence, a run writes
    only inside what Hydra created for it, and it is why the per-run parents can
    be denied whole while a run still writes in its own worktree and a `curate`
    pass in its own checkout of the store.

    One function rather than two so the `file_write` check and the shell check
    cannot drift apart. A carve-out that held for `Write` and not for `Bash`
    would be a rule with a redirect-shaped hole in it, which is the hole ticket
    02 found in the first version of the shell check.

    The own list is read with a default because a gate compiled before ticket 40
    does not carry one. A `KeyError` here would exit non-zero with nothing on
    stdout, and 6.6 measured `claude` allowing the call in that case, so the
    missing key has to read as an empty carve-out rather than as a crash.

    Since build ticket 44 there is a third kind of entry, checked first: the
    hook and provider config the run's own binary reads out of the worktree,
    `.cursor/hooks.json` and its siblings. The skill roots are not here; they
    are write-denied in `evaluate` and a shell command may name them. Those sit inside the one directory the
    run owns, so the carve-out would hand them back, and a `standard` run could
    rewrite the file that points at this script. The Rust side names them in
    `protected_worktree_paths`, and the same default covers a gate compiled
    before the key existed.
    """
    for protected in HYDRA_CONFIG.get("protected_worktree_paths", []):
        if inside(path, protected):
            return protected
    for own in HYDRA_CONFIG.get("own_write_paths", []):
        if inside(path, own):
            return None
    for protected in HYDRA_CONFIG["deny_write_paths"]:
        if inside(path, protected):
            return protected
    return None


def protected_path_named_by(command, cwd):
    """The first protected path a shell command names, or None.

    **`shell` carries no path for the gate to read**, so the deny list of 7.5
    cannot be applied to it the way it is applied to `Write`. Without this, a
    policy that grants `shell` grants a write to the policy directory, to the run
    corpus and to the gate script itself, by redirect, under every policy that
    allows shell at all. That last one composes into a full bypass: an emptied
    script exits 0 and prints nothing, and 6.6 measured `claude` failing open on
    a response in a shape it does not recognise.

    So a command that names a denied path it does not own is refused whole,
    reads included. Which token of a command is a redirect target is not
    decidable without running it, and neither is which is an argument to a
    writing binary, so the refusal is coarse in the same direction 7.4's floor
    is coarse. Reading the corpus is what `curate` scopes `file_read` to, and
    that path goes through the file tools, which carry a path the gate can
    check.

    **A path a substitution builds is not caught**, and neither is one assembled
    from a variable. That is the failure mode 7.4 names out loud for the floor:
    silent permission on the spelling nobody wrote down. This is a floor for the
    same reason and with the same limit, and `IMPLEMENTATION.md` says so rather
    than letting a passing test read as coverage.

    **Two passes, since build ticket 56.** The token pass below resolves each
    argument, which is what catches a relative path and a symlink. It sees whole
    tokens only, so a denied path spelled in full inside a larger one walked
    through it: `sh -c 'cat /…/runs/x'` is one token to shlex, and so is
    `python3 -c "open('/…/runs/x','w')"` and `$'/…/runs/x'`. The path is written
    down in full in all three, which is exactly what this function claims to
    refuse, so missing them was a parsing bug rather than the documented limit.
    The raw pass runs first: the command, case-folded, is searched for each
    denied prefix as a substring. It is coarser still, and deliberately: a
    command that merely mentions a denied path in a comment or a string is
    refused, which is the direction 7.5 already chose when it refuses reads.

    **What the raw pass cannot carry**, and the residual that leaves. A denied
    path with one of this spawn's own paths under it is not on the list, because
    `~/hydra/worktrees` is denied while the run's own worktree sits inside it,
    and a substring search has no way to say "this one but not that one". So a
    sibling run's worktree spelled inside a larger token, `sh -c 'cat
    /…/worktrees/<other-run>/x'`, still reaches the token pass and still walks
    through it. The corpus, the store, the snapshots, the spawn directory and
    the per-run config files carry no own path and are on the list.
    """
    # The raw pass. `resolve` is not used here because there is nothing to
    # resolve: this looks for a denied path already spelled absolutely, which is
    # how every interpreter and every nested shell spells one.
    lowered = fold_ascii(command)
    for protected in denied_prefixes():
        if protected in lowered:
            return protected

    try:
        segments = split_segments(command)
    except ValueError:
        # An unparseable command. The floor refuses it on its own, and guessing
        # at a path inside one would be the evaluation 7.4 rules out.
        return None
    for argv in segments:
        for token in argv:
            for candidate in path_candidates(token):
                protected = protected_root(resolve(candidate, cwd))
                if protected is not None:
                    return protected
    return None


def protected_path_named_in(value):
    """The denied path named anywhere inside a tool call's arguments, or None.

    Walks strings, lists and dicts, because an MCP server's arguments are its
    own shape and a path can sit at any depth. The match is the raw pass's: the
    string, case-folded, searched for each denied spelling. Nothing is resolved,
    because there is nothing to resolve. A relative path in an MCP argument
    walks through, which is the same residual the raw shell pass has and is
    written down beside it in `protected_path_named_by`.
    """
    if isinstance(value, str):
        lowered = fold_ascii(value)
        for protected in denied_prefixes():
            if protected in lowered:
                return protected
        return None
    if isinstance(value, dict):
        value = value.values()
    if isinstance(value, (list, tuple)) or hasattr(value, "__iter__"):
        for item in value:
            found = protected_path_named_in(item)
            if found is not None:
                return found
    return None


def denied_prefixes():
    """Every denied path this spawn does not own, resolved and case-folded.

    The own carve-out is applied here rather than after the match, because a run
    naming its own worktree is the ordinary case and must not be refused. The
    per-run config files of build ticket 44 are denied whole and stay on the
    list; the skill roots are not, because a command naming one is how two of
    the three providers read a skill.
    """
    spellings = HYDRA_CONFIG.get("denied_spellings")
    if spellings is not None:
        return spellings
    # A gate compiled before the key existed. One spelling each, resolved, which
    # is what the other checks already use.
    own = HYDRA_CONFIG.get("own_write_paths", [])
    out = list(HYDRA_CONFIG.get("protected_worktree_paths", []))
    for protected in HYDRA_CONFIG["deny_write_paths"]:
        if not any(inside(protected, o) for o in own):
            out.append(protected)
    return out


def path_candidates(token):
    """Every path a single shell token might be, before resolution.

    Three shapes, because one token is not always one path:

    `--output=/x` hides a path behind a flag and `D=/x` is an assignment whose
    value the next segment will use, so the tail after the first `=` is read as
    well as the token itself. That catches the assignment at the point it is
    written down rather than at the point it is used, which is the only one of
    the two that appears in argv at all.

    `~` and `$HOME` are expanded, because the shell expands them and a rule that
    does not is a rule against one spelling. `expandvars` reads the gate's own
    environment, which is the child's, so it resolves exactly the variables the
    child's shell would. A variable the command sets itself stays unexpanded and
    is caught, when it is caught, by the assignment above.
    """
    if not token:
        return
    seen = set()
    for part in (token, token.split("=", 1)[1] if "=" in token else None):
        if not part or part in seen:
            continue
        seen.add(part)
        yield os.path.expandvars(os.path.expanduser(part))


# --------------------------------------------------------------------------
# The pattern floor, harness spec 7.4
# --------------------------------------------------------------------------


FLOOR_TARGETS = ("git", "aws", "slack.com")


def has_substitution(command):
    return any(m in command for m in HYDRA_CONFIG["floor"]["substitution_markers"])


def split_segments(command):
    """The command's argv lists, split on the shell's own operators.

    Raises ValueError on an unbalanced quote, which the caller turns into a
    refusal rather than a guess.
    """
    lex = shlex.shlex(command, posix=True, punctuation_chars=True)
    lex.whitespace_split = True
    operators = {";", "&&", "||", "|", "&", "\n", "(", ")"}
    segments, current = [], []
    for token in lex:
        if token in operators:
            if current:
                segments.append(current)
            current = []
        else:
            current.append(token)
    if current:
        segments.append(current)
    return segments


def short_flags(argv):
    """Every character in a `-abc` cluster, so `git clean -fd` reads as -f -d."""
    chars = set()
    for token in argv:
        if token.startswith("-") and not token.startswith("--") and len(token) > 1:
            chars.update(token[1:])
    return chars


def git_subcommand(argv):
    """The subcommand, after consuming the options that precede it.

    `git -C /elsewhere push --force` hides its subcommand behind an option that
    takes a value, and 7.4 requires each pattern to consume `-C` and
    `--git-dir` before reading it. Both spellings are handled, `-C x` and
    `--git-dir=x`.
    """
    takes_value = HYDRA_CONFIG["floor"]["git"]["consume_before_subcommand"]
    i = 1
    while i < len(argv):
        token = argv[i]
        if token in takes_value:
            i += 2
            continue
        if any(token.startswith(opt + "=") for opt in takes_value):
            i += 1
            continue
        if token.startswith("-"):
            i += 1
            continue
        return token, argv[i + 1 :]
    return None, []


def check_git(argv):
    rules = HYDRA_CONFIG["floor"]["git"]
    sub, rest = git_subcommand(argv)
    if sub is None:
        return None

    if sub == "push":
        for flag in rules["push_forbidden_flags"]:
            if flag in rest:
                return "git push {}".format(flag)
        positional = [t for t in rest if not t.startswith("-")]
        for token in positional[1:]:
            if token.startswith("+"):
                return "git push with a forced refspec, {}".format(token)
            branch = token.split(":")[-1]
            branch = branch.rsplit("/", 1)[-1] if branch.startswith("refs/") else branch
            if branch in rules["protected_branches"]:
                return "git push to the protected branch {}".format(branch)
            for prefix in rules["protected_branch_prefixes"]:
                if token.split(":")[-1].startswith(prefix):
                    return "git push to a protected branch, {}".format(token)
        return None

    if sub == "clean":
        flags = short_flags(rest)
        forced = "f" in flags or "--force" in rest
        wanted = {c.lstrip("-") for c in rules["clean_requires_force_with"]}
        if forced and (flags & wanted):
            return "git clean -f with -d or -x"
        return None

    if sub == "worktree" and rest[:1] == ["list"]:
        # 7.4 forbids `git worktree` in every form but this one, and a forbidden
        # entry is an argv prefix that cannot say "except", so the exception is
        # here beside `clean`'s. Listing worktrees reads; `remove --force`
        # discards a sibling run's uncommitted work and `prune` drops the
        # registration `hydra reap` needs. Bare `git worktree` names no form and
        # falls through to the forbidden entry below.
        return None

    if sub == "symbolic-ref":
        positional = [t for t in rest if not t.startswith("-")]
        if len(positional) >= 2:
            return "git symbolic-ref in write form"
        return None

    for entry in rules["forbidden"]:
        head = entry["argv"]
        if sub != head[0]:
            continue
        if len(head) == 1:
            return "git {}, {}".format(sub, entry["why"])
        if head[1] in rest:
            return "git {} {}, {}".format(sub, head[1], entry["why"])
    return None


def check_aws(argv):
    rules = HYDRA_CONFIG["floor"]["aws"]
    positional = [t for t in argv[1:] if not t.startswith("-")]
    if not positional:
        return None
    sub = positional[0]
    for forbidden in rules["forbidden_subcommands"]:
        if sub == forbidden[0]:
            return "aws {}".format(sub)
    if sub == "s3" and len(positional) > 1:
        if positional[1] in rules["s3_forbidden_operations"]:
            return "aws s3 {}".format(positional[1])
    return None


def check_slack(command):
    rules = HYDRA_CONFIG["floor"]["slack"]
    lowered = fold_ascii(command)
    for host in rules["forbidden_hosts"]:
        if host in lowered:
            return "a {} URL".format(host)
    for method in rules["forbidden_methods"]:
        if method in lowered:
            return "the Slack method {}".format(method)
    for prefix in rules["forbidden_method_prefixes"]:
        if "slack.com" in lowered and prefix in lowered:
            return "a Slack {}* method".format(prefix)
    return None


def floor_violation(command):
    """What the floor refuses in this shell command, or None.

    A floor and never a gate. It catches the spellings compiled into this
    script and nothing else, which is the failure mode 7.4 names out loud
    rather than hides.
    """
    if has_substitution(command):
        # 7.4: refuse a subcommand arriving from a substitution rather than
        # trying to evaluate it. Which segment the substitution feeds is not
        # decidable without running it, so the refusal covers the command when
        # any floor target is named anywhere in it.
        lowered = fold_ascii(command)
        if any(target in lowered for target in FLOOR_TARGETS):
            return "a shell substitution in a command naming git, aws or Slack"
        return None

    try:
        segments = split_segments(command)
    except ValueError:
        return "a command the floor could not parse"

    hit = check_slack(command)
    if hit:
        return hit

    for argv in segments:
        if not argv:
            continue
        binary = os.path.basename(argv[0])
        if binary == "git":
            hit = check_git(argv)
        elif binary == "aws":
            hit = check_aws(argv)
        else:
            hit = None
        if hit:
            return hit
    return None


# --------------------------------------------------------------------------
# Policy
# --------------------------------------------------------------------------


def capability_of(tool):
    # A prefix on two providers and the provider's own spelling on each:
    # `mcp__<server>__<tool>` on claude, `MCP:<tool>` on cursor. `agy` names one
    # tool, `call_mcp_tool`, so its config carries no prefix and the table
    # answers for it like any other name. An empty string here would match every
    # tool, which is why the absent case is null rather than "".
    prefix = HYDRA_CONFIG.get("mcp_prefix")
    if prefix and tool.startswith(prefix):
        return "mcp_call"
    return HYDRA_CONFIG["tools"].get(tool)


def as_tool_call(payload):
    """The payload as `evaluate` reads it: a tool name, its input, a cwd.

    Three shapes reach here and only one is already in this shape.

    Cursor's `beforeReadFile` carries no `tool_name`. Measured on
    2026.09.02-c22c1a3, 2026-09-06: the path is a top-level `file_path`, the
    file's content rides beside it, and there is no `cwd`. It is a read, so it is
    evaluated as the provider's `Read` tool with that path, which is the row the
    table already holds for it.

    `agy` sends `toolCall.name` and `toolCall.args` in camelCase protojson, with
    no `cwd` field at all. Measured on 1.1.26, 2026-09-06. Two things stand in
    for the cwd, in order: `run_command` puts the directory it would run in on
    `args.Cwd`, and every payload carries `workspacePaths`, whose first entry is
    the worktree Hydra passed on `--add-dir`. Falling back to the compiled
    `worktree` scope, which `evaluate` does when both are absent, would resolve a
    relative path against the run's own checkout, and that is the same directory
    in every launch Hydra makes.

    Keying on the payload rather than on the dialect is deliberate: contract
    7.7's read-time re-evaluation sends the normalized shape for every provider,
    so the same script answers the live hook and the replay.
    """
    if payload.get("hook_event_name") == "beforeReadFile":
        return {
            "tool_name": "Read",
            "tool_input": {"file_path": payload.get("file_path")},
            "cwd": payload.get("cwd"),
        }
    call = payload.get("toolCall")
    if isinstance(call, dict):
        args = call.get("args")
        if not isinstance(args, dict):
            args = {}
        paths = payload.get("workspacePaths")
        workspace = paths[0] if isinstance(paths, list) and paths else None
        return {
            "tool_name": call.get("name"),
            "tool_input": args,
            "cwd": args.get("Cwd") or workspace,
        }
    return payload


def scopes_for(capability):
    return [
        entry.get("scope")
        for entry in HYDRA_CONFIG["allow"]
        if entry["capability"] == capability
    ]


def evaluate(payload):
    """(allowed, reason) for one tool call."""
    payload = as_tool_call(payload)
    tool = payload.get("tool_name")
    if not isinstance(tool, str) or not tool:
        return False, "hydra/no-tool-name: the payload named no tool"

    tool_input = payload.get("tool_input")
    if not isinstance(tool_input, dict):
        tool_input = {}
    cwd = payload.get("cwd") or HYDRA_CONFIG["scopes"].get("worktree") or os.getcwd()

    capability = capability_of(tool)
    if capability is None:
        # 7.1, and it is deliberate that this fires under `permissive` too. A
        # self-update that renames a tool breaks the next run loudly instead of
        # quietly widening what a child may do.
        return False, (
            "hydra/unknown-tool: {} is not in Hydra's capability table for this "
            "provider. The table is stale or the provider renamed a tool.".format(tool)
        )

    path = target_path(tool, tool_input, cwd)

    # Own-read carve-out, before the denied-parent check. A Cursor run's
    # `cursor-data/` sits under the spawn directory, a denied parent since
    # ticket 40, and is not a read scope, so without this a `file_read` of
    # the files the provider wrote there (the terminal files `awaitToolCall`
    # is answered from) is refused as out-of-scope. Every policy that grants
    # `file_read` at all is covered; `judge` grants none and falls through.
    # A gate compiled before this key existed reads it as empty, the same
    # default `own_write_paths` uses, so a missing key cannot crash the
    # script open on `claude`.
    if capability == "file_read" and path is not None:
        for own in HYDRA_CONFIG.get("own_read_paths", []):
            if inside(path, own) and scopes_for(capability):
                return True, "hydra/allow: policy {} allows {}.".format(
                    HYDRA_CONFIG["policy"], capability
                )

    # The path denials of 7.5, before the policy is consulted at all. This is
    # where `permissive` stops being permissive.
    if capability == "file_write" and path is not None:
        protected = protected_root(path)
        if protected is not None:
            return False, (
                "hydra/protected-path: {} writes under {}, denied under every "
                "policy by harness spec 7.5.".format(tool, protected)
            )
        # The skill roots Hydra linked into the worktree (6.4). A write there
        # replaces a link, so the tool-level write is refused under every
        # policy. The shell floor does not read this list on purpose: two of
        # three providers read a SKILL.md with `cat`, and a command that names
        # the root is how they do it. Build ticket 44, narrowed the same day
        # after `ls -la .claude/skills/` on run 01a09405-f451 was refused.
        for root in HYDRA_CONFIG.get("write_denied_worktree_paths", []):
            if inside(path, root):
                return False, (
                    "hydra/protected-path: {} writes under {}, the skills Hydra "
                    "linked for this run, denied under every policy by harness "
                    "spec 6.4.".format(tool, root)
                )

    # The same list, reached through the capability that carries no path field
    # at all. An MCP tool's arguments are whatever its server declared, so there
    # is no field to name in the table and nothing for `target_path` to read.
    # Until build ticket 59 that meant `permissive` let an MCP tool write the
    # corpus: the deny list was applied to `file_write` and to `shell`, and
    # `mcp_call` met neither. `config/mcp.md` admits no writing server today, so
    # nothing had fired; the gun was loaded. The check is the raw pass of
    # `protected_path_named_by` over every string the call carries, which is
    # coarse in the same direction and for the same reason.
    if capability == "mcp_call":
        protected = protected_path_named_in(tool_input)
        if protected is not None:
            return False, (
                "hydra/protected-path: an argument of {} names {}, denied under "
                "every policy by harness spec 7.5. An MCP call carries no path "
                "field for the gate to check, so one that names a protected path "
                "is refused whole.".format(tool, protected)
            )

    # The same list, reached through the capability that carries no path.
    if capability == "shell":
        raw_command = tool_input.get(HYDRA_CONFIG["command_field"], "")
        if isinstance(raw_command, str) and raw_command:
            protected = protected_path_named_by(raw_command, cwd)
            if protected is not None:
                return False, (
                    "hydra/protected-path: the command names {}, denied under "
                    "every policy by harness spec 7.5. A shell command carries no "
                    "path for the gate to check, so one that names a protected "
                    "path is refused whole.".format(protected)
                )

    allowed_scopes = scopes_for(capability)
    if not allowed_scopes:
        return False, (
            "hydra/capability: policy {} does not allow {} ({}).".format(
                HYDRA_CONFIG["policy"], capability, tool
            )
        )

    if capability in ("file_read", "file_write"):
        if "anywhere" in allowed_scopes:
            pass
        elif path is None:
            return False, (
                "hydra/no-path: policy {} scopes {} and {} named no path to "
                "check.".format(HYDRA_CONFIG["policy"], capability, tool)
            )
        else:
            roots = [
                HYDRA_CONFIG["scopes"][name]
                for name in allowed_scopes
                if name in HYDRA_CONFIG["scopes"]
            ]
            if not any(inside(path, root) for root in roots):
                return False, (
                    "hydra/out-of-scope: policy {} allows {} inside {} and this "
                    "call is outside it.".format(
                        HYDRA_CONFIG["policy"],
                        capability,
                        " or ".join(allowed_scopes),
                    )
                )

    if capability == "shell" and not HYDRA_CONFIG["floor_lifted"]:
        command = tool_input.get(HYDRA_CONFIG["command_field"], "")
        if isinstance(command, str) and command:
            hit = floor_violation(command)
            if hit:
                return False, (
                    "hydra/floor: the pattern floor refuses {}. Lift it for one run "
                    "with --allow-floor-escape.".format(hit)
                )

    return True, "hydra/allow: policy {} allows {}.".format(
        HYDRA_CONFIG["policy"], capability
    )


def main():
    dialect = sys.argv[1] if len(sys.argv) > 1 else None
    if dialect not in DIALECTS:
        deny_without_dialect(
            "no response dialect on the command line, so the answer cannot be "
            "spoken in a shape the caller is known to read"
        )

    # The config names the provider this gate was compiled for, and argv names
    # the shape the caller reads. Both name one binary, so a mismatch means
    # something is wired wrong, and a gate that shrugged and answered anyway
    # would be answering with the wrong provider's tool table.
    if dialect != HYDRA_CONFIG["dialect"]:
        respond(
            dialect,
            False,
            "hydra/wrong-dialect: this gate was compiled for the {} dialect and "
            "was asked in {}.".format(HYDRA_CONFIG["dialect"], dialect),
        )
        return

    try:
        raw = sys.stdin.read()
        payload = json.loads(raw) if raw.strip() else {}
    except ValueError:
        respond(dialect, False, "hydra/bad-payload: the hook payload was not JSON")
        return

    if not isinstance(payload, dict):
        respond(dialect, False, "hydra/bad-payload: the hook payload was not an object")
        return

    allowed, reason = evaluate(payload)
    respond(dialect, allowed, reason)


if __name__ == "__main__":
    main()
