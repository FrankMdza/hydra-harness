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
time is a file that may have changed since the previous tool call. The four
providers measured, `claude`, `cursor-agent`, `agy` and `codex`, snapshot their hook config at
process start, but the config is a
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


def answer_for(dialect, allowed, reason):
    """The decision in the provider's own shape, as a dict."""
    decision = "allow" if allowed else "deny"
    if dialect == "claude":
        return {
            "hookSpecificOutput": {
                "hookEventName": "PreToolUse",
                "permissionDecision": decision,
                "permissionDecisionReason": reason,
            }
        }
    if dialect == "cursor":
        return {
            "permission": decision,
            "user_message": reason,
            "agent_message": reason,
        }
    if dialect == "agy":
        return {"decision": decision, "reason": reason}
    raise AssertionError("unreachable: dialect checked in main")


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
    sys.stdout.write(json.dumps(answer_for(dialect, allowed, reason)))
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


PATCH_HEADERS = (
    "*** Add File: ",
    "*** Update File: ",
    "*** Delete File: ",
    "*** Move to: ",
)


def parse_patch_paths(body, cwd):
    """Every path an `apply_patch` body names in its headers, resolved."""
    paths = []
    for line in body.splitlines():
        for prefix in PATCH_HEADERS:
            if line.startswith(prefix):
                raw = line[len(prefix) :].strip()
                if raw:
                    paths.append(resolve(raw, cwd))
                break
    return paths


def paths_from_changes(tool_input, cwd):
    """Paths a replayed `file_change` carries in `changes[].path`."""
    changes = tool_input.get("changes")
    if not isinstance(changes, list):
        return []
    paths = []
    for change in changes:
        if not isinstance(change, dict):
            continue
        raw = change.get("path")
        if isinstance(raw, str) and raw:
            paths.append(resolve(raw, cwd))
    return paths


def target_paths(tool, tool_input, cwd):
    """Every path a tool call acts on, or an empty list when it names none."""
    patch_tools = HYDRA_CONFIG.get("patch_path_tools", {})
    if tool in patch_tools:
        paths = []
        field = patch_tools[tool]
        raw = tool_input.get(field)
        if isinstance(raw, str) and raw:
            paths.extend(parse_patch_paths(raw, cwd))
        paths.extend(paths_from_changes(tool_input, cwd))
        seen = set()
        out = []
        for path in paths:
            if path not in seen:
                seen.add(path)
                out.append(path)
        return out
    path = target_path(tool, tool_input, cwd)
    if path is None:
        return []
    return [path]


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
    the per-run config files are covered when the denied prefix appears whole
    and in canonical form. A path split by a quote, assembled by concatenation,
    percent-encoded, or spelled with `..`, `//` or `/./` inside an interpreter
    or nested-shell token is not.
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
    list; the skill roots are not, because a command naming one is how three of
    the four providers read a skill.
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
# The bounded shell, build ticket 139
# --------------------------------------------------------------------------


def shell_list():
    """The command list the policy bounds `shell` to, or None when it is unbounded."""
    entries = [e for e in HYDRA_CONFIG["allow"] if e["capability"] == "shell"]
    if not entries or any("commands" not in e for e in entries):
        return None
    return [c for e in entries for c in e["commands"]]


def shell_hazard(command):
    """The first thing in `command` the shell would act on and the list cannot read.

    A character walk with the quote state kept, because shlex drops the quotes
    and `grep 'x$' f` and `grep x$HOME f` are the same tokens afterwards. Single
    quotes make everything literal; double quotes still expand `$` and the
    backquote; a backslash outside single quotes takes the next character.
    """
    quote = None
    i = 0
    n = len(command)
    while i < n:
        c = command[i]
        following = command[i + 1] if i + 1 < n else ""
        if quote == "'":
            if c == "'":
                quote = None
        elif c == "\\":
            i += 1
        elif c == "`":
            return "a backquote substitution"
        elif c == "$" and following and not following.isspace() and following != '"':
            return "the expansion `${}`, whose value the gate cannot read".format(following)
        elif quote == '"':
            if c == '"':
                quote = None
        elif c in "'\"":
            quote = c
        elif c in "\n\r":
            return "a newline, which the shell reads as a separator"
        elif c in "<>":
            return "the redirect `{}`".format(c)
        elif c in "()":
            return "the subshell `{}`".format(c)
        elif c == "&":
            if following != "&":
                return "the operator `&`"
            i += 1
        elif c in "*?[":
            return "the unquoted glob `{}`, which the shell may expand into a flag".format(c)
        elif c == "{":
            close = command.find("}", i)
            if close != -1 and ("," in command[i:close] or ".." in command[i:close]):
                return "a brace expansion, which the shell may expand into a flag"
        i += 1
    if quote is not None:
        return "an unbalanced quote, so a command the gate could not split"
    return None


