//! Approved panel shapes, protected config #6 of harness spec 15.1, build
//! ticket 138. The loader against fixture files in a scratch directory; nothing
//! launches, and the real `config/panels.json` is never read.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use hydra::panels::{self, Panels};
use hydra::policy::PolicySet;
use hydra::provider::Provider;
use serde_json::{Value, json};

fn policies() -> PolicySet {
    PolicySet::load(Path::new(env!("CARGO_MANIFEST_DIR")).join("policies")).unwrap()
}

/// A scratch `panels.json` holding `document`, one directory per call because
/// the suite runs in parallel.
fn fixture(document: &Value) -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "hydra-panels-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("panels.json");
    std::fs::write(&path, serde_json::to_string_pretty(document).unwrap()).unwrap();
    path
}

/// The fixed three-seat panel `config/README.md` shows.
fn fixed_panel() -> Value {
    json!({
        "shapes": [{
            "name": "review",
            "askers": ["standard"],
            "cap_per_hour": 2,
            "seats": [
                { "provider": "claude", "model": "opus", "policy": "judge", "role": "reviewer" },
                { "provider": "codex", "policy": "standard", "role": "adversary" },
                { "provider": "cursor-agent", "model": "composer-2.5", "policy": "judge", "role": "security" }
            ]
        }]
    })
}

fn refusal(document: &Value) -> String {
    let err = panels::load(&fixture(document), &policies())
        .expect_err("the shape loaded and should have been refused");
    format!("{err:#}")
}

#[test]
fn one_shape_parses_into_typed_seats() {
    let loaded = panels::load(&fixture(&fixed_panel()), &policies()).unwrap();
    assert_eq!(loaded.shapes.len(), 1);
    let shape = &loaded.shapes[0];
    assert_eq!(shape.name, "review");
    assert_eq!(shape.cap_per_hour, 2);
    assert_eq!(shape.askers, ["standard"]);
    let seats: Vec<_> = shape
        .seats
        .iter()
        .map(|s| {
            (
                s.provider,
                s.model.as_deref(),
                s.policy.as_str(),
                s.role.as_str(),
            )
        })
        .collect();
    assert_eq!(
        seats,
        [
            (Provider::Claude, Some("opus"), "judge", "reviewer"),
            (Provider::Codex, None, "standard", "adversary"),
            (
                Provider::CursorAgent,
                Some("composer-2.5"),
                "judge",
                "security"
            ),
        ]
    );
}

#[test]
fn a_missing_file_loads_as_no_shapes() {
    let dir = fixture(&json!({}));
    let missing = dir.with_file_name("absent.json");
    assert_eq!(
        panels::load(&missing, &policies()).unwrap(),
        Panels::default()
    );
}

#[test]
fn an_unknown_provider_is_refused_with_the_field_named() {
    let mut document = fixed_panel();
    document["shapes"][0]["seats"][1]["provider"] = json!("hermes");
    let err = refusal(&document);
    assert!(err.contains("provider") && err.contains("hermes"), "{err}");
}

#[test]
fn an_unknown_policy_is_refused_with_the_field_named() {
    let mut document = fixed_panel();
    document["shapes"][0]["seats"][0]["policy"] = json!("root");
    let err = refusal(&document);
    assert!(err.contains("policy") && err.contains("root"), "{err}");

    let mut document = fixed_panel();
    document["shapes"][0]["askers"] = json!(["standard", "admin"]);
    let err = refusal(&document);
    assert!(err.contains("askers") && err.contains("admin"), "{err}");
}

#[test]
fn a_seat_wider_than_an_allowed_asker_is_refused() {
    // Map 82, answer 3: a seat never gets more than the session that asked for
    // it. `standard` holds `judge`, so the fixed panel loads for a `standard`
    // asker; `read-only` does not hold `standard`, so the codex seat is wider
    // than that asker and the shape is refused.
    let mut document = fixed_panel();
    document["shapes"][0]["askers"] = json!(["standard", "read-only"]);
    let err = refusal(&document);
    assert!(
        err.contains("policy") && err.contains("standard") && err.contains("read-only"),
        "{err}"
    );

    // `curate` and `standard` are incomparable (7.3), so neither may seat the
    // other.
    let mut document = fixed_panel();
    document["shapes"][0]["askers"] = json!(["curate"]);
    let err = refusal(&document);
    assert!(err.contains("curate"), "{err}");

    // `permissive` holds all five, so it may ask for any seat.
    let mut document = fixed_panel();
    document["shapes"][0]["askers"] = json!(["permissive"]);
    panels::load(&fixture(&document), &policies()).unwrap();
}

#[test]
fn an_unknown_field_is_refused_with_the_field_named() {
    let mut document = fixed_panel();
    document["shapes"][0]["seats"][0]["sandbox"] = json!("off");
    let err = refusal(&document);
    assert!(err.contains("sandbox"), "{err}");

    let mut document = fixed_panel();
    document["shapes"][0]["prompt_free"] = json!(true);
    let err = refusal(&document);
    assert!(err.contains("prompt_free"), "{err}");

    let mut document = fixed_panel();
    document["comment"] = json!("x");
    let err = refusal(&document);
    assert!(err.contains("comment"), "{err}");
}

#[test]
fn a_shape_with_no_seats_askers_or_cap_is_refused() {
    for (field, value) in [
        ("seats", json!([])),
        ("askers", json!([])),
        ("cap_per_hour", json!(0)),
    ] {
        let mut document = fixed_panel();
        document["shapes"][0][field] = value;
        let err = refusal(&document);
        assert!(err.contains(field), "{field}: {err}");
    }
}

#[test]
fn two_shapes_with_one_name_are_refused() {
    let mut document = fixed_panel();
    let shape = document["shapes"][0].clone();
    document["shapes"].as_array_mut().unwrap().push(shape);
    let err = refusal(&document);
    assert!(err.contains("name") && err.contains("review"), "{err}");
}
