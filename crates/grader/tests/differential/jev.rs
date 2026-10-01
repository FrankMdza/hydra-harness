#[test]
fn jev_commands_match_python() {
    super::queue::replay("jev.json", "jev");
}
