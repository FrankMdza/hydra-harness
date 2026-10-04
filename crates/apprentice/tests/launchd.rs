//! The launchd template the human installs; nothing here installs it.
//!
//! This is the `LaunchdTests` class of `integrations/apprentice/tests/test_apprentice.py`,
//! which build ticket 234 deleted with the rest of the apprentice's Python, and three checks
//! it did not have: the program is `hydra-apprentice` with no interpreter in front of it,
//! every path is under the placeholder home, and the README says how to install it.
//!
//! The Python read the file with `plistlib`. The reader below is a copy of the one in
//! `crates/grader/tests/launchd.rs`, which knows the five kinds of value the templates hold
//! and nothing else, so this crate takes no XML dependency for one file. Where
//! `/usr/bin/plutil` exists, which is every Mac, the file is linted by it too.

use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, PartialEq)]
enum Plist {
    Dict(Vec<(String, Plist)>),
    Array(Vec<Plist>),
    Text(String),
    Whole(i64),
    Flag(bool),
}

impl Plist {
    fn get(&self, key: &str) -> &Plist {
        let Plist::Dict(pairs) = self else {
            panic!("{key} of {self:?}");
        };
        let found = pairs.iter().find(|(name, _)| name == key);
        &found.unwrap_or_else(|| panic!("no {key}")).1
    }

    fn text(&self) -> &str {
        match self {
            Plist::Text(text) => text,
            other => panic!("not a string: {other:?}"),
        }
    }
}

/// The tags and the text between them, with comments, the declaration and the doctype gone.
fn tokens(source: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut rest = source;
    while let Some(open) = rest.find('<') {
        let before = rest[..open].trim();
        if !before.is_empty() {
            found.push(before.to_string());
        }
        rest = &rest[open..];
        if let Some(comment) = rest.strip_prefix("<!--") {
            let end = comment.find("-->").expect("a comment that ends");
            assert!(
                !comment[..end].contains("--"),
                "a double hyphen inside a comment is not XML"
            );
            rest = &comment[end + 3..];
            continue;
        }
        let close = rest.find('>').expect("a tag that ends");
        let tag = &rest[..=close];
        rest = &rest[close + 1..];
        if !tag.starts_with("<?") && !tag.starts_with("<!") {
            found.push(tag.to_string());
        }
    }
    assert!(rest.trim().is_empty(), "text after the last tag");
    found
}

/// `&amp;` and its four siblings, which is all a path or a label can need.
fn unescaped(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

/// The text of `<name>text</name>`, with `at` on the opening tag.
fn element(tokens: &[String], at: &mut usize, name: &str) -> String {
    let close = format!("</{name}>");
    *at += 1;
    if tokens[*at] == close {
        *at += 1;
        return String::new();
    }
    let text = unescaped(&tokens[*at]);
    assert_eq!(tokens[*at + 1], close);
    *at += 2;
    text
}

fn value(tokens: &[String], at: &mut usize) -> Plist {
    match tokens[*at].as_str() {
        "<dict>" => {
            *at += 1;
            let mut pairs = Vec::new();
            while tokens[*at] != "</dict>" {
                assert_eq!(tokens[*at], "<key>");
                let key = element(tokens, at, "key");
                pairs.push((key, value(tokens, at)));
            }
            *at += 1;
            Plist::Dict(pairs)
        }
        "<array>" => {
            *at += 1;
            let mut items = Vec::new();
            while tokens[*at] != "</array>" {
                items.push(value(tokens, at));
            }
            *at += 1;
            Plist::Array(items)
        }
        "<string>" => Plist::Text(element(tokens, at, "string")),
        "<integer>" => Plist::Whole(element(tokens, at, "integer").parse().unwrap()),
        "<true/>" | "<false/>" => {
            *at += 1;
            Plist::Flag(tokens[*at - 1] == "<true/>")
        }
        other => panic!("a value this reader does not know: {other}"),
    }
}

fn parse(source: &str) -> Plist {
    let tokens = tokens(source);
    assert_eq!(
        tokens.first().map(String::as_str),
        Some("<plist version=\"1.0\">")
    );
    assert_eq!(tokens.last().map(String::as_str), Some("</plist>"));
    let mut at = 1;
    let root = value(&tokens, &mut at);
    assert_eq!(at, tokens.len() - 1, "one value under <plist>");
    root
}

fn apprentice() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../integrations/apprentice")
}

