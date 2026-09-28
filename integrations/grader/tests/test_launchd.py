from pathlib import Path
import plistlib
import unittest

GRADER = Path(__file__).resolve().parents[1]
PLIST = GRADER / "launchd" / "co.hephaestuslab.hydra-grader.plist"


class LaunchdTemplateTests(unittest.TestCase):
    """The template the human installs; nothing here installs it."""

    def setUp(self):
        with PLIST.open("rb") as handle:
            self.job = plistlib.load(handle)

    def test_runs_grader_once_every_fifteen_minutes(self):
        self.assertEqual(self.job["Label"], "co.hephaestuslab.hydra-grader")
        self.assertEqual(self.job["StartInterval"], 900)
        args = self.job["ProgramArguments"]
        self.assertTrue(args[1].endswith("integrations/grader/grader.py"), args)
        self.assertEqual(args[2:], ["once"])

    def test_sets_both_homes_and_logs_under_the_grader_home(self):
        env = self.job["EnvironmentVariables"]
        self.assertIn("HYDRA_ROOT", env)
        home = env["HYDRA_GRADER_HOME"]
        self.assertTrue(self.job["StandardOutPath"].startswith(home + "/"))
        self.assertTrue(self.job["StandardErrorPath"].startswith(home + "/"))

    def test_names_the_hydra_binary_launchd_cannot_find_on_its_own(self):
        # launchd's PATH has no ~/.local/bin, and the first installed run on 2026-09-28 died with
        # FileNotFoundError: 'hydra'. grader.py reads HYDRA_BIN before it searches the PATH.
        env = self.job["EnvironmentVariables"]
        self.assertTrue(env["HYDRA_BIN"].endswith("/hydra/target/release/hydra"), env)
        self.assertIn("/.local/bin", env["PATH"])

    def test_the_readme_says_how_to_install_and_remove_it(self):
        readme = (GRADER / "README.md").read_text()
        self.assertIn("launchctl bootstrap", readme)
        self.assertIn("launchctl bootout", readme)


if __name__ == "__main__":
    unittest.main()
