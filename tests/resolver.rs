//! Build ticket 39: `hydra show` and `hydra reap` take a run-id prefix.
//!
//! Every case builds record files in a scratch root and launches nothing.

use std::path::{Path, PathBuf};

use hydra::paths::Layout;
use hydra::resolver::resolve;

/// A scratch `~/hydra` holding whatever run files a case names.
struct Corpus {
    root: PathBuf,
}

impl Corpus {
    fn new(name: &str, run_ids: &[&str]) -> Self {
        let root =
            std::env::temp_dir().join(format!("hydra-resolver-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("runs")).expect("scratch runs directory");
        for id in run_ids {
            std::fs::write(root.join("runs").join(format!("{id}.jsonl")), "{}\n")
                .expect("scratch record");
        }
        Self { root }
    }

    fn layout(&self) -> Layout {
        Layout::at(&self.root)
    }
}

impl Drop for Corpus {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

const A: &str = "01a084d3-bdd3-7831-af07-339c253faf33";
const B: &str = "01a08684-67f4-7971-a458-ac34a12d5e28";
const C: &str = "01a086b3-ca7c-7f01-812f-40dc2478f1c4";

#[test]
fn a_prefix_matching_one_record_resolves_to_it() {
    let corpus = Corpus::new("one", &[A, B, C]);
    assert_eq!(resolve(&corpus.layout(), "01a084").unwrap(), A);
    assert_eq!(resolve(&corpus.layout(), "01a08684").unwrap(), B);
}

#[test]
fn one_character_is_a_prefix_when_only_one_record_starts_with_it() {
    let corpus = Corpus::new("short", &[A]);
    assert_eq!(resolve(&corpus.layout(), "0").unwrap(), A);
}

#[test]
fn a_prefix_matching_several_names_every_one_of_them() {
    let corpus = Corpus::new("several", &[A, B, C]);
    let err = resolve(&corpus.layout(), "01a08").unwrap_err().to_string();
    assert!(err.contains("matches 3 runs"), "{err}");
    for id in [A, B, C] {
        assert!(err.contains(id), "{err} omits {id}");
    }
    // Sorted, so the same corpus reports the same list twice running.
    let again = resolve(&corpus.layout(), "01a08").unwrap_err().to_string();
    assert_eq!(err, again);
}

#[test]
fn a_prefix_matching_nothing_is_an_error_rather_than_a_shrug() {
    let corpus = Corpus::new("none", &[A]);
    let err = resolve(&corpus.layout(), "nobody").unwrap_err().to_string();
    assert!(err.contains("no run matches"), "{err}");
}

#[test]
fn a_full_uuid_passes_through_whether_or_not_it_was_ever_recorded() {
    let corpus = Corpus::new("uuid", &[A]);
    assert_eq!(resolve(&corpus.layout(), A).unwrap(), A);
    // The run whose record was pruned. `hydra reap` still has work to do here,
    // which is why a uuid is never looked up.
    assert_eq!(resolve(&corpus.layout(), B).unwrap(), B);
}

#[test]
fn thirty_six_characters_of_anything_is_not_a_uuid() {
    let corpus = Corpus::new("shape", &[A]);
    let ones = "11111111-1111-1111-1111-111111111111";
    assert_eq!(ones.len(), 36);
    assert_eq!(
        resolve(&corpus.layout(), ones).unwrap(),
        ones,
        "hex is a uuid"
    );

    // Pass 01a084d3-bdd3 fed a length check an invented id to make a test pass.
    // These three are 36 characters and none of them is a run.
    for not_a_uuid in [
        "zzzzzzzz-zzzz-zzzz-zzzz-zzzzzzzzzzzz",
        "01a084d3.bdd3.7831.af07.339c253faf33",
        "01a084d3bdd37831af07339c253faf331234",
    ] {
        assert_eq!(not_a_uuid.len(), 36);
        let err = resolve(&corpus.layout(), not_a_uuid)
            .unwrap_err()
            .to_string();
        assert!(err.contains("no run matches"), "{not_a_uuid}: {err}");
    }
}

#[test]
fn an_empty_id_says_so_rather_than_matching_every_run() {
    let corpus = Corpus::new("empty", &[A, B]);
    let err = resolve(&corpus.layout(), "").unwrap_err().to_string();
    assert!(err.contains("a run id is required"), "{err}");
}

#[test]
fn a_corpus_that_does_not_exist_reports_no_match_rather_than_a_read_error() {
    let root = std::env::temp_dir().join(format!("hydra-resolver-absent-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let err = resolve(&Layout::at(Path::new(&root)), "01a08")
        .unwrap_err()
        .to_string();
    assert!(err.contains("no run matches"), "{err}");
}

#[test]
fn ambiguous_same_minute_runs_offer_unique_labels_and_metadata() {
    let ids = [
        "01a0fe73-1b67-7000-8000-000000000001",
        "01a0fe73-5a9c-7000-8000-000000000002",
    ];
    let corpus = Corpus::new("same-minute", &ids);
    for (id, provider, policy) in [(ids[0], "codex", "standard"), (ids[1], "claude", "judge")] {
        let record = serde_json::json!({"run_id": id, "provider": provider, "hydra": {"type": "run_start", "policy": policy}});
        std::fs::write(corpus.layout().run_file(id), format!("{record}\n")).unwrap();
    }
    let err = resolve(&corpus.layout(), "01a0fe73")
        .unwrap_err()
        .to_string();
    assert!(err.contains("01a0fe73-1 codex standard"), "{err}");
    assert!(err.contains("01a0fe73-5 claude judge"), "{err}");
    for id in ids {
        assert_eq!(resolve(&corpus.layout(), &id[..10]).unwrap(), id);
    }
}