fn template() -> PathBuf {
    apprentice().join("launchd/co.hephaestuslab.hydra-apprentice.plist")
}

fn job() -> Plist {
    parse(&std::fs::read_to_string(template()).unwrap())
}

fn strings(list: &Plist) -> Vec<&str> {
    match list {
        Plist::Array(items) => items.iter().map(Plist::text).collect(),
        other => panic!("not an array: {other:?}"),
    }
}

#[test]
fn the_reader_reads_the_five_kinds_of_value() {
    let read = parse(
        "<?xml version=\"1.0\"?>\n<!DOCTYPE plist>\n<!-- a note -->\n<plist version=\"1.0\">\n\
         <dict><key>a</key><array><string>x &amp; y</string><string></string></array>\
         <key>n</key><integer>4</integer><key>t</key><true/><key>f</key><false/></dict>\n\
         </plist>\n",
    );
    assert_eq!(strings(read.get("a")), ["x & y", ""]);
    assert_eq!(read.get("n"), &Plist::Whole(4));
    assert_eq!(read.get("t"), &Plist::Flag(true));
    assert_eq!(read.get("f"), &Plist::Flag(false));
}

#[test]
fn the_template_is_a_property_list_to_the_system_too() {
    let plutil = Path::new("/usr/bin/plutil");
    if !plutil.exists() {
        // Not a Mac. The reader above is the only check there, and it still ran.
        return;
    }
    let ran = Command::new(plutil)
        .arg("-lint")
        .arg(template())
        .output()
        .unwrap();
    assert!(
        ran.status.success(),
        "{}",
        String::from_utf8_lossy(&ran.stdout)
    );
}

#[test]
fn runs_attempt_at_four_for_thirty_minutes() {
    let job = job();
    assert_eq!(job.get("Label").text(), "co.hephaestuslab.hydra-apprentice");
    assert_eq!(
        job.get("StartCalendarInterval"),
        &Plist::Dict(vec![
            ("Hour".to_string(), Plist::Whole(4)),
            ("Minute".to_string(), Plist::Whole(0)),
        ])
    );
    let Plist::Dict(keys) = &job else {
        unreachable!()
    };
    // Loading the job must not start a pass.
    assert!(!keys.iter().any(|(key, _)| key == "RunAtLoad"));
    let args = strings(job.get("ProgramArguments"));
    // The binary, by an absolute path, and no interpreter in front of it.
    assert!(
        args[0].ends_with("/hydra/target/release/hydra-apprentice"),
        "{args:?}"
    );
    assert!(args[0].starts_with('/'), "{args:?}");
    assert_eq!(args[1..], ["attempt", "--minutes", "30"]);
    assert!(
        !args
            .iter()
            .any(|arg| arg.contains("python") || arg.ends_with(".py")),
        "{args:?}"
    );
}

#[test]
fn sets_both_homes_and_logs_under_the_apprentice_home() {
    let job = job();
    let env = job.get("EnvironmentVariables");
    assert!(env.get("HYDRA_ROOT").text().starts_with('/'));
    let home = format!("{}/apprentice/", env.get("HYDRA_GRADER_HOME").text());
    assert!(job.get("StandardOutPath").text().starts_with(&home));
    assert!(job.get("StandardErrorPath").text().starts_with(&home));
}

#[test]
fn every_path_of_the_template_is_under_the_placeholder_home() {
    // The README's install command replaces `/Users/USER` and nothing else.
    let source = std::fs::read_to_string(template()).unwrap();
    let installed = source.replace("/Users/USER", "/Users/someone");
    assert!(
        !installed.contains("USER"),
        "a placeholder the command would miss"
    );
    assert_eq!(parse(&installed).get("Label"), job().get("Label"));
}

#[test]
fn the_readme_says_how_to_build_install_and_remove_it() {
    let readme = std::fs::read_to_string(apprentice().join("README.md")).unwrap();
    for said in [
        "cargo build --release --workspace",
        "launchctl bootstrap",
        "launchctl bootout",
        "s|/Users/USER|$HOME|g",
    ] {
        assert!(readme.contains(said), "the README does not say `{said}`");
    }
}
