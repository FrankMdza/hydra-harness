#!/usr/bin/env python3
"""Record SHA-256 of CPython 3.14's math answers for ticket 211. Keep in sync with `math_probe.rs`."""
from __future__ import annotations

import hashlib
import math
import struct
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
OUT = HERE / "fixtures" / "math"

SEED = 21_100_943_147
UNARY_COUNT = 1_000_000
BLOCK_SIZE = 10_000
SUM_LISTS = 10_000
LOG_EPS = 1e-12
# Build 213: `baseline.py` takes `log1p` of a task's length in characters, an integer far past
# the 0 to 707 above. Every count up to COUNT_DENSE, then COUNT_SPARSE of them below COUNT_MAX.
COUNT_SEED = SEED + 213
COUNT_DENSE = 200_000
COUNT_SPARSE = 100_000
COUNT_MAX = 100_000_000


class SplitMix64:
    def __init__(self, seed: int) -> None:
        self.state = seed & ((1 << 64) - 1)

    def next_u64(self) -> int:
        self.state = (self.state + 0x9E3779B97F4A7C15) & ((1 << 64) - 1)
        z = self.state
        z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9 & ((1 << 64) - 1)
        z = (z ^ (z >> 27)) * 0x94D049BB133111EB & ((1 << 64) - 1)
        return z ^ (z >> 31)

    def next_u53(self) -> int:
        return self.next_u64() >> 11


def unary_input(i: int, rng: SplitMix64) -> float:
    slot = i % 11
    if slot == 0:
        return math.ldexp(1.0, -1022)
    if slot == 1:
        return -math.ldexp(1.0, -1074)
    if slot == 2:
        return 700.0
    if slot == 3:
        return -709.0
    if slot == 4:
        u = rng.next_u53() / (1 << 53)
        return max(u, LOG_EPS)
    if slot == 5:
        u = rng.next_u53() / (1 << 53)
        return max(1.0 - u, LOG_EPS)
    if slot == 6:
        return float(rng.next_u64() % 708)
    if slot == 7:
        z = rng.next_u53() / (1 << 53) * 40.0
        return math.exp(-z)
    if slot == 8:
        z = -rng.next_u53() / (1 << 53) * 40.0
        return math.exp(z)
    if slot == 9:
        return -(rng.next_u53() / (1 << 53))
    u = rng.next_u53() / (1 << 53)
    return u * 80.0 - 40.0


def unary_log(x: float) -> float:
    return math.log(x) if x > 0.0 else float("nan")


def unary_log1p(x: float) -> float:
    return math.log1p(x) if x > -1.0 else float("nan")


def sum_list(_i: int, rng: SplitMix64) -> list[float]:
    n = 2 + (rng.next_u64() % 199)
    out = []
    for j in range(n):
        base = (rng.next_u64() % 2000) / 100.0 - 10.0
        out.append(base * ((j % 3) + 1))
    return out


def count_inputs() -> list[int]:
    rng = SplitMix64(COUNT_SEED)
    return list(range(COUNT_DENSE)) + [rng.next_u64() % COUNT_MAX for _ in range(COUNT_SPARSE)]


def pack_f64(x: float) -> bytes:
    return struct.pack(">d", x)


def unary_block_hashes(xs: list[float], transform) -> list[str]:
    blocks = []
    for start in range(0, len(xs), BLOCK_SIZE):
        chunk = xs[start : start + BLOCK_SIZE]
        data = b"".join(pack_f64(transform(x)) for x in chunk)
        blocks.append(hashlib.sha256(data).hexdigest())
    return blocks


def main() -> int:
    rng = SplitMix64(SEED)
    xs = [unary_input(i, rng) for i in range(UNARY_COUNT)]

    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / "version.txt").write_text(sys.version + "\n")
    (OUT / "seed.txt").write_text(f"{SEED}\n")
    (OUT / "block_size.txt").write_text(f"{BLOCK_SIZE}\n")

    def write_lines(name: str, lines: list[str]) -> None:
        (OUT / name).write_text("\n".join(lines) + "\n")

    write_lines("input_hashes.txt", unary_block_hashes(xs, lambda x: x))
    write_lines("exp_hashes.txt", unary_block_hashes(xs, math.exp))
    write_lines("log_hashes.txt", unary_block_hashes(xs, unary_log))
    write_lines("log1p_hashes.txt", unary_block_hashes(xs, unary_log1p))
    # Build 213. `x ** 2` is the C library's `pow`, which is not `x * x`: the two differ on
    # about one input in a thousand, so the port has to call `pow` as well.
    write_lines("square_hashes.txt", unary_block_hashes(xs, lambda x: x ** 2))
    write_lines("sqrt_hashes.txt", unary_block_hashes(xs, lambda x: math.sqrt(abs(x))))
    # `math.log1p(len(text))`: the argument is an int, as it is in `baseline.py`.
    write_lines("log1p_count_hashes.txt", unary_block_hashes(count_inputs(), math.log1p))

    rng = SplitMix64(SEED)
    for i in range(UNARY_COUNT):
        unary_input(i, rng)
    sum_hashes = []
    for i in range(SUM_LISTS):
        values = sum_list(i, rng)
        total = sum(values)
        sum_hashes.append(hashlib.sha256(pack_f64(total)).hexdigest())
    write_lines("sum_hashes.txt", sum_hashes)
    print(f"wrote {OUT}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
