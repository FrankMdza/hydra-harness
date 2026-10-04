# m2-saturating-bump

Size: medium

`tally::bump` uses wrapping add on a `u8`. `bump(255)` is therefore 0. Change it to saturating
add so `bump(255)` stays 255, and add tests for 0, 254 and 255. The tests belong in
`src/lib.rs` beside the others. Done means `cargo test` is green and no call of `bump` wraps.
