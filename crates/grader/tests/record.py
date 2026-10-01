"""Record what `records.py` and `redact.py` give, so `differential.rs` can compare the Rust.

Build ticket 212, the first of the grader's port. From the checkout's root:

    python3 crates/grader/tests/record.py

It starts itself again with nothing in the environment, loads the two modules from
`integrations/grader/`, calls every function the ticket ported on the inputs below, and
writes `crates/grader/tests/fixtures/`. Each fixture holds its inputs beside what the Python
gave, so the Rust tests read one file per group and start no `python3`. Nothing is
normalized here.

`aliases.json` is copied to `fixtures/data/` once and read from there, by this script and
by the tests, so a new alias row does not change a recorded row. The twelve records of
`tests/fixtures/records/` are copied into `scan.json` as bytes for the same reason.

A second directory can be named, to record somewhere else, and `--fuzz N` records N
generated timestamps instead of the 600 the fixtures hold. `differential.rs` reads
`HYDRA_GRADER_FIXTURES` when it is set, which is how a longer fuzz was run once.
"""

import contextlib
import hashlib
import importlib.util
import io
import json
import os
import random
import shutil
import sys
import tempfile
from pathlib import Path
from unittest import mock

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
GRADER = REPO / "integrations" / "grader"

MARK = "HYDRA_GRADER_RECORDING"


def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


def outcome(call) -> dict:
    """What a call gave: its value, or the class of what it raised."""
    try:
        return {"value": call(), "raises": None}
    except Exception as err:  # noqa: BLE001
        return {"value": None, "raises": type(err).__name__}


def write(out: Path, name: str, value) -> None:
    (out / name).write_text(json.dumps(value, indent=1) + "\n", encoding="utf-8")


# ---------------------------------------------------------------------------- redact

LEGACY = "sk-abcdefghijklmnopqrstuvwxyz0123456789ABCD"
PROJECT = "sk-proj-planted_openai-key-value-0123"
SERVICE = "sk-svcacct-planted_openai-key-value-0123"
HOOK = "hooks.slack.com/services/T00/B00/planted"
PEM = (
    "-----BEGIN RSA PRIVATE KEY-----\n"
    "MIIEowIBAAKCAQEAplanted\n"
    "bodyline2\n"
    "-----END RSA PRIVATE KEY-----\n"
    "after"
)


def redact_strings() -> list[str]:
    # Every string `test_redact.py` redacts.
    texts = [
        "here is sk-ant-secret-key-value",
        "token sk-ant-planted end",
        "xAIzaSyPlanted",
        "ghp_plantedtoken",
        "ya29.plantedtoken",
        "AKIAPlantedSecretKey",
        "aws_secret_access_key=planted-aws-secret-value",
        "AWS_SECRET_ACCESS_KEY=planted-aws-env-secret",
        "xoxb-planted-slack-bot-token",
        "https://" + HOOK,
        "github_pat_plantedtokenvalue",
        "gho_plantedoauthtokenvalue",
        PEM,
        "-----BEGIN RSA PRIVATE KEY-----\r\nshort-body\r\n-----END RSA PRIVATE KEY-----",
        "before -----BEGIN PRIVATE KEY-----\nno footer",
        "before -----BEGIN RSA PRIVATE KEY-----\nMIIE body\n-----END RSA PRIVATE KEY----- after",
        PROJECT,
        LEGACY,
        SERVICE,
        f"key `{LEGACY}`, done",
        f"key `{PROJECT}`, done",
        f"key `{SERVICE}`, done",
        "the sk- prefix in prose is not a key",
        "task-management-system-v2",
        "disk-utilization-report",
        "risk-assessment-notes",
        "flask-sqlalchemy==3.1.1",
        "the `sk-proj-` prefix",
        "sk-ant-planted-secret",
        "AIzaSyPlanted",
        "\u00e9 " + LEGACY,
        "\u00e9\u00e9\u00e9 " + PROJECT + " tail",
    ]
    # What the ticket asks to be synthesized, and the edges of each rule.
    texts += [
        "",
        "nothing to hide",
        "\U0001f600 caf\u00e9 " + LEGACY + " \u00e9 " + PROJECT + " fin",
        "sk-admin-planted_openai-key-value-0123",
        "sk-proj-" + "a" * 20,
        "sk-proj-" + "a" * 19,
        "sk-" + "a" * 32,
        "sk-" + "a" * 31,
        "sk-" + "a" * 32 + "-tail",
        "sk-" + "a" * 32 + "_tail",
        "sk-" + "a" * 32 + ".tail",
        "sk-" + "a" * 32 + "\u00e9",
        "sk-" + "a" * 16 + "\u00e9" + "a" * 32,
        "sk-" + "\uff11" * 40,
        "xsk-" + "a" * 40,
        "_sk-" + "a" * 40,
        "-sk-" + "a" * 40,
        "\u00e9sk-" + "a" * 40,
        "sk-sk-" + "a" * 40,
        "sk-proj-sk-" + "a" * 40,
        LEGACY + " " + LEGACY,
        LEGACY + "," + PROJECT + ";" + SERVICE,
        "sk-proj-" + "a" * 25 + "\u00e9",
        "sk-svcacct-short",
        "sk-",
        "sk",
        "ends with sk-",
        "sk-ant-" + "a" * 40,
        "sk-ant-api03 'quoted' tail",
        'say "ghp_inquotes" and go',
        "say 'ghp_inquotes' and go",
        "ghp_one ghp_two\tghp_three\nghp_four",
        "AIzaAIzaAIza",
        "AKIA",
        "prefix-at-end ghp_",
        "ya29.a ya29.b",
        "aws_secret_access_key= spaced",
        "AWS_SECRET_ACCESS_KEY=\"quoted\"",
        "xoxb-1 and https://" + HOOK + " and http://" + HOOK,
        "http://" + HOOK,
        HOOK,
        "ftp://" + HOOK,
        "ttps://" + HOOK,
        "s://" + HOOK,
        "'https://" + HOOK + "'",
        "a " + HOOK + " b " + HOOK,
        "-----BEGIN",
        "-----BEGIN-----END",
        "-----BEGIN-----END-----",
        "-----BEGIN X-----END X----- tail -----BEGIN Y-----\nbody",
        "-----END RSA PRIVATE KEY----- then -----BEGIN RSA PRIVATE KEY-----",
        "x -----BEGIN A----- ghp_inside -----END A----- y ghp_outside z",
        "ghp_before -----BEGIN A-----\nbody\n-----END A-----",
        "github_pat_x and ghp_y and gho_z",
        "[REDACTED:github_pat] already",
        "ghp_lone\ud800surrogate",
        "\ud800 " + LEGACY + " \udfff",
        "ghp_nul\x00after",
        "tab\tAIzaTab\tend",
    ]
    # The plan's first risk: `str.isspace` ends a secret, and it counts U+001C to U+001F.
    for code in list(range(0x00, 0x30)) + [
        0x7F, 0x85, 0xA0, 0x1680, 0x180E, 0x2000, 0x200A, 0x200B, 0x2028, 0x2029, 0x202F,
        0x205F, 0x2060, 0x3000, 0xFEFF,
    ]:
        texts.append("ghp_planted" + chr(code) + "tail")
    for code in (0x1C, 0x1D, 0x1E, 0x1F, 0x20, 0x85, 0x3000):
        texts.append(LEGACY + chr(code) + "tail")
        texts.append("https://" + HOOK + chr(code) + "tail")
    return texts


