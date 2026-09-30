# 195 - puppets in Rust: the commands that launch

Type: build
Status: open
Blocked by: 194

## What

Third of four tickets for map 90's `puppets` port: the commands that start runs or other
programs: `plan`, `plans`, `launch`, `wait`, `review` (including `--no-open` and the Hunk comment
pusher), and the judge panel `choose` and `review` start.

## Do

Port them with the same arguments, output, exit codes and log lines. Launching goes through the
same `hydra run` argv the Python builds (compare the argv, never launch a real provider in a
test): the tests put a stub `hydra` and a stub `hunk` on `PATH`, record their argv and environment,
and compare what the Python and the Rust hand them, and the resulting logs. The Python stays.

## Done when

- fmt and clippy over the workspace; `cargo test -p hydra-puppets`; every command compared.
