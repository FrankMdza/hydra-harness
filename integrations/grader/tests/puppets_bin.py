"""The built `hydra-puppets` the grader's tests shell out to.

`labels.py` reads a goal through `hydra-puppets show` (map 93), so its tests need the binary.
They use the one cargo built in this checkout, the newer of the release and the debug build,
and fail when there is none. They never skip. CI builds the release before this suite runs;
by hand, `cargo build -p hydra-puppets` is enough.
"""

import os
from pathlib import Path

CHECKOUT = Path(__file__).resolve().parents[3]


def built() -> Path:
    target = Path(os.environ.get("CARGO_TARGET_DIR") or CHECKOUT / "target")
    found = [
        path
        for path in (target / "release" / "hydra-puppets", target / "debug" / "hydra-puppets")
        if path.is_file()
    ]
    if not found:
        raise AssertionError(
            f"no hydra-puppets under {target}; run `cargo build -p hydra-puppets` first"
        )
    return max(found, key=lambda path: path.stat().st_mtime).resolve()