REDACT_VALUES = [
    '{"output": "token sk-ant-planted end"}',
    '{"output": "xAIzaSyPlanted"}',
    '{"events": [{"output": "sk-ant-planted-secret"}, {"arguments": {"nested": "AIzaSyPlanted"}}]}',
    '{"sk-ant-planted": "AIzaPlanted"}',
    '{"ghp_one": 1, "ghp_two": 2, "plain": 3, "ghp_three": [4, "ghp_four"]}',
    '{"plain": 1, "ghp_x": 2, "[REDACTED:github_pat]": 3}',
    '["ghp_a", 1, 2.5, true, false, null, {"k": ["AKIAX", {"AKIAY": "xoxb-z"}]}]',
    '"ya29.top"',
    "12",
    "null",
    "[]",
    "{}",
    '{"nan": NaN, "big": 123456789012345678901234567890123456789012345, "f": 1e+16}',
    '{"\\ud800ghp_k": "ghp_v\\udc00 tail"}',
]


def record_redact(out: Path, redact) -> None:
    strings = []
    for text in redact_strings():
        strings.append(
            {
                "text": text,
                "spans": [list(span) for span in redact.openai_key_spans(text)],
                "redacted": redact.redact_string(text),
            }
        )
    values = []
    for source in REDACT_VALUES:
        values.append(
            {"source": source, "redacted": json.dumps(redact.redact_value(json.loads(source)))}
        )
    write(out, "redact.json", {"strings": strings, "values": values})


def record_isspace(out: Path) -> None:
    spaces = [c for c in range(0x110000) if chr(c).isspace()]
    (out / "isspace.txt").write_text("".join(f"{c:x}\n" for c in spaces))


# ---------------------------------------------------------------------------- records


def envelope(ident: str, **over) -> dict:
    row = dict(
        run_id=ident,
        parent_run_id=None,
        provider="claude",
        model="Sonnet 5",
        ts="2026-09-24T00:00:00Z",
    )
    row.update(over)
    return row


def start_line(ident: str, command="run", **over) -> dict:
    return dict(
        envelope(ident, **over),
        hydra=dict(
            type="run_start",
            command=command,
            policy="standard",
            task="do the thing",
            subscription="test",
            provider_version="1",
            launch_mode="recorded",
        ),
    )


def end_line(ident: str, ended_at="2026-09-24T00:00:10Z", **over) -> dict:
    return dict(envelope(ident, **over), hydra=dict(type="run_end", ended_at=ended_at, exit_code=0))


def dump_lines(*rows) -> bytes:
    return "".join(json.dumps(row) + "\n" for row in rows).encode()


