//! The launch-time collision check, harness spec 6.1 item 5 and 6.4.
//!
//! **Collisions are checked twice and this is the second one.** At write, the store
//! refuses a name the index already holds, so no collision Hydra causes reaches a
//! run. At launch, a hit fails the run as its own outcome. That looks harsh until
//! you notice what the write check did to the odds: a launch-time collision means
//! the world moved under the store, it should be loud, and the fix is one rename.
//!
//! **The last of the three tests asks the provider rather than comparing strings.**
//! There is no shared notion of a skill's identity: `claude` keys on the directory
//! name, `agy` on the frontmatter `name`, and Cursor's listing has no name field at
//! all. So Hydra links the farm, asks the provider what it resolved, and calls a
//! name it linked that the answer does not carry a collision, whichever side won.
//!
//! The first two tests are still worth their cost, and one measurement says why.
//! Ticket 24 put the same name in `~/.claude/skills` and in the injected root: the
//! provider listed it once, carrying the user-scope marker, and the store's entry
//! was silently absent. The name is in the listing, so the diff sees nothing. Only
//! the index comparison catches that one.

use std::path::PathBuf;

use crate::provider::Provider;
use crate::skills::Injected;
use crate::skills::index::Index;

/// One name two things claim.
#[derive(Debug, Clone)]
pub struct Collision {
    pub name: String,
    /// What clashed, in the words the refusal shows a human.
    pub why: String,
}

/// The three tests, over one run's injection.
///
/// `user` is the index over the roots this provider reads in the human's home;
/// `project` is the one over the checkout's own skills, which are the entries
/// Hydra did not link. Both are compared on the directory name and the frontmatter
/// name, because 6.4 asks for both and the four providers disagree about which
/// one is a skill's name.
///
/// `resolved` is what the provider itself reported, or `None` when it could not be
/// asked. An absent listing costs the third test and leaves the other two, which
/// is the same stance contract 5.3 takes on the field.
pub fn check(
    injected: &Injected,
    provider: Provider,
    home: &std::path::Path,
    user: &Index,
    project: &Index,
    resolved: Option<&[String]>,
) -> Vec<Collision> {
    let roots: Vec<PathBuf> = provider.user_skill_roots(home);
    let mut hits = Vec::new();

    // Names Hydra could not link at all, because the checkout already holds them.
    // Every one is a hit whatever holds it: a store skill that is silently not
    // linked is the failure this check exists to make loud.
    for name in &injected.blocked {
        let why = match project.all().iter().find(|entry| entry.answers_to(name)) {
            Some(entry) => format!(
                "the checkout's own {} already holds this name, at {}",
                provider.skill_root(),
                entry.path.display()
            ),
            None => format!(
                "{} already exists in the checkout and is not a skill Hydra linked, \
                 so the store's copy was not linked",
                injected.root.join(name).display()
            ),
        };
        hits.push(Collision {
            name: name.clone(),
            why,
        });
    }

    // A linked name can still clash with a project skill's frontmatter name, which
    // no link could have collided on: `agy` keys on that name and the filesystem
    // does not.
    for name in &injected.linked {
        for entry in project.all().iter().filter(|e| e.answers_to(name)) {
            hits.push(Collision {
                name: name.clone(),
                why: format!(
                    "the checkout's own {} declares this name at {}",
                    provider.skill_root(),
                    entry.path.display()
                ),
            });
        }
    }

    // User scope, where the provider's own precedence hides the loser.
    for name in &injected.linked {
        for entry in user.holding(name, &roots) {
            hits.push(Collision {
                name: name.clone(),
                why: format!(
                    "{} already holds this name at user scope, which beats the \
                     project scope Hydra injects into, so the store's skill would \
                     be the one silently dropped",
                    entry.path.display()
                ),
            });
        }
    }

    // The provider's own answer, which is the only test that needs no model of
    // how it decides.
    if let Some(resolved) = resolved {
        for name in &injected.linked {
            if !resolved.iter().any(|listed| listed == name) {
                hits.push(Collision {
                    name: name.clone(),
                    why: format!(
                        "{provider} was linked this skill and did not list it, so \
                         something else resolved under the same name"
                    ),
                });
            }
        }
    }

    hits
}

