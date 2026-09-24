//! The write-time validator, harness spec 6.4.
//!
//! **A YAML parser and not a regex.** An unquoted YAML scalar may not contain a
//! colon followed by a space, and a `SKILL.md` whose description does breaks on
//! two of the four providers without a word: ticket 24 measured `agy` dropping
//! the skill from its listing and Cursor keeping it with an empty description.
//! Both failures are silent, so the store has to refuse the file rather than
//! discover it later.
//!
//! A regex over `: ` gets this wrong in both directions, which is why the spec
//! names a parser. Measured on this machine 2026-09-06: `writing-fragments` and
//! `writing-shape` both carry `: ` inside a **quoted** description and are valid
//! YAML, and a `3:1` with no space is valid too. What breaks is the bare colon in
//! an unquoted scalar, and only a parser knows the difference.
//!
//! **`SKILL.md` carries `name` and `description` and Hydra's own metadata lives in
//! `hydra.toml`** (6.4). This validator holds the store to that, plus two rules the
//! store needs and the file format does not:
//!
//! - The frontmatter `name` must equal the directory name. There is no shared
//!   notion of a skill's identity across the four providers, `claude` keys on the
//!   directory and `agy` on the frontmatter, so a store skill where the two
//!   disagree is a different skill to each of them. Making them equal is what
//!   gives a store entry one name to collide on.
//! - `disable-model-invocation: true` is refused. It hides the skill from the
//!   provider's own listing, which is the answer the launch-time collision check
//!   reads, so such a skill would be linked and then indistinguishable from one a
//!   collision swallowed. Measured 2026-09-06: eight of the eleven skills in
//!   `~/.claude/skills` carry it, and none of the eight appear in `claude`'s
//!   listing.

use std::path::Path;

use anyhow::{Context, Result, bail};

/// The two portable fields, off one `SKILL.md`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skill {
    pub name: String,
    pub description: String,
}

/// Reads and validates `<dir>/SKILL.md`.
///
/// The directory name is part of the check rather than a caller's convenience:
/// see the module note on identity.
pub fn read(dir: &Path) -> Result<Skill> {
    let file = dir.join("SKILL.md");
    let text =
        std::fs::read_to_string(&file).with_context(|| format!("reading {}", file.display()))?;
    let dir_name = dir
        .file_name()
        .and_then(|n| n.to_str())
        .with_context(|| format!("{} has no usable directory name", dir.display()))?;
    parse(&text, dir_name).with_context(|| format!("{} is not a usable skill", file.display()))
}

/// The frontmatter of one `SKILL.md`, checked against the name it is filed under.
pub fn parse(text: &str, dir_name: &str) -> Result<Skill> {
    let block = frontmatter(text)?;

    let docs = yaml_rust2::YamlLoader::load_from_str(block).map_err(|err| {
        anyhow::anyhow!(
            "its frontmatter is not YAML: {err}. An unquoted description may not \
             contain a colon followed by a space; quote it or rewrite it."
        )
    })?;
    let doc = docs
        .first()
        .context("its frontmatter is empty, so it declares no name and no description")?;

    let name = field(doc, "name")?;
    let description = field(doc, "description")?;

    if doc["disable-model-invocation"].as_bool() == Some(true) {
        bail!(
            "it sets disable-model-invocation, which hides it from the provider's \
             own skill listing. That listing is what the launch-time collision \
             check reads, so a store skill may not set it."
        );
    }

    if name != dir_name {
        bail!(
            "it declares the name {name} and sits in a directory called {dir_name}. \
             `claude` keys on the directory and `agy` on the frontmatter, so the two \
             have to agree or the entry is a different skill to each provider."
        );
    }

    Ok(Skill { name, description })
}