def scan_inputs() -> list[tuple[str, bytes]]:
    cases = []
    corpus = REPO / "tests" / "fixtures" / "records"
    for path in sorted(corpus.glob("*/*.jsonl")):
        cases.append((str(path.relative_to(corpus)), path.read_bytes()))

    a, b = start_line("a"), end_line("a")
    whole = dump_lines(a, b)
    hydra = lambda value: json.dumps(dict(envelope("a"), hydra=value)).encode() + b"\n"  # noqa: E731
    first = lambda **over: dump_lines(start_line("a", **over), b)  # noqa: E731
    cases += [
        ("a run and its end", whole),
        ("a resume", dump_lines(start_line("a", command="resume", parent_run_id="p"), b)),
        ("no end", dump_lines(a)),
        ("a torn tail", whole + b'{"run_id": "a", "hy'),
        ("a torn tail that parses", whole + json.dumps(b).encode()),
        ("a torn first line", json.dumps(a).encode()),
        ("a torn first line of nothing but a brace", b"{"),
        ("empty", b""),
        ("one newline", b"\n"),
        ("a blank line between two", json.dumps(a).encode() + b"\n\n" + json.dumps(b).encode() + b"\n"),
        ("a garbled interior line", dump_lines(a) + b'{"run_id": "a", "hy\n' + dump_lines(b)),
        ("a garbled last line", whole + b"not json\n"),
        ("a garbled first line", b"}{\n" + whole),
        ("a trailing comma", dump_lines(a) + b'{"a": 1,}\n'),
        ("extra data on a line", dump_lines(a) + b'{"a": 1} {"b": 2}\n'),
        ("line 0 is a list", b"[1, 2]\n" + whole),
        ("line 0 is a number", b"7\n" + whole),
        ("line 0 is a string", b'"run"\n' + whole),
        ("line 0 is null", b"null\n" + whole),
        ("line 0 is an empty object", b"{}\n" + whole),
        ("a later line is a list", dump_lines(a) + b"[1]\n" + dump_lines(b)),
        ("a later line is a number", dump_lines(a) + b"7\n"),
        ("crlf endings", whole.replace(b"\n", b"\r\n")),
        ("a lone carriage return ends the scan", json.dumps(a).encode() + b"\r" + dump_lines(b)),
        ("a carriage return first", b"\r" + whole),
        ("a vertical tab is no line end", json.dumps(a).encode() + b" \x0b \n" + dump_lines(b)),
        ("a form feed is no line end", json.dumps(a).encode() + b"\x0c\n"),
        ("spaces around a line", b"  " + json.dumps(a).encode() + b" \t\n" + dump_lines(b)),
        ("command evolve", dump_lines(start_line("a", command="evolve"), b)),
        ("command missing", hydra({"type": "run_start"})),
        ("command null", hydra({"type": "run_start", "command": None})),
        ("command a list", hydra({"type": "run_start", "command": ["run"]})),
        ("hydra null", hydra(None)),
        ("hydra an empty list", hydra([])),
        ("hydra a list", hydra(["run"])),
        ("hydra a string", hydra("run")),
        ("hydra a number", hydra(3)),
        ("hydra zero", hydra(0)),
        ("hydra missing", dump_lines(envelope("a"))),
        ("provider null", first(provider=None)),
        ("provider empty", first(provider="")),
        ("provider a number", first(provider=7)),
        ("provider zero and hydra a string", json.dumps(dict(envelope("a", provider=0), hydra="x")).encode() + b"\n"),
        ("run id empty", first(run_id="")),
        ("run id null", first(run_id=None)),
        ("run id a number", first(run_id=12)),
        ("run id a list", first(run_id=["a"])),
        ("model null", first(model=None)),
        ("a model no alias names", first(model="nobody's model")),
        ("non-ascii text", first(model="caf\u00e9 \U0001f600")),
        ("non-ascii bytes, unescaped", json.dumps(start_line("a", model="caf\u00e9 \U0001f600"), ensure_ascii=False).encode() + b"\n"),
        ("a lone surrogate escape", dump_lines(a) + b'{"x": "\\ud800"}\n'),
        ("a surrogate in utf-8", dump_lines(a) + b'{"x": "\xed\xa0\x80"}\n'),
        ("NaN on a line", dump_lines(a) + b'{"x": NaN, "y": -Infinity}\n'),
        ("a duplicate key", b'{"run_id": "a", "run_id": "b", "provider": "claude", "hydra": {"command": "run"}}\n'),
        ("a utf-8 mark on line 0", b"\xef\xbb\xbf" + whole),
        ("a utf-8 mark on line 1", dump_lines(a) + b"\xef\xbb\xbf" + dump_lines(b)),
        ("an integer of 4300 digits", dump_lines(a) + b"9" * 4300 + b"\n"),
        ("an integer of 4301 digits", dump_lines(a) + b"9" * 4301 + b"\n"),
        ("utf-16 with a mark", json.dumps(a).encode("utf-16") + "\n".encode("utf-16-le")),
        ("utf-16-le, split inside the newline", json.dumps(a).encode("utf-16-le") + "\n".encode("utf-16-le")),
        ("utf-32-le", json.dumps(a).encode("utf-32-le") + "\n".encode("utf-32-le")),
        ("a nul byte", dump_lines(a) + b"\x00\n"),
    ]
    # Invalid UTF-8, each way CPython's decoder names.
    bad = {
        "a start byte": b'{"x": "\xff"}',
        "a start byte c0": b'{"x": "\xc0\x80"}',
        "a start byte f8": b'{"x": "\xf8\x88\x80\x80\x80"}',
        "a stray continuation": b'{"x": "\x80"}',
        "latin-1": b'{"x": "caf\xe9"}',
        "two of three": b'{"x": "\xe2\x82"}',
        "three of four": b'{"x": "\xf0\x9f\x98"}',
        "two of four": b'{"x": "\xf0\x9f"}',
        "an overlong e0": b'{"x": "\xe0\x80\x80"}',
        "past the last code point": b'{"x": "\xf4\x90\x80\x80"}',
        "a surrogate cut short": b'{"x": "\xed\xa0"}',
        "an overlong f0": b'{"x": "\xf0\x80\x80\x80"}',
        "the end, one of two": b'{"x": "\xc3',
        "the end, one of three": b'{"x": "\xe2',
        "the end, two of three": b'{"x": "\xe2\x82',
        "the end, one of four": b'{"x": "\xf0',
        "the end, three of four": b'{"x": "\xf0\x9f\x98',
        "after an accent": b'{"x": "\xc3\xa9\xff"}',
        "after a mark": b'\xef\xbb\xbf{"x": "\xff"}',
    }
    for name, line in bad.items():
        cases.append((f"invalid utf-8, {name}, line 0", line + b"\n" + whole))
        cases.append((f"invalid utf-8, {name}, line 1", dump_lines(a) + line + b"\n"))
    wide = {
        "utf-16-le with an odd byte": b'"\x00a\x00"\x00',
        "utf-16 marked with an odd byte": b'\xff\xfe"\x00a\x00"',
        "utf-16-be marked with an odd byte": b'\xfe\xff\x00"\x00a\x00"\x00',
        "utf-16-be with an odd byte": b'\x00"\x00a\x00"\x00',
        "utf-16-le, a high surrogate then an odd byte": b'"\x00\x00\xd8a',
        "utf-16-le, a lone surrogate": b'"\x00\x00\xd8"\x00',
        "utf-32-le with one byte over": b'"\x00\x00\x00a\x00\x00\x00"\x00\x00\x00\x01',
        "utf-32-le with three bytes over": b'"\x00\x00\x00a\x00\x00\x00"\x00\x00\x00\x01\x02\x03',
        "utf-32-le past the last code point": b'"\x00\x00\x00\x00\x00\x11\x00"\x00\x00\x00',
        "utf-32-le past the last code point, then two bytes over": b'"\x00\x00\x00\x00\x00\x11\x00"\x00\x00\x00\x01\x02',
        "utf-32-le, a surrogate": b'"\x00\x00\x00\x00\xd8\x00\x00"\x00\x00\x00',
        "utf-32-be with two bytes over": b'\x00\x00\x00"\x00\x00\x00a\x00\x00\x00"\x00\x00',
        "utf-32 marked past the last code point": b'\xff\xfe\x00\x00"\x00\x00\x00\x00\x00\x11\x00',
        "utf-32-be marked with two bytes over": b'\x00\x00\xfe\xff\x00\x00\x00"\x00\x00',
    }
    for name, line in wide.items():
        cases.append((f"{name}", line + b"\n"))
    return cases