def refused_flag(argv, entry):
    """The first flag in `argv` the list entry refuses, or None.

    A long option matches its own abbreviations too, because GNU getopt and
    git both accept a unique prefix: `git diff --outp=f` is `--output=f`.
    """
    for token in argv[len(entry["argv"]) :]:
        if not token.startswith("-"):
            continue
        name = token.split("=", 1)[0]
        for flag in entry.get("refuse", []):
            if token == flag or token.startswith(flag + "="):
                return flag
            if flag.startswith("--") and name.startswith("--") and len(name) > 2 and flag.startswith(name):
                return flag
            short = len(flag) == 2 and flag[0] == "-"
            if short and not token.startswith("--") and flag[1] in token[1:]:
                return flag
    return None


def read_roots():
    """Where `file_read` reaches under this policy, resolved; None for anywhere."""
    names = scopes_for("file_read")
    if "anywhere" in names:
        return None
    return [HYDRA_CONFIG["scopes"][s] for s in names if s in HYDRA_CONFIG["scopes"]]


def shell_path_candidates(token):
    """What one argument might name as a path: itself, a value after `=`, and
    the value glued to a short flag, `-f/etc/hosts`."""
    yield token
    if "=" in token:
        yield token.split("=", 1)[1]
    if token.startswith("-") and not token.startswith("--") and len(token) > 2:
        yield token[2:]


def bounded_shell_violation(command, commands, cwd):
    """What the list refuses in `command`, or None.

    The list is the first filter, and the seatbelt `src/sandbox.rs` composes
    for a bounded shell is the guarantee: a command on the list can still write
    (`sed -n 'w f'` names no flag), and the kernel is what refuses it. So this
    refuses rather than interprets. It splits on a pipe, `;`, `&&` and `||`
    with the floor's own `split_segments`, and refuses anything else the
    shell would act on before the words reach the binary:
    redirects, substitutions, expansions, globs and background jobs. Each
    segment must then start with the words of one list entry and carry none of
    its refused flags, and every argument must resolve inside the policy's
    `file_read` scopes, which is where `realpath` catches a symlink out.
    """
    if not command.strip():
        return "an empty command"
    hazard = shell_hazard(command)
    if hazard:
        return hazard
    try:
        segments = split_segments(command)
    except ValueError:
        return "a command the gate could not split"
    roots = read_roots()
    if roots is not None and not any(inside(resolve(cwd, cwd), r) for r in roots):
        return "the working directory {}, outside what the policy reads".format(cwd)
    # `shell_hazard` has already refused every unquoted operator but these
    # four, so what `split_segments` splits on here is a real separator. A
    # quoted `'|'` is split as well, which refuses a pattern rather than
    # admitting a command, the direction this errs in.
    for argv in segments:
        matches =[c for c in commands if argv[: len(c["argv"])] == c["argv"]]
        if not matches:
            longer = any(len(c["argv"]) > 1 and c["argv"][0] == argv[0] for c in commands)
            name = " ".join(argv[:2] if longer else argv[:1])
            return "`{}`, which is not on its list".format(name)
        for entry in matches:
            flag = refused_flag(argv, entry)
            if flag:
                return "`{} {}`, which its list refuses".format(" ".join(entry["argv"]), flag)
        if roots is None:
            continue
        for token in argv[1:]:
            for candidate in shell_path_candidates(token):
                path = resolve(os.path.expanduser(candidate), cwd)
                if not any(inside(path, r) for r in roots):
                    return "the argument {}, which resolves to {}, outside what the policy reads".format(
                        token, path
                    )
    return None


# --------------------------------------------------------------------------
# Policy
# --------------------------------------------------------------------------


# --------------------------------------------------------------------------
# `mcp_call` scoped to named servers, build ticket 140
# --------------------------------------------------------------------------


