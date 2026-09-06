//! The canonical MCP document, and what one launch does with it. Harness spec 6.5.
//!
//! One document generates the per-provider file and is none of them. The stdio
//! server body is byte-identical across the three providers, and the file name,
//! the directory and the remote key all differ, so a document that was any one of
//! them could not produce the other two.
//!
//! **It is protected config #5** (15.1). An MCP server definition is a command
//! Hydra spawns, so a run that edits this document runs code of its choosing
//! inside every future run. The gate denies writes to it under every policy and
//! `tests/gate.rs` proves that rather than assuming it.
//!
//! **Each server carries an admission field keyed on a policy name.** The policy
//! stays a capability allowlist and the admission sits here, pointing at a policy
//! by name, because a `judge` run receiving a filesystem server would be the
//! policy set saying one thing and the server set saying another.
//!
//! **Admission is not permission.** A server admitted to `standard` still cannot
//! be called: `mcp_call` is granted by `permissive` alone (7.2), so under the
//! other four policies an admitted server starts, its tools reach the model, and
//! the gate refuses every call. That is ticket 27's decision working as written,
//! and `IMPLEMENTATION.md` records it beside the same shape for skills.
//!
//! **On `claude` the config goes on the command line and the skills go in the
//! worktree.** The asymmetry is 6.5's and is deliberate: `--strict-mcp-config`
//! makes the server set an argument Hydra wrote rather than the state of five
//! directories at spawn time, which is what lets `run_start.mcp` claim the set
//! and be right.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::paths::Layout;
use crate::provider::{Provider, agy, claude, cursor};

/// Protected config #5's path, the one [`crate::policy::denied_write_paths`]
/// denies every policy from writing to.
///
/// `CARGO_MANIFEST_DIR` for the reason `PolicySet::discover` and
/// `preamble::path` use it, and with the same limit: an installed binary would
/// look in its source tree. `IMPLEMENTATION.md` records that gap once for all
/// three.
pub fn path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("config/mcp.md")
}

/// The document as Hydra read it: every server, and the sha the record carries.
#[derive(Debug)]
pub struct Document {
    /// `mcp.doc_sha`, contract 5.3. A sha256 over the whole file.
    pub sha: String,
    /// Every server the document declares, in name order.
    pub servers: Vec<Server>,
}

/// One server, in the shape all three providers can be generated from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Server {
    pub name: String,
    pub transport: Transport,
    /// The policy names this server is admitted to. Anything not named here
    /// never sees it.
    pub admit: Vec<String>,
    /// `stdio` only.
    pub command: Option<String>,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    /// `http` and `sse` only.
    pub url: Option<String>,
    pub headers: BTreeMap<String, String>,
}

impl Server {
    pub fn admits(&self, policy: &str) -> bool {
        self.admit.iter().any(|name| name == policy)
    }
}

/// The three transports the document can express.
///
/// The stdio body is identical on all three providers. The remote ones are where
/// they differ: `claude` takes `type` plus `url`, `cursor-agent` takes `url`
/// alone, `agy` takes `serverUrl`, and `agy` refuses `command` and `serverUrl`
/// together. That difference is why the document is not any provider's own file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transport {
    Stdio,
    Http,
    Sse,
}

impl Transport {
    pub fn as_str(self) -> &'static str {
        match self {
            Transport::Stdio => "stdio",
            Transport::Http => "http",
            Transport::Sse => "sse",
        }
    }
}

impl Document {
    pub fn load() -> Result<Self> {
        let file = path();
        let text = std::fs::read_to_string(&file)
            .with_context(|| format!("reading the canonical MCP document at {}", file.display()))?;
        Self::parse(&text).with_context(|| format!("in {}", file.display()))
    }