ANSWERS = [
    '{"projection": {"outcome": "completed", "usage": {"usage_source": "provider", "counts": {"raw_input": 100, "cache_read_tokens": 80, "output_tokens": 20}}}, "events": []}',
    '{"projection": {"outcome": null, "usage": {"counts": {}}}}',
    '{"projection": null}',
    "{}",
    '{"projection": {"outcome": "failed", "usage": null}}',
    '{"projection": {"outcome": ["odd"], "usage": {"counts": null}}}',
    '{"projection": {"usage": {"counts": 0}}}',
    '{"projection": "text"}',
    '{"projection": {"usage": [1]}}',
]


def record_scan(out: Path, records, aliases: dict) -> None:
    cases = []
    with tempfile.TemporaryDirectory() as tmp:
        for index, (name, data) in enumerate(scan_inputs()):
            path = Path(tmp) / f"{index:03d}.jsonl"
            path.write_bytes(data)
            case = {"name": name, "file": path.name, "data": data.hex()}

            try:
                lines, covered = records.committed_lines(data)
                case["committed"] = {"lines": json.dumps(lines), "covered": covered}
            except Exception as err:  # noqa: BLE001
                case["committed"] = {"raises": type(err).__name__}

            stderr = io.StringIO()
            item = None
            case["raises"] = None
            with contextlib.redirect_stderr(stderr):
                try:
                    item = records.scan_record(path)
                except Exception as err:  # noqa: BLE001
                    case["raises"] = type(err).__name__
            case["stderr"] = stderr.getvalue()
            case["scanned"] = None
            if item is not None:
                assert item["path"] == path and item["data"] == data
                case["scanned"] = {
                    "run_id": json.dumps(item["run_id"]),
                    "first": json.dumps(item["first"]),
                    "lines": json.dumps(item["lines"]),
                    "fingerprint": item["fingerprint"],
                }
                rows = []
                for source in ANSWERS:
                    answer = json.loads(source)
                    got = outcome(
                        lambda: json.dumps(
                            records.record_row(item["first"], item["lines"], answer, aliases)
                        )
                    )
                    rows.append({"answer": source, "row": got["value"], "raises": got["raises"]})
                case["rows"] = rows
            cases.append(case)
    write(out, "scan.json", cases)


# What `record_row` and `duration_seconds` do with a first line and lines that never came
# through `scan_record`.
ROW_CASES = [
    ('{"run_id": "a", "provider": "claude", "model": "Sonnet 5", "hydra": {"policy": "standard", "task": "t"}}', "[]"),
    ('{"run_id": "a", "provider": "claude", "model": "Haiku 4.5"}', "[]"),
    ('{"run_id": "a", "provider": "nobody", "model": "nothing", "hydra": {}}', "[]"),
    ('{"run_id": "a", "provider": "claude"}', "[]"),
    ('{"provider": "claude", "model": null, "hydra": {}}', "[]"),
    ('{"run_id": "a", "model": "x", "hydra": {}}', "[]"),
    ('{"run_id": "a", "provider": "claude", "model": ["x"], "hydra": {}}', "[]"),
    ('{"run_id": "a", "provider": {"p": 1}, "model": "x", "hydra": {}}', "[]"),
    ('{"run_id": "a", "provider": "claude", "model": "x", "hydra": null}', "[]"),
    ('{"run_id": "a", "provider": "claude", "model": "x", "hydra": "text"}', "[]"),
    ('{"run_id": "a", "provider": "claude", "model": "x", "hydra": []}', "[]"),
    ('{"run_id": "a", "provider": 1, "model": 1.0, "hydra": {"policy": 2, "task": ["t"]}}', "[]"),
    ('{"run_id": "a", "provider": true, "model": NaN, "hydra": {}}', "[]"),
    ('{"run_id": {"deep": [1]}, "provider": "claude", "model": "caf\\u00e9 \\ud83d\\ude00", "hydra": {"task": "sk-ant-kept-as-is"}}', "[]"),
    ("[]", "[]"),
    ('"text"', "[]"),
]

