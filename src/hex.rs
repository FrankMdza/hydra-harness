//! Lowercase hex of a digest, for every sha this crate writes.
//!
//! `crates/records/src/hex.rs` holds the same function for the grader and the other workspace
//! crates. This crate depends on none of them, so it carries its own copy and the same fixed
//! vectors.

/// Lowercase hex, two characters per byte, of `bytes`.
///
/// Build ticket 337: `sha2` 0.11 dropped `LowerHex` for its digest array, so a sha written as
/// `format!("{:x}", digest)` no longer compiles. `gate_sha`, the sandbox profile sha and the hook
/// config sha on every `run_start` go through here, so a record written before that bump still
/// reads back as the gate compiled today rather than a foreign one.
pub fn hex_lower(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::hex_lower;
    use sha2::{Digest, Sha256};

    /// Both literals came from `shasum -a 256` on 2026-10-15 with `sha2` 0.10 in the lock file,
    /// never from the function under test.
    #[test]
    fn the_sha256_of_two_fixed_inputs_is_the_same_string_after_any_sha2_bump() {
        assert_eq!(
            hex_lower(&Sha256::digest(b"hydra")),
            "517ffce87ad701f071040b32ddaa7f4b7b0bb6774b02ff45bf2eef3f2fc1a549"
        );
        assert_eq!(
            hex_lower(&Sha256::digest(b"")),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(hex_lower(&[0x00, 0x0f, 0xa0, 0xff]), "000fa0ff");
    }
}