    /// Reads the document: the fenced `toml` block is the data and the rest is
    /// prose for whoever edits the file.
    ///
    /// **The sha is over the whole file, prose included.** So
    /// `shasum -a 256 config/mcp.md` reproduces what every run record carries,
    /// and a reader holding a record can check the document in front of them
    /// against it with one command. The cost is that fixing a typo in the prose
    /// moves a value every later record carries, which is the opposite of the
    /// call `policy_sha` made when it left `policies/README.md` out of its hash.
    /// The two differ because there the prose is a separate file and here it is
    /// the same one, and a sha that covers only part of a file is a sha nobody
    /// can recompute by hand.
    pub fn parse(text: &str) -> Result<Self> {
        let sha = format!("{:x}", Sha256::digest(text.as_bytes()));
        let block = fenced_toml(text)?;
        let parsed: Toml =
            toml::from_str(&block).context("the document's toml block does not parse")?;

        let mut servers: Vec<Server> = Vec::new();
        for entry in parsed.server {
            servers.push(entry.check()?);
        }
        servers.sort_by(|a, b| a.name.cmp(&b.name));
        for pair in servers.windows(2) {
            if pair[0].name == pair[1].name {
                bail!(
                    "two servers are called {:?}, and a provider keys on the name",
                    pair[0].name
                );
            }
        }

        Ok(Self { sha, servers })
    }

    /// The servers this policy admits, in name order.
    pub fn admitted(&self, policy: &str) -> Vec<&Server> {
        self.servers
            .iter()
            .filter(|server| server.admits(policy))
            .collect()
    }
}

/// What the launch's free pre-flight probe is told about MCP.
///
/// The probe asks the provider what it resolved, and its answer is only the
/// run's answer when it ran under the run's own MCP shape.
pub enum Probe<'a> {
    /// The run's own `--mcp-config`, which the probe passes with
    /// `--strict-mcp-config` so the servers it resolves are exactly the run's.
    Strict(&'a Path),
    /// A run whose MCP shape the probe cannot reproduce, so it carries no MCP
    /// flags and answers nothing about servers. `judge` is that shape: the run
    /// is `--restricted` and the probe is not.
    Unanswerable,
}

/// Whose tree the run stands in, which is a fact about the launch rather than
/// about the policy.
///
/// 6.5: the redirect follows the tree, not the launch mode. A tree Hydra created
/// gets Cursor's data directory and an MCP config; a tree it did not create,
/// which is a `--shared-tree` run or a run outside any repo, gets neither,
/// because approving a server there would write into `~/.cursor/`. `claude`
/// takes its config on the command line and does not care.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Checkout {
    /// A worktree Hydra created for this run.
    Hydra,
    /// No checkout of Hydra's own: the child runs where Hydra was invoked.
    None,
}

/// What one launch admits, and whether its shape can deliver any of it.
pub struct Plan<'a> {
    /// What the run's policy admits, whether or not the launch can deliver it.
    pub admitted: Vec<&'a Server>,
    /// Why this launch shape carries no MCP config at all, when it carries none.
    /// `None` means every admitted server is written.
    pub undeliverable: Option<&'static str>,
}

/// Compares what the policy admits against what the launch can deliver, 6.1
/// item 6.
///
/// It reads a document, two names and one fact about the tree, and writes
/// nothing, so the whole of the launch check is testable without a provider, a
/// worktree or a run id.
pub fn plan<'a>(
    document: &'a Document,
    provider: Provider,
    policy: &str,
    checkout: Checkout,
) -> Plan<'a> {
    Plan {
        admitted: document.admitted(policy),
        undeliverable: undeliverable(provider, policy, checkout),
    }
}

impl<'a> Plan<'a> {
    /// The servers Hydra writes, which is nothing when the launch cannot carry
    /// them.
    pub fn injected(&self) -> &[&'a Server] {
        match self.undeliverable {
            Some(_) => &self.admitted[..0],
            None => &self.admitted,
        }
    }

    /// Contract 5.3's `mcp.injected`: the names Hydra wrote, in name order.
    pub fn injected_names(&self) -> Vec<String> {
        self.injected()
            .iter()
            .map(|server| server.name.clone())
            .collect()
    }