DURATION_LINES = [
    # (first, lines)
    ('{"ts": "2026-09-24T00:00:00Z"}', "[]"),
    ('{"ts": "2026-09-24T00:00:00Z"}', '[{"hydra": {"type": "run_start"}}]'),
    ('{"ts": "2026-09-24T00:00:00Z"}', '[{"hydra": {"type": "run_end"}}]'),
    ('{"ts": "2026-09-24T00:00:00Z"}', '[{"hydra": {"type": "run_end", "ended_at": null}}]'),
    ('{"ts": "2026-09-24T00:00:00Z"}', '[{"hydra": {"type": "run_end", "ended_at": ""}}]'),
    ('{"ts": "2026-09-24T00:00:00Z"}', '[{"hydra": {"type": "run_end", "ended_at": 0}}]'),
    ('{"ts": "2026-09-24T00:00:00Z"}', '[{"hydra": {"type": "run_end", "ended_at": 20260925}}]'),
    ('{"ts": 20260924}', '[{"hydra": {"type": "run_end", "ended_at": 20260925}}]'),
    ('{"ts": 20260924}', '[{"hydra": {"type": "run_end", "ended_at": "2026-09-25"}}]'),
    ('{"ts": 2026092.0}', '[{"hydra": {"type": "run_end", "ended_at": "2026-09-25"}}]'),
    ('{"ts": null}', '[{"hydra": {"type": "run_end", "ended_at": "2026-09-25"}}]'),
    ("{}", '[{"hydra": {"type": "run_end", "ended_at": "2026-09-25"}}]'),
    ('{"ts": true}', '[{"hydra": {"type": "run_end", "ended_at": "2026-09-25"}}]'),
    ('{"ts": ["2026-09-24"]}', '[{"hydra": {"type": "run_end", "ended_at": "2026-09-25"}}]'),
    ('{"ts": "2026-09-24"}', '[{"hydra": {"type": "run_end", "ended_at": ["2026-09-25"]}}]'),
    ('{"ts": "2026-09-24"}', '[{"hydra": {"type": "run_end", "ended_at": {"at": 1}}}]'),
    ('{"ts": "2026-09-24"}', '[{"hydra": {"type": "run_end", "ended_at": true}}]'),
    ('{"ts": "2026-09-24T00:00:00Z"}', '[{"hydra": {"type": "run_end", "ended_at": "2026-09-24T00:00:10Z"}}, {"hydra": {"type": "run_end", "ended_at": "2026-09-24T00:00:20Z"}}]'),
    ('{"ts": "2026-09-24T00:00:00Z"}', '[{"hydra": {"type": "run_end", "ended_at": "2026-09-24T00:00:10Z"}}, {"hydra": {"type": "other"}}, {"other": 1}]'),
    ('{"ts": "2026-09-24T00:00:00Z"}', '[{"hydra": {"type": "run_end", "ended_at": "2026-09-24T00:00:10Z"}}, {"hydra": {"type": "run_end"}}]'),
    ('{"ts": "2026-09-24T00:00:00Z"}', '[{"hydra": {"type": ["run_end"], "ended_at": "2026-09-24T00:00:10Z"}}]'),
    ('{"ts": "2026-09-24T00:00:00Z"}', '[{"hydra": null}]'),
    ('{"ts": "2026-09-24T00:00:00Z"}', '[{"hydra": "text"}]'),
    ('{"ts": "2026-09-24T00:00:00Z"}', '[{"hydra": []}]'),
    ('{"ts": "2026-09-24T00:00:00Z"}', "[[1]]"),
    ('{"ts": "2026-09-24T00:00:00Z"}', '["text"]'),
    ('{"ts": "2026-09-24T00:00:00Z"}', '[{"hydra": null}, {"hydra": {"type": "run_end", "ended_at": "2026-09-24T00:00:10Z"}}]'),
    ('{"ts": "2026-09-24T00:00:00Z"}', '[7, {"hydra": {"type": "run_end", "ended_at": "2026-09-24T00:00:10Z"}}]'),
    ('"text"', '[{"hydra": {"type": "run_end", "ended_at": "2026-09-24T00:00:10Z"}}]'),
    ("[]", "[]"),
]

STAMPS = [
    "2026-09-24T00:00:00Z",
    "2026-09-24T00:00:10Z",
    "2026-09-05T20:51:39.370Z",
    "2026-09-05T20:52:01.905Z",
    "2026-09-05T20:51:39.369329Z",
    "2026-09-24T10:00:00+05:30",
    "2026-09-24 10:00:00-08:00",
    "2026-09-24T10:00:00",
    "2026-09-24T10:00",
    "2026-09-24T10",
    "2026-09-24",
    "20260924T100000",
    "20260924T1000",
    "20260924",
    "2026-W39-4T10:00",
    "2026-W39-4",
    "2026-W39",
    "2026W394",
    "2026W39",
    "2026W394T1000Z",
    "2026-09-24T24:00:00",
    "2026-12-31T24:00:00Z",
    "2024-02-29T12:00:00.5+00:00:30",
    "2026-09-24T10:00:00.123456789-23:59:59.999999",
    "2026-09-24T10:00:00,25+0530",
    "2026-09-24T10:00:00.1+05",
    "0001-01-01T00:00:00",
    "0001-01-01T00:00:00.000001Z",
    "9999-12-31T23:59:59.999999",
    "9999-12-31T23:59:59.999999Z",
    "2026-09-24\u00e910:00:00Z",
    "2026-09-24\U0001f60010:00:00Z",
    "2026-09-24\ud80010:00:00Z",
    "2026-W53-1",
    "2020-W53-7T00:00Z",
    "2026-09-24T10:00:00-00:00",
    "2026-09-24T10:00:00+00:00:00.000001",
]