/// The text between the opening `---` and the next one.
///
/// Nothing clever: the file has to start with the fence, because a `SKILL.md` that
/// buries its frontmatter further down has none as far as any provider is
/// concerned.
///
/// Crate-visible because [`crate::skills::index`] reads the same block leniently,
/// and a second copy of the fence rules is a second place to forget the `\r\n`
/// arm.
pub(crate) fn frontmatter(text: &str) -> Result<&str> {
    let rest = text
        .strip_prefix("---\n")
        .or_else(|| text.strip_prefix("---\r\n"))
        .context("it does not open with a `---` frontmatter fence")?;
    let end = rest
        .find("\n---")
        .context("its frontmatter fence is never closed")?;
    Ok(&rest[..end])
}

fn field(doc: &yaml_rust2::Yaml, key: &str) -> Result<String> {
    match &doc[key] {
        yaml_rust2::Yaml::String(value) if !value.trim().is_empty() => Ok(value.trim().to_string()),
        yaml_rust2::Yaml::BadValue => bail!("its frontmatter declares no {key}"),
        other => bail!("its {key} is {other:?} rather than a string"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skill(description: &str) -> String {
        format!("---\nname: probe\ndescription: {description}\n---\n\nbody\n")
    }

    #[test]
    fn a_plain_description_parses() {
        let parsed = parse(&skill("A probe skill. Marker ALPHA."), "probe").unwrap();
        assert_eq!(parsed.name, "probe");
        assert_eq!(parsed.description, "A probe skill. Marker ALPHA.");
    }

    #[test]
    fn a_colon_followed_by_a_space_is_refused() {
        // 6.4's rule, and the whole reason this is a parser. Ticket 24 measured
        // `agy` dropping such a skill and Cursor emptying its description, both
        // without a word.
        let err = parse(&skill("A probe skill. label: value here."), "probe").unwrap_err();
        assert!(
            format!("{err:#}").contains("colon followed by a space"),
            "{err:#}"
        );
    }

    #[test]
    fn a_quoted_colon_is_fine_and_a_regex_would_have_refused_it() {
        // `~/.claude/skills/writing-shape` has exactly this shape, and it is valid
        // YAML. A `: ` scan would refuse it, which is why the spec asks for a
        // parser.
        let parsed = parse(
            &skill("\"Writing, exploit: shape raw material into an article.\""),
            "probe",
        )
        .unwrap();
        assert!(parsed.description.contains("exploit: shape"));
    }

    #[test]
    fn a_colon_with_no_space_is_fine_and_a_regex_would_have_had_to_know_that() {
        let parsed = parse(&skill("A probe skill. Ratio 3:1 no space."), "probe").unwrap();
        assert!(parsed.description.contains("3:1"));
    }

    #[test]
    fn a_missing_description_is_refused() {
        let err = parse("---\nname: probe\n---\n\nbody\n", "probe").unwrap_err();
        assert!(format!("{err:#}").contains("no description"), "{err:#}");
    }

    #[test]
    fn a_name_that_disagrees_with_the_directory_is_refused() {
        // Ticket 24's identity probe: one directory named `t24b-dirname-only`
        // declaring `name: t24b-frontmatter-name` is a different skill to
        // `claude` than to `agy`.
        let err = parse(&skill("A probe."), "somewhere-else").unwrap_err();
        let text = format!("{err:#}");
        assert!(text.contains("probe"), "{text}");
        assert!(text.contains("somewhere-else"), "{text}");
    }

    #[test]
    fn a_skill_hidden_from_the_model_is_refused() {
        let text = "---\nname: probe\ndescription: A probe.\ndisable-model-invocation: true\n---\n";
        let err = parse(text, "probe").unwrap_err();
        assert!(
            format!("{err:#}").contains("disable-model-invocation"),
            "{err:#}"
        );
    }

    #[test]
    fn a_file_with_no_frontmatter_is_refused() {
        let err = parse("# just a heading\n", "probe").unwrap_err();
        assert!(format!("{err:#}").contains("frontmatter fence"), "{err:#}");
    }
}