    /// 6.1 item 6's verdict. `Some` is a run to refuse: the policy admits a
    /// server this launch cannot deliver.
    ///
    /// **A launch that delivers nothing and admits nothing is not a shortfall.**
    /// 6.5 puts it plainly: a run whose policy admits no servers proceeds
    /// untouched, which is what keeps `judge` runs launching on a machine whose
    /// document names servers for other policies.
    pub fn shortfall(&self) -> Option<&'static str> {
        match self.undeliverable {
            Some(reason) if !self.admitted.is_empty() => Some(reason),
            _ => None,
        }
    }

    /// What to tell a human whose launch this refused, for
    /// `run_end.remediation`.
    pub fn remediation(&self) -> String {
        let names: Vec<&str> = self
            .admitted
            .iter()
            .map(|server| server.name.as_str())
            .collect();
        format!(
            "the MCP launch check refused this run. This policy admits {}, and the \
             launch cannot deliver {}: {}. Take the policy out of that server's \
             admit list in {}, or run the task under a policy the server is not \
             admitted to.",
            names.join(", "),
            if names.len() == 1 { "it" } else { "them" },
            self.undeliverable
                .unwrap_or("this launch carries no MCP config"),
            path().display(),
        )
    }
}

/// Why a launch of this shape carries no MCP config at all.
///
/// Three providers, two reasons. `claude` answers on the policy name: 6.7's
/// `judge` argv carries no `--mcp-config`. The other two answer on the tree,
/// because both read the file out of the worktree: 6.5 gives `cursor-agent` no
/// injected config outside a checkout Hydra created, and `agy` has nowhere to
/// write one.
fn undeliverable(provider: Provider, policy: &str, checkout: Checkout) -> Option<&'static str> {
    match provider {
        Provider::Claude => claude::mcp_undeliverable(policy),
        Provider::CursorAgent => cursor::mcp_undeliverable(checkout),
        Provider::Agy => agy::mcp_undeliverable(checkout),
    }
}

/// Writes the run's MCP config where the launch will point at it.
///
/// `~/hydra/spawn/<run-id>/mcp.json` on `claude`, from 6.5's table,
/// `<worktree>/.cursor/mcp.json` on `cursor-agent` and
/// `<worktree>/.agents/mcp_config.json` on `agy`, which is why this takes the
/// worktree and returns a path rather than assuming the spawn directory is where
/// every provider's file goes.
///
/// **The file is written even when no server is admitted.** On `claude` an
/// empty `mcpServers` object is what makes `--strict-mcp-config` mean
/// something. On `cursor-agent` it is the same claim made the only way that
/// provider can make it: a project file that declares nothing, so the run's
/// server set is what the pre-flight lists and not what a file the human's
/// repo carried would have added.
///
/// **A file the checkout already holds is refused rather than overwritten.**
/// The worktree is seeded from the human's repo (6.3), and a `.cursor/mcp.json`
/// in it is theirs: writing over it edits their work and `hydra reap` would
/// commit the edit. [`crate::run`] checks the parent repo for the same file
/// before a run id exists, and [`cursor::claim`] is the second line.
pub fn write_config(
    layout: &Layout,
    run_id: &str,
    provider: Provider,
    servers: &[&Server],
    worktree: Option<&Path>,
) -> Result<PathBuf> {
    let (file, config) = match provider {
        Provider::Claude => {
            let file = layout.mcp_file(run_id);
            if let Some(parent) = file.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("creating {}", parent.display()))?;
            }
            (file, claude::mcp_config(servers))
        }
        Provider::CursorAgent => (
            cursor::claim(worktree, "MCP config", cursor::mcp_file)?,
            cursor::mcp_config(servers),
        ),
        Provider::Agy => (
            agy::claim(worktree, "MCP config", agy::mcp_file)?,
            agy::mcp_config(servers),
        ),
    };
    std::fs::write(&file, serde_json::to_string_pretty(&config)?)
        .with_context(|| format!("writing {}", file.display()))?;
    Ok(file)
}

/// Contract 5.3's `mcp` group.
///
/// **`resolved` is null when the provider could not be asked**, never an empty
/// array. An empty array claims the run got no servers, which is a different
/// fact from nobody having asked, and 5.3 already requires a reader to tell them
/// apart. `skills.resolved` set the shape and this follows it.
pub fn record(
    doc_sha: &str,
    injected: &[String],
    resolved: Option<&[String]>,
) -> serde_json::Value {
    serde_json::json!({
        "doc_sha": doc_sha,
        "injected": injected,
        "resolved": resolved,
    })
}