ODD_STAMPS = [
    "",
    "Z",
    "2026",
    "2026-09",
    "2026-9-24",
    "2026092",
    "202609241",
    "2026-09-24T",
    "2026-09-24TZ",
    "2026-09-24T10:00:00:123",
    "2026-09-24T100000123",
    "2026-09-24T10:00:00.+00:00",
    "2026-09-24T10:00:00.",
    "2026-09-24T10:00:00,",
    "2026-09-24T10:00.5",
    "2026-09-24T10.5",
    "2026-09-24T1000:00",
    "2026-09-24T10:0000",
    "2026-09-24T10:00:00+24:00",
    "2026-09-24T10:00:00-24:00",
    "2026-09-24T10:00:00+23:59:59.999999",
    "2026-09-24T10:00:00+23:60",
    "2026-09-24T10:00:00+00:60",
    "2026-09-24T10:00:00+00:99",
    "2026-09-24T10:00:00+00:00:60",
    "2026-09-24T10:00:00+99",
    "2026-09-24T10:00:00+",
    "2026-09-24T10:00:00+0",
    "2026-09-24T10:00:00+053",
    "2026-09-24T10:00:00+05:3",
    "2026-09-24T10:00:00+05:30:",
    "2026-09-24T10:00:00+05:30:15.",
    "2026-09-24T10:00:00+05:30x",
    "2026-09-24T10:00:00+00:00+00:00",
    "2026-09-24T10:00:00ZZ",
    "2026-09-24T10:00:00Zx",
    "2026-09-24T10:00:00 Z",
    "Z026-09-24T10:00:00",
    "2026-09-24T24:00:01",
    "2026-09-24T24:01",
    "2026-09-24T24:00:00.000001",
    "2026-09-24T24",
    "2026-09-24T25:00",
    "2026-09-24T23:60",
    "2026-09-24T23:59:60",
    "2026-02-28T24:00",
    "2026-02-29T00:00",
    "2026-02-29T24:00",
    "2024-02-29T24:00",
    "2026-13-01T24:00",
    "2026-00-10",
    "2026-01-00",
    "2026-01-32",
    "2026-04-31",
    "9999-12-31T24:00",
    "0000-01-01",
    "2026-W00-1",
    "2026-W54-1",
    "2026-W53-8",
    "2026-W39-0",
    "2026-W39-",
    "2026-W3",
    "2026-W39-4-",
    "2026-W39-10:00",
    "2026-W39410",
    "2026W39410",
    "2026W394100",
    "2026W3941000",
    "2026W39-4",
    "2026-W394",
    "0001-W01-1",
    "9999-W52-7",
    "9999-W52-5",
    "20260924T100000.123456789+0530",
    "2026-09-24t10:00:00",
    "2026-09-24_10:00:00",
    "2026-09-2410:00:00",
    " 2026-09-24T10:00:00",
    "2026-09-24T10:00:00 ",
    "2026-09-24T10:00:00\n",
    "2026-09-24T10:00:00\x00",
    "2026-09-24\x0010:00:00",
    "2026\u00e909-24",
    "\uff12026-09-24",
    "2026-09-24T1\uff10:00",
    "2026-09-\ud80024T10:00",
    "2026W39\ud80010",
    "2026W394\ud80010",
    "2026-09-24\ud800\ud80010",
    "+2026-09-24",
    "-026-09-24",
    "2026-09-24T-1:00",
    "2026-09-24T+10:00",
    "2026-09-24T10-05",
    "2026-09-24T10:00-05:00:00.5",
    "2026-09-24T10:00:00.123+05:00:00.1234567",
    "None",
    "20260924",
    "2026092410",
]


def mutate(rng: random.Random, text: str) -> str:
    alphabet = "0123456789-:.,TZW+ 0123456789"
    at = rng.randrange(len(text) + 1)
    kind = rng.randrange(4)
    if kind == 0 and text:
        at = min(at, len(text) - 1)
        return text[:at] + text[at + 1 :]
    if kind == 1:
        return text[:at] + rng.choice(alphabet) + text[at:]
    if kind == 2 and text:
        at = min(at, len(text) - 1)
        return text[:at] + rng.choice(alphabet) + text[at + 1 :]
    cut = rng.randrange(len(text) + 1)
    return text[:cut]


def record_rows(out: Path, records, aliases: dict, fuzz: int) -> None:
    rows = []
    for first, lines in ROW_CASES:
        for answer in (ANSWERS[0], ANSWERS[2]):
            got = outcome(
                lambda: json.dumps(
                    records.record_row(
                        json.loads(first), json.loads(lines), json.loads(answer), aliases
                    )
                )
            )
            rows.append(
                {"first": first, "lines": lines, "answer": answer, "row": got["value"], "raises": got["raises"]}
            )

    durations = []

    def duration(first: str, lines: str) -> None:
        got = outcome(
            lambda: json.dumps(records.duration_seconds(json.loads(first), json.loads(lines)))
        )
        durations.append(
            {"first": first, "lines": lines, "seconds": got["value"], "raises": got["raises"]}
        )

    def between(started: str, ended: str) -> None:
        duration(
            json.dumps({"ts": started}),
            json.dumps([{"hydra": {"type": "run_end", "ended_at": ended}}]),
        )

    for first, lines in DURATION_LINES:
        duration(first, lines)
    for started in STAMPS:
        for ended in STAMPS:
            between(started, ended)
    for odd in ODD_STAMPS:
        between(odd, "2026-09-25T00:00:00")
        between("2026-09-23T00:00:00+00:00", odd)
    rng = random.Random(212)
    for _ in range(fuzz):
        started = rng.choice(STAMPS + ODD_STAMPS)
        for _ in range(rng.randrange(1, 3)):
            started = mutate(rng, started)
        ended = rng.choice(STAMPS)
        if rng.randrange(3) == 0:
            ended = mutate(rng, ended)
        between(started, ended)
    write(out, "rows.json", {"rows": rows, "durations": durations})


