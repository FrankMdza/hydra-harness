import json
import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
REPO = ROOT.parents[1]
HY_ZSH = ROOT / "hy.zsh"
VENDORS = REPO / "integrations" / "puppets" / "vendors.json"
PROVIDERS = ROOT / "interactive-gate-providers.txt"


def write_stub_herdr(bin_dir: Path, log_file: Path) -> None:
    script = f"""#!/bin/sh
log={log_file}
case "$1" in
pane)
  case "$2" in
  split)
    id="pane-$RANDOM"
    echo '{{"result":{{"pane":{{"pane_id":"'"$id"'"}}}}}}'
    ;;
  run)
    printf '%s\\n' "$*" >> "$log"
    ;;
  esac
  ;;
esac
"""
    path = bin_dir / "herdr"
    path.write_text(script)
    path.chmod(0o755)


def run_hy(env: dict, *args: str, stdin: str = "") -> subprocess.CompletedProcess:
    cmd = [
        "zsh",
        "-c",
        f"source {HY_ZSH}; hy {' '.join(args)}",
    ]
    return subprocess.run(
        cmd,
        input=stdin,
        text=True,
        capture_output=True,
        env=env,
        cwd=ROOT,
    )


def bottom_pane_tui_args(log_file: Path) -> str:
    lines = log_file.read_text().strip().splitlines()
    for line in reversed(lines):
        if "pane run" in line and "hydra-tui-in-pane" in line:
            cmd = line.split("pane run ", 1)[1].split(" ", 1)[1]
            marker = "hydra-tui-in-pane "
            idx = cmd.index(marker)
            return cmd[idx + len(marker) :]
    raise AssertionError(f"no hydra-tui-in-pane run in log:\n{log_file.read_text()}")


class TestHy(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.bin_dir = Path(self.tmp) / "bin"
        self.bin_dir.mkdir()
        self.log = Path(self.tmp) / "herdr.log"
        write_stub_herdr(self.bin_dir, self.log)

        self.env = os.environ.copy()
        self.env["HERDR_ENV"] = "1"
        self.env["HERDR_PANE_ID"] = "w0:p0"
        self.env["HYDRA_HERDR"] = str(ROOT)
        self.env["HYDRA_BIN"] = str(self.bin_dir / "hydra-stub")
        self.env["PATH"] = f"{self.bin_dir}:{self.env['PATH']}"
        (self.bin_dir / "hydra-stub").write_text("#!/bin/sh\n")
        (self.bin_dir / "hydra-stub").chmod(0o755)

    def tearDown(self):
        shutil.rmtree(self.tmp)

    def test_hy_sonnet_standard_matches_direct_launch(self):
        proc = run_hy(self.env, "sonnet", "standard")
        self.assertEqual(proc.returncode, 0, proc.stderr)
        args = bottom_pane_tui_args(self.log)
        self.assertEqual(args, "--provider claude --model sonnet --policy standard")

    def test_hy_prompt_picks_claude_and_sonnet(self):
        proc = run_hy(self.env, stdin="1\n\n")
        self.assertEqual(proc.returncode, 0, proc.stderr + proc.stdout)
        args = bottom_pane_tui_args(self.log)
        self.assertEqual(args, "--provider claude --model sonnet --policy standard")

    def test_hy_refuses_cursor_agent(self):
        proc = run_hy(self.env, "cursor-agent")
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("cursor-agent", proc.stderr)
        self.assertIn("130", proc.stderr)
        self.assertFalse(self.log.exists())


if __name__ == "__main__":
    unittest.main()