/// The one fenced `toml` block in the document.
///
/// **Exactly one, and a document with none is an error rather than a document
/// with no servers.** A human who breaks the fence would otherwise get a launch
/// that silently admits nothing, which is the failure this file exists to
/// prevent one level down: a run that comes up short quietly.
fn fenced_toml(text: &str) -> Result<String> {
    let mut blocks: Vec<String> = Vec::new();
    let mut body: Vec<&str> = Vec::new();
    let mut inside = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if inside {
            if trimmed == "```" {
                blocks.push(body.join("\n"));
                body.clear();
                inside = false;
            } else {
                body.push(line);
            }
        } else if trimmed == "```toml" {
            inside = true;
        }
    }
    if inside {
        bail!("a ```toml fence opens and never closes");
    }
    match blocks.len() {
        1 => Ok(blocks.remove(0)),
        0 => bail!("the document carries no ```toml block, so nothing declares a server"),
        n => bail!("the document carries {n} ```toml blocks and the format allows one"),
    }
}

#[derive(Deserialize)]
struct Toml {
    #[serde(default)]
    server: Vec<Entry>,
}

/// One `[[server]]` table, before it has been checked.
///
/// **Unknown fields are refused.** `admits = [...]` for `admit = [...]` would
/// otherwise parse, admit the server to nothing, and launch every run without
/// it, which is the silent shortfall 6.1's check exists to catch.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    name: String,
    transport: String,
    admit: Vec<String>,
    command: Option<String>,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    env: BTreeMap<String, String>,
    url: Option<String>,
    #[serde(default)]
    headers: BTreeMap<String, String>,
}

impl Entry {
    fn check(self) -> Result<Server> {
        let name = self.name;
        if name.is_empty() {
            bail!("a server has an empty name");
        }
        if !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            bail!(
                "the server name {name:?} carries something other than a letter, a \
                 digit, a hyphen or an underscore, and it reaches a provider as a key"
            );
        }
        // `claude` names an MCP tool `mcp__<server>__<tool>`, which is the only
        // free source `mcp.resolved` has. A server name carrying `__` makes that
        // name ambiguous and the record's answer a guess, so the document refuses
        // one.
        if name.contains("__") {
            bail!(
                "the server name {name:?} carries `__`, and `claude` joins a server \
                 and a tool with it, so the record could not tell which half is the \
                 server"
            );
        }
        for policy in &self.admit {
            if !crate::policy::NAMES.contains(&policy.as_str()) {
                bail!(
                    "server {name} is admitted to {policy:?}, which is not one of the \
                     five policies: {}",
                    crate::policy::NAMES.join(", ")
                );
            }
        }

        let transport = match self.transport.as_str() {
            "stdio" => Transport::Stdio,
            "http" => Transport::Http,
            "sse" => Transport::Sse,
            other => bail!(
                "server {name} names transport {other:?}, which is not one of stdio, http, sse"
            ),
        };

        match transport {
            Transport::Stdio => {
                if self.command.is_none() {
                    bail!("server {name} is stdio and names no command");
                }
                if self.url.is_some() || !self.headers.is_empty() {
                    bail!(
                        "server {name} is stdio and carries a url or headers; `agy` \
                         refuses a server that names both a command and a url, so the \
                         document refuses one too"
                    );
                }
            }
            Transport::Http | Transport::Sse => {
                if self.url.is_none() {
                    bail!("server {name} is {} and names no url", transport.as_str());
                }
                if self.command.is_some() || !self.args.is_empty() || !self.env.is_empty() {
                    bail!(
                        "server {name} is remote and carries a command, args or env; \
                         `agy` refuses a server that names both a command and a url"
                    );
                }
            }
        }