WRITE_VALUES = [
    "{}",
    "[]",
    "null",
    '"text"',
    "0",
    '{"a": 1, "b": [1, 2, {"c": null}], "d": {"e": true, "f": false}, "g": []}',
    '{"z": 1, "a": 2, "m": {"z": 1, "a": 2}}',
    '{"caf\\u00e9": "\\ud83d\\ude00 \\u2028 \\u007f \\u0000 \\" \\\\ / \\b\\f\\n\\r\\t", "lone": "\\ud800"}',
    "[0.1, 1e16, 1e-05, 1.5e300, 5e-324, -0.0, 0.0, 1e22, 123456789.123456789, 1.0, 100.0, 9007199254740993, -1]",
    "[123456789012345678901234567890123456789012345678901234567890, -123456789012345678901234567890123456789012]",
    '{"cursor": {"a": "f1", "b": "f2"}, "collected_at": "2026-09-24T00:00:00Z"}',
    '{"x": NaN}',
    "[1, [2, [Infinity]]]",
    "-Infinity",
]

WRITE_ROWS = [
    "[]",
    '[{"run_id": "a", "tokens": {"raw_input": 100}, "duration_seconds": 10.0}]',
    '[{"run_id": "a"}, {"run_id": "b", "task": "caf\\u00e9 \\ud83d\\ude00"}, [1, 2], "text", 7, null]',
    '[{"run_id": "a"}, {"run_id": "b", "p": NaN}, {"run_id": "c"}]',
    '[{"p": Infinity}]',
    '[{"run_id": "a"}, {"run_id": "b"}, {"deep": [{"p": -Infinity}]}]',
]

READ_JSONL = [
    None,
    b"",
    b'{"a": 1}\n{"b": 2}\n',
    b'{"a": 1}\n{"b": 2}',
    b'{"a": 1}\n{"b": 2}\n{"c"',
    b'{"a": 1}\nnot json\n{"b": 2}\n',
    b'{"a": 1}\n\n',
    b'[1]\n"text"\n7\nnull\n',
    b'{"a": "\xff"}\n',
    b'{"a": 1}\r\n{"b": 2}\r{"c": 3}\n',
    b"9" * 4301 + b"\n",
]

READ_JSON = [
    None,
    b"{}",
    b'{"a": {"fingerprint": "f", "answer": {"projection": {"outcome": null}}}}',
    b'  [1, 2]\n',
    b"",
    b"not json",
    b'{"a": 1} trailing',
    b'\xef\xbb\xbf{"a": 1}',
    b'{"a": "\xff"}',
    b'{"a": "caf\xc3\xa9"}',
    b'{"a":\r\n1}',
]


def listing(root: Path) -> list[dict]:
    return [
        {"path": str(p.relative_to(root)), "text": p.read_text(encoding="utf-8")}
        for p in sorted(root.rglob("*"))
        if p.is_file()
    ]


def record_files(out: Path, records) -> None:
    writes = []
    for kind, sources, call in (
        ("json", WRITE_VALUES, records.write_json),
        ("jsonl", WRITE_ROWS, records.write_jsonl),
    ):
        for source in sources:
            # Twice: into a directory that does not exist yet, and over a file that does.
            for target, before in (("deep/er/out." + kind, None), ("out." + kind, "kept\n"), ("plain", None)):
                with tempfile.TemporaryDirectory() as tmp:
                    root = Path(tmp)
                    if before is not None:
                        (root / target).write_text(before)
                    got = outcome(lambda: call(root / target, json.loads(source)))
                    writes.append(
                        {
                            "kind": kind,
                            "source": source,
                            "target": target,
                            "before": before,
                            "raises": got["raises"],
                            "files": listing(root),
                        }
                    )

    reads = []
    for kind, inputs in (("jsonl", READ_JSONL), ("json", READ_JSON)):
        for data in inputs:
            with tempfile.TemporaryDirectory() as tmp:
                path = Path(tmp) / ("in." + kind)
                if data is not None:
                    path.write_bytes(data)
                if kind == "jsonl":
                    got = outcome(lambda: json.dumps(records.read_jsonl(path)))
                else:
                    got = outcome(lambda: json.dumps(records.read_json(path, {"default": 1})))
                reads.append(
                    {
                        "kind": kind,
                        "data": None if data is None else data.hex(),
                        "value": got["value"],
                        "raises": got["raises"],
                    }
                )
    write(out, "files.json", {"writes": writes, "reads": reads})


PREAMBLE = (
    "Hydra launched this run and wrote its instructions at spawn. There are three.\n"
    "\n"
    "Work inside it, and if the task cannot be done inside it,\n"
    "say so and stop.\n"
)

PREAMBLE_FILES = [
    None,
    PREAMBLE.encode(),
    PREAMBLE.encode() + b"\n\n\n",
    PREAMBLE.rstrip("\n").encode(),
    b"",
    b"\n",
    PREAMBLE.replace("\n", "\r\n").encode(),
    b"one\rtwo\r\n\r\n",
    b"trailing spaces  \n \n",
    "caf\u00e9 \U0001f600\n".encode(),
    b"\xef\xbb\xbfmarked\n",
    b"latin \xe9\n",
]


