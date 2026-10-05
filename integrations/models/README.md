# hydra-models

A table of every model each harness offers, taken from the CLIs themselves.

`hydra models refresh` runs `agy models`, `cursor-agent models` and `codex debug models`
under the launch environment's allowlist, with a timeout each, and reads Claude's rows
from `integrations/puppets/aliases.json`. It writes `models.json` under the data root
(`~/hydra` unless `HYDRA_ROOT` is set). A provider whose command fails keeps its previous
rows. `hydra models` prints that file and launches nothing.

Nothing under `src/` imports this directory. The job is a LaunchAgent a human installs.

## Schedule

Nothing refreshes on its own. `launchd/co.hephaestuslab.hydra-models.plist` is a template
that runs `target/release/hydra models refresh` at 04:00 and 16:00 local
(`StartCalendarInterval`, two entries) and logs to `$HYDRA_ROOT/models-refresh.log`. Its
paths assume the checkout is `~/hydra`. It names `target/release/hydra` as the program and
sets `HYDRA_HOME` to the checkout so `aliases.json` is found, and `PATH` so the three
listing binaries are found. It has no `RunAtLoad`, so loading it does not start a refresh.

Install it, or replace a copy already installed, with these two commands from the checkout.
The first builds the binary. The second stops the old job if there is one, writes the
template with your home in place of `/Users/USER`, and starts it:

```
cargo build --release
launchctl bootout gui/$(id -u)/co.hephaestuslab.hydra-models; sed "s|/Users/USER|$HOME|g" integrations/models/launchd/co.hephaestuslab.hydra-models.plist > ~/Library/LaunchAgents/co.hephaestuslab.hydra-models.plist && launchctl bootstrap gui/$(id -u) ~/Library/LaunchAgents/co.hephaestuslab.hydra-models.plist
```

`bootout` prints an error when no job is loaded, and the rest still runs.

Remove it:

```
launchctl bootout gui/$(id -u)/co.hephaestuslab.hydra-models && rm ~/Library/LaunchAgents/co.hephaestuslab.hydra-models.plist
```
