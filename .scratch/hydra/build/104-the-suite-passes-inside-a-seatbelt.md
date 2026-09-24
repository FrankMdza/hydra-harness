# 104 - The suite passes inside a seatbelt, so a rung-two pass can clear admission

Type: build
Status: claimed
Blocked by: 98

## What

Found by build ticket 98. Admission runs the whole suite under a seatbelt, and two tests fail there on
any profile: `procinfo::tests::our_own_start_time_matches_ps`, because the seatbelt refuses to exec
setuid `/bin/ps`, and `tests/delegation.rs::a_run_relaunching_the_binary_without_a_parent_is_refused_and_recorded`,
because the binary refuses `HYDRA_ROOT` inside a seatbelt. So every rung-two pass on macOS stops at
`cargo test`, and has since build ticket 75. Make both skip visibly inside a seatbelt, the way
`tests/corpus/mod.rs::seatbelt_exec_or_skip` does, and look for any other test with the same shape.

## Done when

The whole suite passes under a real admission profile on a copy of this repository, measured the way
build ticket 98's Outcome measured it, and the two tests still run and pass outside a seatbelt.

## Outcome
