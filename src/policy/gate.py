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

    `claude` and `cursor` share one shape: 6.6 records that Cursor accepts
    Claude's response dialect on `preToolUse`, which is why the script answers
    in two shapes rather than three. Only the `claude` shape has been measured
    against a running provider (ticket 26, on 2.1.257). Tickets 09 and 10 own
    the other two.

    An explicit allow is emitted rather than silence. On `claude` the two are
    the same thing, because 6.6 measured that a hook is a veto and never a key:
    an allow is still checked against the permission rules and cannot suppress
    another hook's deny. Saying it out loud is what lets a test tell a decision
    from a crash, and a crash on `claude` fails open.
    """
    decision = "allow" if allowed else "deny"
    if dialect in ("claude", "cursor"):
        out = {
            "hookSpecificOutput": {
                "hookEventName": "PreToolUse",
                "permissionDecision": decision,
                "permissionDecisionReason": reason,
            }
        }
    elif dialect == "agy":
        out = {"decision": decision, "reason": reason}
    else:
        raise AssertionError("unreachable: dialect checked in main")
    sys.stdout.write(json.dumps(out))
    sys.stdout.write("\n")


def deny_without_dialect(reason):
    """The one answer that cannot be spoken in the caller's dialect.

    Nothing said who is asking, so the script answers in every shape it has and
    exits non-zero as well. `claude` and `cursor` read the JSON; `agy` fails
    closed on a non-zero exit, so the exit code covers it whatever it makes of
    the body. Unverified against a running provider on purpose: this ticket
    launches none, and a wrong guess here fails toward a run that cannot start.
    """
    sys.stderr.write("hydra gate: {}\n".format(reason))
    respond("claude", False, reason)
    respond("agy", False, reason)
    sys.exit(2)


# --------------------------------------------------------------------------
# Paths
# --------------------------------------------------------------------------


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
    return os.path.realpath(path).casefold()


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


def protected_path_named_by(command, cwd):
    """The first protected path a shell command names, or None.

    **`shell` carries no path for the gate to read**, so the deny list of 7.5
    cannot be applied to it the way it is applied to `Write`. Without this, a
    policy that grants `shell` grants a write to the policy directory, to the run
    corpus and to the gate script itself, by redirect, under every policy that
    allows shell at all. That last one composes into a full bypass: an emptied
    script exits 0 and prints nothing, and 6.6 measured `claude` failing open on
    a response in a shape it does not recognise.

    So a command that names one of the seven is refused whole, reads included.
    Which token of a command is a redirect target is not decidable without
    running it, and neither is which is an argument to a writing binary, so the
    refusal is coarse in the same direction 7.4's floor is coarse. Reading the
    corpus is what `curate` scopes `file_read` to, and that path goes through the
    file tools, which carry a path the gate can check.

    **A path a substitution builds is not caught**, and neither is one assembled
    from a variable. That is the failure mode 7.4 names out loud for the floor:
    silent permission on the spelling nobody wrote down. This is a floor for the
    same reason and with the same limit, and `IMPLEMENTATION.md` says so rather
    than letting a passing test read as coverage.
    """
    try:
        segments = split_segments(command)
    except ValueError:
        # An unparseable command. The floor refuses it on its own, and guessing
        # at a path inside one would be the evaluation 7.4 rules out.
        return None
    for argv in segments:
        for token in argv:
            if not token:
                continue
            # `--output=/path` hides a path behind a flag, so read the tail of
            # any option that carries one rather than skipping the whole token.
            if token.startswith("-"):
                if "=" not in token:
                    continue
                token = token.split("=", 1)[1]
                if not token:
                    continue
            candidate = resolve(os.path.expanduser(token), cwd)
            for protected in HYDRA_CONFIG["deny_write_paths"]:
                if inside(candidate, protected):
                    return protected
    return None


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
    lowered = command.casefold()
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
        lowered = command.casefold()
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
    if tool.startswith("mcp__"):
        return "mcp_call"
    return HYDRA_CONFIG["tools"].get(tool)


def scopes_for(capability):
    return [
        entry.get("scope")
        for entry in HYDRA_CONFIG["allow"]
        if entry["capability"] == capability
    ]


def evaluate(payload):
    """(allowed, reason) for one tool call."""
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

    # The seven path denials of 7.5, before the policy is consulted at all.
    # This is where `permissive` stops being permissive.
    if capability == "file_write" and path is not None:
        for protected in HYDRA_CONFIG["deny_write_paths"]:
            if inside(path, protected):
                return False, (
                    "hydra/protected-path: {} writes under {}, denied under every "
                    "policy by harness spec 7.5.".format(tool, protected)
                )

    # The same seven, reached through the capability that carries no path.
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