def mcp_server_list():
    """The servers the policy scopes `mcp_call` to, or None when it names none."""
    entries = [e for e in HYDRA_CONFIG["allow"] if e["capability"] == "mcp_call"]
    if not entries or any("servers" not in e for e in entries):
        return None
    return [s for e in entries for s in e["servers"]]


def mcp_server_violation(tool, servers):
    """Why `tool` reaches no server on the list, or None when it reaches one.

    Only a prefix of `mcp__` carries the server in the tool's name, as
    `mcp__<server>__<tool>`. `cursor-agent` spells `MCP:<tool>` and `agy`
    names one tool for every server, so neither says which server a call
    reaches, and a list refuses both.

    The server part has to equal a listed server, not start with one. A name
    splits into server and tool one way only when the tool part neither
    starts with `_` nor holds `__`; otherwise `mcp__hydra___x` is server
    `hydra_` as easily as `hydra`, and `mcp__hydra__evil__x` is server
    `hydra__evil`. Build ticket 142. Hydra's own tools are single words.
    """
    prefix = HYDRA_CONFIG.get("mcp_prefix")
    if prefix == "mcp__":
        for server in servers:
            head = "mcp__" + server + "__"
            if not tool.startswith(head):
                continue
            name = tool[len(head):]
            if name and not name.startswith("_") and "__" not in name:
                return None
        return "a server the list does not name"
    return "no server in its name for the list to match"


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


def wrong_dialect_envelope(payload):
    """Another provider's hook envelope, or None when the payload matches ours.

    `beforeReadFile` is Cursor's and `toolCall` is `agy`'s. A gate compiled for
    one dialect reads only that dialect's shape; a payload that carries the
    other's key is refused rather than unwrapped, because unwrapping would leave
    the real `tool_name` unchecked. Contract 7.7's replay sends the normalized
    shape and carries neither key.
    """
    dialect = HYDRA_CONFIG["dialect"]
    has_before_read = payload.get("hook_event_name") == "beforeReadFile"
    # Key presence, whatever the value. `"toolCall": null` or `[]` is still
    # agy's envelope, and reading it as ours would check a `tool_name` the
    # sender never meant as the call.
    has_tool_call = "toolCall" in payload
    if dialect == "claude" and (has_before_read or has_tool_call):
        if has_before_read:
            return "cursor's beforeReadFile"
        return "agy's toolCall"
    if dialect == "cursor" and has_tool_call:
        return "agy's toolCall"
    if dialect == "agy" and has_before_read:
        return "cursor's beforeReadFile"
    return None


def as_tool_call(payload):
    """The payload as `evaluate` reads it: a tool name, its input, a cwd.

    Three shapes reach here and only one is already in this shape.

    Cursor's `beforeReadFile` carries no `tool_name`. Measured on
    2026.09.02-c22c1a3, 2026-09-06: the path is a top-level `file_path`, the
    file's content rides beside it, and there is no `cwd`. It is a read, so it is
    evaluated as the provider's `Read` tool with that path, which is the row the
    table already holds for it. Only a gate compiled for `cursor` unwraps it.

    `agy` sends `toolCall.name` and `toolCall.args` in camelCase protojson, with
    no `cwd` field at all. Measured on 1.1.26, 2026-09-06. Two things stand in
    for the cwd, in order: `run_command` puts the directory it would run in on
    `args.Cwd`, and every payload carries `workspacePaths`, whose first entry is
    the worktree Hydra passed on `--add-dir`. Falling back to the compiled
    `worktree` scope, which `evaluate` does when both are absent, would resolve a
    relative path against the run's own checkout, and that is the same directory
    in every launch Hydra makes. Only a gate compiled for `agy` unwraps it.

    Contract 7.7's read-time re-evaluation sends the normalized shape for every
    provider, which carries neither envelope, so the same script answers the
    live hook and the replay.
    """
    dialect = HYDRA_CONFIG["dialect"]
    if dialect == "cursor" and payload.get("hook_event_name") == "beforeReadFile":
        return {
            "tool_name": "Read",
            "tool_input": {"file_path": payload.get("file_path")},
            "cwd": payload.get("cwd"),
        }
    if dialect == "agy":
        call = payload.get("toolCall")
        if isinstance(call, dict):
            # `args` goes through as it arrived. One that is not an object,
            # an explicit null included, reaches `evaluate` as a `tool_input`
            # it cannot read and is refused there, rather than emptied here
            # into a command line of nothing. A missing `args` leaves the key
            # out, so `evaluate` reads it as absent.
            paths = payload.get("workspacePaths")
            workspace = paths[0] if isinstance(paths, list) and paths else None
            unwrapped = {"tool_name": call.get("name")}
            own_cwd = None
            if "args" in call:
                args = call["args"]
                unwrapped["tool_input"] = args
                own_cwd = args.get("Cwd") if isinstance(args, dict) else None
            unwrapped["cwd"] = own_cwd or workspace
            return unwrapped
    return payload