def record_preamble(out: Path, records) -> None:
    current = PREAMBLE.rstrip("\n") + "\n\n"
    old = "Hydra launched this run and wrote its instructions at spawn. An older wording,\nsay so and stop.\n\n"
    strips = []
    for task, now in [
        (current + "the task", current),
        (current + "the task", None),
        (current + "the task", ""),
        (current, current),
        (current.rstrip("\n"), current),
        ("the task", current),
        ("", current),
        ("", None),
        (old + "the task", current),
        (old + "the task", None),
        (old + "say so and stop.\n\nthe task", None),
        (old.rstrip("\n") + "the task", None),
        ("x" + old + "the task", None),
        ("Hydra launched this run and wrote its instructions at spawn.", None),
        ("Hydra launched this run and wrote its instructions at spawn.say so and stop.\n\n", None),
        ("caf\u00e9 \U0001f600\n\nthe task \u00e9", "caf\u00e9 \U0001f600\n\n"),
        ("\ud800 lone\n\nthe task", "\ud800 lone\n\n"),
        (old + "caf\u00e9 \U0001f600 \ud800", "something else\n\n"),
    ]:
        text, stripped = records.strip_preamble(task, now)
        strips.append({"task": task, "current": now, "text": text, "stripped": stripped})

    currents = []
    for data in PREAMBLE_FILES:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            if data is not None:
                (root / "config").mkdir()
                (root / "config" / "preamble.md").write_bytes(data)
            with mock.patch.dict(os.environ, {"HYDRA_ROOT": str(root)}, clear=True):
                got = outcome(records.current_preamble)
            currents.append(
                {
                    "data": None if data is None else data.hex(),
                    "preamble": got["value"],
                    "raises": got["raises"],
                }
            )
    with tempfile.TemporaryDirectory() as tmp:
        (Path(tmp) / "config" / "preamble.md").mkdir(parents=True)
        with mock.patch.dict(os.environ, {"HYDRA_ROOT": tmp}, clear=True):
            got = outcome(records.current_preamble)
        currents.append({"data": "directory", "preamble": got["value"], "raises": got["raises"]})

    homes = []
    for env in [
        {"HYDRA_ROOT": "/named/root", "HOME": "/users/x"},
        {"HYDRA_ROOT": "/named/root/", "HOME": "/users/x"},
        {"HYDRA_ROOT": "relative/root", "HOME": "/users/x"},
        {"HYDRA_ROOT": "", "HOME": "/users/x"},
        {"HOME": "/users/x"},
        {"HOME": "/users/x/"},
        {"HOME": ""},
        {"HOME": "/"},
        {"HOME": "relative"},
        {"HYDRA_GRADER_HOME": "/named/grader", "HOME": "/users/x"},
        {"HYDRA_GRADER_HOME": "", "HYDRA_ROOT": "/named/root", "HOME": "/users/x"},
        {"HYDRA_GRADER_HOME": "/named/grader", "HYDRA_ROOT": "/named/root", "HOME": "/users/x"},
        {"HYDRA_GRADER_HOME": "relative/./grader//x/", "HOME": "/users/x"},
    ]:
        with mock.patch.dict(os.environ, env, clear=True):
            homes.append(
                {"env": env, "root": str(records.hydra_root()), "home": str(records.grader_home())}
            )
    write(out, "preamble.json", {"strips": strips, "currents": currents, "homes": homes})


def main() -> int:
    if os.environ.get(MARK) != "1":
        # Nothing of the recorder's shell reaches the Python: no locale, no `PATH`, no `HOME`.
        os.execve(
            sys.executable,
            [sys.executable, str(Path(__file__).resolve()), *sys.argv[1:]],
            {MARK: "1", "PYTHONDONTWRITEBYTECODE": "1"},
        )
    args = sys.argv[1:]
    fuzz = 600
    if "--fuzz" in args:
        at = args.index("--fuzz")
        fuzz = int(args[at + 1])
        del args[at : at + 2]
    out = Path(args[0]).resolve() if args else HERE / "fixtures"
    (out / "data").mkdir(parents=True, exist_ok=True)

    frozen = out / "data" / "aliases.json"
    if not frozen.exists():
        shutil.copyfile(REPO / "integrations" / "puppets" / "aliases.json", frozen)

    sys.path.insert(0, str(GRADER))
    records = load_module("recorded_records", GRADER / "records.py")
    redact = load_module("recorded_redact", GRADER / "redact.py")
    aliases = records.load_aliases(frozen)

    (out / "version.txt").write_text(sys.version + "\n")
    record_isspace(out)
    record_redact(out, redact)
    record_scan(out, records, aliases)
    record_rows(out, records, aliases, fuzz)
    record_files(out, records)
    record_preamble(out, records)
    # Build ticket 213: `baseline.py`'s `train` and `score`, in the file beside this one.
    load_module("record_baseline", HERE / "record_baseline.py").record(out, REPO, scan_inputs())
    sys.path.insert(0, str(HERE))
    for name in ("queue", "jev"):
        module = load_module("record_" + name, HERE / ("record_" + name + ".py"))
        write(out, name + ".json", {"version": sys.version, "cases": module.record()})

    # Build ticket 215: `monitor.py`'s `features`, `train` and `score`, in its own file too.
    load_module("record_monitor", HERE / "record_monitor.py").record(out, REPO, scan_inputs())

    load_module("record_triage", HERE / "record_triage.py").record(out, REPO, scan_inputs())
    # Build ticket 214: `labels.py` and `grader.py`, in the file beside this one.
    labels = load_module("recorded_labels", GRADER / "labels.py")
    load_module("record_collect", HERE / "record_collect.py").record(out, REPO, labels)
    # Build ticket 218: `judge.py`'s `label` and `agree`, in the file beside this one.
    load_module("record_judge", HERE / "record_judge.py").record(out, REPO)

    digest = hashlib.sha256()
    for path in sorted(p for p in out.rglob("*") if p.is_file()):
        digest.update(path.read_bytes())
    print(f"{sys.version.split()[0]} {digest.hexdigest()}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