        Ok(Server {
            name,
            transport,
            admit: self.admit,
            command: self.command,
            args: self.args,
            env: self.env,
            url: self.url,
            headers: self.headers,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TWO: &str = r#"
# Prose above the block, which the parser ignores and the sha covers.

```toml
[[server]]
name = "filesystem"
transport = "stdio"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]
admit = ["permissive"]

[[server]]
name = "docs"
transport = "http"
url = "https://example.com/mcp"
admit = ["standard", "permissive"]
```

Prose below it too.
"#;

    #[test]
    fn a_server_reaches_only_the_policies_it_names() {
        let doc = Document::parse(TWO).unwrap();
        assert_eq!(doc.servers.len(), 2);
        let names = |policy: &str| -> Vec<String> {
            doc.admitted(policy)
                .iter()
                .map(|s| s.name.clone())
                .collect()
        };
        assert_eq!(names("permissive"), vec!["docs", "filesystem"]);
        assert_eq!(names("standard"), vec!["docs"]);
        assert!(names("read-only").is_empty());
        assert!(names("judge").is_empty());
    }

    #[test]
    fn the_sha_is_the_sha_of_the_whole_file() {
        // The value a human reproduces with `shasum -a 256 config/mcp.md`, which
        // is the point of hashing the file rather than the block.
        let doc = Document::parse("```toml\n```\n").unwrap();
        assert_eq!(
            doc.sha,
            format!("{:x}", Sha256::digest(b"```toml\n```\n")),
            "the sha must be of the bytes on disk"
        );
        // Prose is inside the hash, so an edit to it moves the value.
        let edited = Document::parse("a word\n```toml\n```\n").unwrap();
        assert_ne!(doc.sha, edited.sha);
    }

    #[test]
    fn an_empty_block_is_a_document_with_no_servers() {
        // What this machine ships: the format and no server in it.
        let doc = Document::parse("```toml\n# nothing yet\n```\n").unwrap();
        assert!(doc.servers.is_empty());
        assert!(doc.admitted("permissive").is_empty());
    }

    #[test]
    fn a_missing_block_is_an_error_rather_than_an_empty_document() {
        let err = Document::parse("# just prose\n").unwrap_err().to_string();
        assert!(err.contains("no ```toml block"), "{err}");
        let err = Document::parse("```toml\n[[server]]\n")
            .unwrap_err()
            .to_string();
        assert!(err.contains("never closes"), "{err}");
        let err = Document::parse("```toml\n```\n```toml\n```\n")
            .unwrap_err()
            .to_string();
        assert!(err.contains("2 ```toml blocks"), "{err}");
    }

    #[test]
    fn a_typo_in_a_field_name_is_refused_rather_than_admitting_nothing() {
        let text = "```toml\n[[server]]\nname = \"x\"\ntransport = \"stdio\"\n\
                    command = \"true\"\nadmits = [\"standard\"]\n```\n";
        assert!(Document::parse(text).is_err());
    }

    #[test]
    fn a_policy_name_that_is_not_one_of_the_five_is_refused() {
        let text = "```toml\n[[server]]\nname = \"x\"\ntransport = \"stdio\"\n\
                    command = \"true\"\nadmit = [\"standrad\"]\n```\n";
        let err = Document::parse(text).unwrap_err().to_string();
        assert!(err.contains("not one of the five policies"), "{err}");
    }

    #[test]
    fn a_server_name_with_a_double_underscore_is_refused() {
        // `mcp__a__b__ping` cannot be split back into a server and a tool, and
        // that split is the only free source `mcp.resolved` has on `claude`.
        let text = "```toml\n[[server]]\nname = \"a__b\"\ntransport = \"stdio\"\n\
                    command = \"true\"\nadmit = [\"standard\"]\n```\n";
        let err = Document::parse(text).unwrap_err().to_string();
        assert!(err.contains("`__`"), "{err}");
    }

    #[test]
    fn a_server_cannot_be_both_a_command_and_a_url() {
        let text = "```toml\n[[server]]\nname = \"x\"\ntransport = \"stdio\"\n\
                    command = \"true\"\nurl = \"https://example.com\"\n\
                    admit = [\"standard\"]\n```\n";
        let err = Document::parse(text).unwrap_err().to_string();
        assert!(err.contains("agy"), "{err}");
    }

    #[test]
    fn judge_admits_a_server_and_the_launch_cannot_deliver_it() {
        // 6.7 gives a `judge` run `--restricted` and no `--mcp-config`, so a
        // document admitting a server to `judge` is one this launch cannot
        // satisfy. 6.1 item 6 fails the run rather than launching short.
        let text = "```toml\n[[server]]\nname = \"fs\"\ntransport = \"stdio\"\n\
                    command = \"true\"\nadmit = [\"judge\", \"standard\"]\n```\n";
        let doc = Document::parse(text).unwrap();

        let judge = plan(&doc, Provider::Claude, "judge", Checkout::Hydra);
        assert!(judge.shortfall().is_some());
        assert!(
            judge.injected().is_empty(),
            "nothing is written for a run about to be refused"
        );
        assert!(judge.remediation().contains("fs"));

        let standard = plan(&doc, Provider::Claude, "standard", Checkout::Hydra);
        assert!(standard.shortfall().is_none());
        assert_eq!(standard.injected_names(), vec!["fs".to_string()]);
    }

    #[test]
    fn a_cursor_run_outside_a_tree_of_hydras_cannot_be_delivered_a_server() {
        // 6.5: the redirect follows the tree. A `judge` run on this provider
        // is fine, because nothing about `--force` drops MCP the way
        // `--restricted` was meant to; a run with no checkout is not.
        let text = "```toml\n[[server]]\nname = \"fs\"\ntransport = \"stdio\"\n\
                    command = \"true\"\nadmit = [\"judge\", \"standard\"]\n```\n";
        let doc = Document::parse(text).unwrap();

        let owned = plan(&doc, Provider::CursorAgent, "standard", Checkout::Hydra);
        assert!(owned.shortfall().is_none());
        assert_eq!(owned.injected_names(), vec!["fs".to_string()]);

        let judge = plan(&doc, Provider::CursorAgent, "judge", Checkout::Hydra);
        assert!(judge.shortfall().is_none(), "judge is claude's shape, not this one");

        let shared = plan(&doc, Provider::CursorAgent, "standard", Checkout::None);
        assert!(shared.shortfall().is_some());
        assert!(shared.injected().is_empty());
        assert!(shared.remediation().contains("~/.cursor/"));

        let admits_nothing = plan(&doc, Provider::CursorAgent, "read-only", Checkout::None);
        assert!(admits_nothing.shortfall().is_none());
    }

    #[test]
    fn a_judge_run_that_admits_nothing_launches() {
        // 6.5: a run whose policy admits no servers proceeds untouched. Without
        // this, every `judge` run on a machine whose document names one server
        // for `standard` would be refused.
        let text = "```toml\n[[server]]\nname = \"fs\"\ntransport = \"stdio\"\n\
                    command = \"true\"\nadmit = [\"standard\"]\n```\n";
        let doc = Document::parse(text).unwrap();
        let judge = plan(&doc, Provider::Claude, "judge", Checkout::Hydra);
        assert!(judge.shortfall().is_none());
        assert!(judge.injected_names().is_empty());
    }

    #[test]
    fn the_record_says_nothing_asked_rather_than_nothing_resolved() {
        let asked = record("abc", &["fs".to_string()], Some(&["fs".to_string()]));
        assert_eq!(asked["doc_sha"], "abc");
        assert_eq!(asked["injected"][0], "fs");
        assert_eq!(asked["resolved"][0], "fs");

        let unasked = record("abc", &["fs".to_string()], None);
        assert!(unasked["resolved"].is_null());

        let nothing_resolved = record("abc", &["fs".to_string()], Some(&[]));
        assert!(nothing_resolved["resolved"].is_array());
        assert_eq!(nothing_resolved["resolved"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn the_shipped_document_parses_and_ships_no_server() {
        // Protected config #5 on this machine. A human adding a real server is a
        // human's decision, so what ships is the format and an example in
        // comments.
        let doc = Document::load().unwrap();
        assert!(
            doc.servers.is_empty(),
            "config/mcp.md now declares a server, which is a human's decision; \
             update this test with what was added and why"
        );
        assert_eq!(doc.sha.len(), 64);
    }
}