def scopes_for(capability):
    return [
        entry.get("scope")
        for entry in HYDRA_CONFIG["allow"]
        if entry["capability"] == capability
    ]


# The URL field of each fetch tool in the dialect tables. `claude`'s `WebFetch`
# and Cursor's `Fetch` send `url`, the second read off the normalizer's own
# mapping of `webFetchToolCall`. `agy`'s `Url` on `read_url_content` follows the
# PascalCase of every field measured on that provider and is itself unmeasured.
# `codex` has no hooked fetch tool. The search tools, `WebSearch` and
# `search_web`, carry a query rather than a URL and are left out on purpose: a
# query that mentions `files.upload` sends nothing to Slack.
FETCH_URL_FIELDS = {
    "WebFetch": "url",
    "Fetch": "url",
    "read_url_content": "Url",
}


def slack_violation_in_fetch(tool, tool_input):
    """What the floor's Slack rules refuse in a fetch tool's URL, or None."""
    field = FETCH_URL_FIELDS.get(tool)
    if field is None:
        return None
    url = tool_input.get(field)
    if not isinstance(url, str) or not url:
        return None
    return check_slack(url)


def evaluate(payload):
    """(allowed, reason) for one tool call."""
    envelope = wrong_dialect_envelope(payload)
    if envelope is not None:
        return False, (
            "hydra/wrong-envelope: this gate was compiled for the {} dialect "
            "and the payload carried {}.".format(HYDRA_CONFIG["dialect"], envelope)
        )

    payload = as_tool_call(payload)
    tool = payload.get("tool_name")
    if not isinstance(tool, str) or not tool:
        return False, "hydra/no-tool-name: the payload named no tool"

    # A missing `tool_input` stays `{}` for tools the table names no field on:
    # `Task`, `Agent`, `mcp__*` and the `provider_native` rows carry optional
    # arguments the gate does not resolve a path from. A present `tool_input` that is not
    # an object is refused, because coercing it to `{}` would empty a shell
    # command or a path field the gate must read. Presence is the key, so an
    # explicit null is present and refused like `true` or a list.
    if "tool_input" not in payload:
        tool_input = {}
    else:
        tool_input = payload["tool_input"]
    if not isinstance(tool_input, dict):
        return False, (
            "hydra/bad-payload: {} carries tool_input the gate cannot read as an "
            "object.".format(tool)
        )
    cwd = payload.get("cwd")
    if not isinstance(cwd, str) or not cwd:
        cwd = HYDRA_CONFIG["scopes"].get("worktree") or os.getcwd()

    capability = capability_of(tool)
    if capability is None:
        # 7.1, and it is deliberate that this fires under `permissive` too. A
        # self-update that renames a tool breaks the next run loudly instead of
        # quietly widening what a child may do.
        return False, (
            "hydra/unknown-tool: {} is not in Hydra's capability table for this "
            "provider. The table is stale or the provider renamed a tool.".format(tool)
        )

    paths = target_paths(tool, tool_input, cwd)
    path = paths[0] if paths else None

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
    if capability == "file_write":
        # A write the gate cannot place is a write it cannot check. Under the
        # four scoped policies `hydra/no-path` below already refuses it; under
        # `permissive` the `anywhere` branch used to pass it, so a write row
        # whose field name the table got wrong, or a provider that renamed the
        # field, was a write to a denied path the gate reported as an allow.
        # The two `agy` rows the table marks unmeasured are the case in point.
        if not paths:
            return False, (
                "hydra/no-path: {} is a write and named no path the gate could "
                "check, so it is refused under every policy.".format(tool)
            )
        for write_path in paths:
            protected = protected_root(write_path)
            if protected is not None:
                return False, (
                    "hydra/protected-path: {} writes {}, under {}, denied under "
                    "every policy by harness spec 7.5.".format(tool, write_path, protected)
                )
            # The skill roots Hydra linked into the worktree (6.4). A write there
            # replaces a link, so the tool-level write is refused under every
            # policy. The shell floor does not read this list on purpose: three of
            # the four providers read a SKILL.md with `cat`, and a command that names
            # the root is how they do it. Build ticket 44, narrowed the same day
            # after `ls -la .claude/skills/` on run 01a09405-f451 was refused.
            for root in HYDRA_CONFIG.get("write_denied_worktree_paths", []):
                if inside(write_path, root):
                    return False, (
                        "hydra/protected-path: {} writes {}, under {}, the skills "
                        "Hydra linked for this run, denied under every policy by "
                        "harness spec 6.4.".format(tool, write_path, root)
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

    # A fetch tool's argument is a URL, and `file://` is a scheme. Whether any of
    # the four providers dereferences one is not measured; the check costs the
    # same raw pass `mcp_call` pays and refuses only a URL that spells a
    # protected path.
    if capability == "network_read":
        protected = protected_path_named_in(tool_input)
        if protected is not None:
            return False, (
                "hydra/protected-path: an argument of {} names {}, denied under "
                "every policy by harness spec 7.5.".format(tool, protected)
            )
        if not HYDRA_CONFIG["floor_lifted"]:
            hit = slack_violation_in_fetch(tool, tool_input)
            if hit:
                return False, (
                    "hydra/floor: the pattern floor refuses {}. Lift it for one run "
                    "with --allow-floor-escape.".format(hit)
                )

    # The same list, reached through the capability that carries no path.
    if capability == "shell":
        raw_command = tool_input.get(HYDRA_CONFIG["command_field"], "")
        # A shell row the gate cannot read is a shell row it cannot check: the
        # floor below runs on the string and nothing else. Measured 2026-09-22
        # on a compiled gate: `command: 123` and `command: null` both came back
        # `hydra/allow` with the floor never consulted.
        if not isinstance(raw_command, str):
            return False, (
                "hydra/bad-payload: {} carries no command string for the gate "
                "to read.".format(tool)
            )
        if raw_command:
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
        elif not paths:
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
            for scoped_path in paths:
                if not any(inside(scoped_path, root) for root in roots):
                    return False, (
                        "hydra/out-of-scope: policy {} allows {} inside {} and "
                        "{} is outside it.".format(
                            HYDRA_CONFIG["policy"],
                            capability,
                            " or ".join(allowed_scopes),
                            scoped_path,
                        )
                    )

    # An `mcp_call` the policy scopes to a list of servers, build ticket 140.
    # After the capability check, so a policy with no `mcp_call` still says
    # `capability`, and after the protected-path pass above, which it does
    # not replace.
    servers = mcp_server_list() if capability == "mcp_call" else None
    if servers is not None:
        hit = mcp_server_violation(tool, servers)
        if hit:
            return False, (
                "hydra/mcp-server: policy {} calls MCP tools only on {}, and {} "
                "has {}.".format(
                    HYDRA_CONFIG["policy"], ", ".join(servers), tool, hit
                )
            )

    # A shell the policy bounds to a list, map ticket 82 answer 3. After the
    # capability check, so a policy with no shell still says `capability`, and
    # before the floor, which it does not replace: the floor still reads git.
    commands = shell_list() if capability == "shell" else None
    if commands is not None:
        command = tool_input.get(HYDRA_CONFIG["command_field"], "")
        hit = bounded_shell_violation(command, commands, cwd)
        if hit:
            return False, (
                "hydra/shell-list: policy {} runs only the commands its list names, "
                "and this command has {}.".format(HYDRA_CONFIG["policy"], hit)
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


def apply_cwd_env(payload, inherited_pwd):
    """Set PWD from one batch payload's cwd before `evaluate`.

    Only the batch entry calls this. The live hook inherits the child's
    environment, where PWD is the run's working directory, and that path is
    left as it was. A batch cannot set PWD once on the process: two calls in
    one record can carry different cwds, and the gate expands `$PWD` out of
    its environment per call. A payload with no usable cwd gets the PWD the
    interpreter started with, not the previous payload's.
    """
    cwd = payload.get("cwd") if isinstance(payload, dict) else None
    if isinstance(cwd, str) and cwd:
        os.environ["PWD"] = cwd
    elif inherited_pwd is None:
        os.environ.pop("PWD", None)
    else:
        os.environ["PWD"] = inherited_pwd


def decide_payload(dialect, payload):
    """One answer dict for one hook payload."""
    if not isinstance(payload, dict):
        return answer_for(
            dialect, False, "hydra/bad-payload: the hook payload was not an object"
        )
    # Whatever `evaluate` raises is answered as a deny in the caller's own
    # shape. A traceback is exit 1 and nothing on stdout, which `codex` reads
    # as permission (SECURITY.md) and `claude` reads as a non-blocking error;
    # measured 2026-09-22 with `cwd: 5` on a compiled gate, which died in
    # `os.path.join`. That path is handled above now. Since build ticket 93,
    # `tests/gate.rs::evaluate_raise_is_answered_as_a_deny` drives this handler
    # by writing a non-string `scopes.worktree` into a compiled gate's config,
    # which makes `inside` raise.
    try:
        allowed, reason = evaluate(payload)
    except Exception as err:  # noqa: BLE001, the point is to catch everything
        allowed, reason = False, (
            "hydra/gate-error: the gate raised {} while evaluating this call, "
            "and a gate that cannot decide refuses.".format(type(err).__name__)
        )
    return answer_for(dialect, allowed, reason)


def decide_batch_entry(dialect, payload, inherited_pwd):
    """One answer for one payload of a batch, never a raise.

    `os.environ` refuses a cwd with a NUL in it by raising ValueError. Caught
    here, that is one deny for that payload rather than a traceback that
    loses the answers for every other payload in the batch.
    """
    try:
        apply_cwd_env(payload, inherited_pwd)
    except Exception as err:  # noqa: BLE001, one bad payload is one deny
        return answer_for(
            dialect,
            False,
            "hydra/gate-error: the gate raised {} setting PWD from this call's "
            "cwd, and a gate that cannot decide refuses.".format(type(err).__name__),
        )
    return decide_payload(dialect, payload)


def write_batch(dialect, answers):
    sys.stdout.write(json.dumps(answers))
    sys.stdout.write("\n")


def wrong_dialect_reason(dialect):
    return (
        "hydra/wrong-dialect: this gate was compiled for the {} dialect and "
        "was asked in {}.".format(HYDRA_CONFIG["dialect"], dialect)
    )


def check_dialect(dialect):
    """Return a wrong-dialect reason, or None when argv matches the compile."""
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
        return wrong_dialect_reason(dialect)
    return None


def main_single(dialect):
    wrong = check_dialect(dialect)
    if wrong is not None:
        respond(dialect, False, wrong)
        return

    try:
        raw = sys.stdin.read()
        payload = json.loads(raw) if raw.strip() else {}
    except (ValueError, RecursionError):
        # A document nested past the interpreter's limit raises RecursionError,
        # not ValueError, and used to be a traceback: measured 2026-09-22 at a
        # depth of five thousand.
        respond(dialect, False, "hydra/bad-payload: the hook payload was not JSON")
        return

    answer = decide_payload(dialect, payload)
    sys.stdout.write(json.dumps(answer))
    sys.stdout.write("\n")


def main_batch(dialect):
    wrong = check_dialect(dialect)
    if wrong is not None:
        write_batch(dialect, [answer_for(dialect, False, wrong)])
        return

    try:
        raw = sys.stdin.read()
        payloads = json.loads(raw) if raw.strip() else []
    except (ValueError, RecursionError):
        write_batch(
            dialect,
            [answer_for(dialect, False, "hydra/bad-payload: the hook payload was not JSON")],
        )
        return

    if not isinstance(payloads, list):
        write_batch(
            dialect,
            [
                answer_for(
                    dialect,
                    False,
                    "hydra/bad-payload: the batch entry expected a JSON array",
                )
            ],
        )
        return

    inherited_pwd = os.environ.get("PWD")
    write_batch(
        dialect,
        [decide_batch_entry(dialect, payload, inherited_pwd) for payload in payloads],
    )


def main():
    dialect = sys.argv[1] if len(sys.argv) > 1 else None
    batch = len(sys.argv) > 2 and sys.argv[2] == "batch"
    if batch:
        main_batch(dialect)
    else:
        main_single(dialect)


if __name__ == "__main__":
    main()
