//! The launchd template the human installs; nothing here installs it.
//!
//! This is `integrations/grader/tests/test_launchd.py`, which build ticket 220 deleted with
//! the rest of the grader's Python, and three checks it did not have: the program is
//! `hydra-grader`, the job names the data directory, and `HYDRA_BIN` is absolute.
//!
//! The Python read the file with `plistlib`. This crate takes no XML dependency for one
//! file, so the reader below knows the five kinds of value the template holds and nothing
//! else. Where `/usr/bin/plutil` exists, which is every Mac, the file is linted by it too.

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

fn grader() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../integrations/grader")
}

fn template() -> PathBuf {
    grader().join("launchd/co.hephaestuslab.hydra-grader.plist")
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
         <key>n</key><integer>900</integer><key>t</key><true/><key>f</key><false/></dict>\n\
         </plist>\n",
    );
    assert_eq!(strings(read.get("a")), ["x & y", ""]);
    assert_eq!(read.get("n"), &Plist::Whole(900));
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
fn runs_the_grader_once_every_fifteen_minutes() {
    let job = job();
    assert_eq!(job.get("Label").text(), "co.hephaestuslab.hydra-grader");
    assert_eq!(job.get("StartInterval"), &Plist::Whole(900));
    let args = strings(job.get("ProgramArguments"));
    // The binary, by an absolute path, and no interpreter in front of it.
    assert!(
        args[0].ends_with("/hydra/target/release/hydra-grader"),
        "{args:?}"
    );
    assert!(args[0].starts_with('/'), "{args:?}");
    assert_eq!(args[1..], ["once"]);
    assert!(
        !args
            .iter()
            .any(|arg| arg.contains("python") || arg.ends_with(".py")),
        "{args:?}"
    );
}

#[test]
fn sets_both_homes_and_logs_under_the_grader_home() {
    let job = job();
    let env = job.get("EnvironmentVariables");
    assert!(env.get("HYDRA_ROOT").text().starts_with('/'));
    let home = format!("{}/", env.get("HYDRA_GRADER_HOME").text());
    assert!(job.get("StandardOutPath").text().starts_with(&home));
    assert!(job.get("StandardErrorPath").text().starts_with(&home));
}

#[test]
fn names_the_hydra_binary_launchd_cannot_find_on_its_own() {
    // launchd's PATH has no ~/.local/bin, and the first installed run on 2026-09-28 died
    // with FileNotFoundError: 'hydra'. The grader reads HYDRA_BIN before it searches the
    // PATH, and runs the `hydra-puppets` beside it only when that path is absolute.
    let job = job();
    let env = job.get("EnvironmentVariables");
    let hydra = env.get("HYDRA_BIN").text();
    assert!(hydra.ends_with("/hydra/target/release/hydra"), "{hydra}");
    assert!(hydra.starts_with('/'), "{hydra}");
    assert!(env.get("PATH").text().contains("/.local/bin"));
    // The helper beside it is in the directory the program itself is in.
    let program = strings(job.get("ProgramArguments"))[0];
    assert_eq!(Path::new(program).parent(), Path::new(hydra).parent());
}

#[test]
fn names_the_directory_the_alias_table_is_read_from() {
    // The Python read `aliases.json` beside its own checkout. A binary has no checkout, so
    // the job says where the data is rather than leaving it to where the binary sits.
    let job = job();
    let data = job
        .get("EnvironmentVariables")
        .get("HYDRA_PUPPETS_DATA")
        .text();
    assert!(data.starts_with('/'), "{data}");
    assert!(data.ends_with("/hydra/integrations/puppets"), "{data}");
    // The template's checkout is this one's shape: the file is there.
    assert!(grader().join("../puppets/aliases.json").is_file());
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
fn the_readme_says_how_to_install_replace_and_remove_it() {
    let readme = std::fs::read_to_string(grader().join("README.md")).unwrap();
    for said in [
        "launchctl bootstrap",
        "launchctl bootout",
        "cargo build --release --workspace",
        "s|/Users/USER|$HOME|g",
    ] {
        assert!(readme.contains(said), "the README does not say `{said}`");
    }
}