/// What the refused run's record says, contract 6's `remediation`.
pub fn remediation(hits: &[Collision]) -> String {
    let mut lines = vec![format!(
        "{} store skill {} a name that is already taken. Rename the store's copy \
         or remove the other one, then launch again.",
        hits.len(),
        if hits.len() == 1 { "carries" } else { "carry" }
    )];
    for hit in hits {
        lines.push(format!("{}: {}", hit.name, hit.why));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn injected(linked: &[&str], blocked: &[&str]) -> Injected {
        Injected {
            store_sha: "0".repeat(40),
            linked: linked.iter().map(|s| s.to_string()).collect(),
            blocked: blocked.iter().map(|s| s.to_string()).collect(),
            root: PathBuf::from("/tmp/wt/.claude/skills"),
        }
    }

    fn home(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("hydra-collision-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn user_skill(home: &std::path::Path, name: &str) {
        let dir = home.join(".claude/skills").join(name);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("SKILL.md"),
            format!("---\nname: {name}\ndescription: A probe.\n---\n"),
        )
        .unwrap();
    }

    #[test]
    fn a_clean_injection_hits_nothing() {
        let home = home("clean");
        let index = Index::user_scope(&home).unwrap();
        let hits = check(
            &injected(&["alpha", "beta"], &[]),
            Provider::Claude,
            &home,
            &index,
            &Index::default(),
            Some(&["alpha".into(), "beta".into(), "unslop".into()]),
        );
        assert!(hits.is_empty(), "{hits:?}");
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn a_user_scope_name_is_a_collision_even_though_the_listing_carries_it() {
        // Ticket 24's measurement: the provider lists the name once, with the
        // user's copy behind it, and says nothing about the one it dropped. The
        // diff cannot see this and the index comparison can.
        let home = home("user-scope");
        user_skill(&home, "alpha");
        let index = Index::user_scope(&home).unwrap();
        let hits = check(
            &injected(&["alpha"], &[]),
            Provider::Claude,
            &home,
            &index,
            &Index::default(),
            Some(&["alpha".into()]),
        );
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!(hits[0].name, "alpha");
        assert!(hits[0].why.contains("user scope"), "{}", hits[0].why);
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn a_name_the_provider_did_not_list_is_a_collision() {
        let home = home("not-listed");
        let index = Index::user_scope(&home).unwrap();
        let hits = check(
            &injected(&["alpha", "beta"], &[]),
            Provider::Claude,
            &home,
            &index,
            &Index::default(),
            Some(&["beta".into()]),
        );
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!(hits[0].name, "alpha");
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn a_name_the_repo_carries_is_a_collision_without_asking_anyone() {
        let home = home("repo-own");
        let index = Index::user_scope(&home).unwrap();
        let hits = check(
            &injected(&[], &["repo-own"]),
            Provider::Claude,
            &home,
            &index,
            &Index::default(),
            None,
        );
        // The project index is empty here, so nothing says what holds the name.
        // It is still a hit, because a store skill Hydra could not link is the
        // thing this check must never let past in silence.
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert!(hits[0].why.contains(".claude/skills"), "{}", hits[0].why);
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn a_project_skill_declaring_a_linked_name_is_a_collision_no_link_could_catch() {
        // `claude` keys on the directory and `agy` on the frontmatter, so a
        // checkout skill in a directory called something else can still claim a
        // linked skill's name. The filesystem let the link through; the index is
        // what notices.
        let home = home("frontmatter-clash");
        let project_root = home.join("wt/.claude/skills/other-directory");
        std::fs::create_dir_all(&project_root).unwrap();
        std::fs::write(
            project_root.join("SKILL.md"),
            "---\nname: alpha\ndescription: The checkout's own.\n---\n",
        )
        .unwrap();
        let project = Index::project_scope(project_root.parent().unwrap()).unwrap();

        let hits = check(
            &injected(&["alpha"], &[]),
            Provider::Claude,
            &home,
            &Index::default(),
            &project,
            Some(&["alpha".into()]),
        );
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert!(
            hits[0].why.contains("declares this name"),
            "{}",
            hits[0].why
        );
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn an_absent_listing_costs_the_third_test_and_keeps_the_other_two() {
        let home = home("no-listing");
        user_skill(&home, "alpha");
        let index = Index::user_scope(&home).unwrap();
        let hits = check(
            &injected(&["alpha", "beta"], &[]),
            Provider::Claude,
            &home,
            &index,
            &Index::default(),
            None,
        );
        // `beta` is unanswerable without the provider's listing and is not
        // guessed at. `alpha` still hits on the index.
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!(hits[0].name, "alpha");
        std::fs::remove_dir_all(&home).ok();
    }
}
