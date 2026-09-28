//! Approved panel shapes, protected config #6 of harness spec 15.1.
//!
//! Map ticket 82, answer 2: a gated session may ask its launcher for seats
//! without a prompt only when the request matches a shape a human approved.
//! The shapes live in `config/panels.json`, which the human writes and no run
//! can: [`crate::policy::denied_write_paths`] names it under every policy. This
//! module is the format and the loader. Nothing launches from a shape yet.
//!
//! **A seat never gets more than the session that asked for it** (answer 3).
//! The order is 7.3's containment, [`crate::policy::Policy::contains`], so a
//! shape loads only when every policy it lets ask contains every seat's policy.
//! Checking at load rather than at the request means a shape that could ever
//! widen a session is refused before any session sees it.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::policy::PolicySet;
use crate::provider::Provider;

/// `config/panels.json` beside the checkout the binary was built from.
pub fn path() -> PathBuf {
    crate::paths::hydra_home().join("config/panels.json")
}

/// One seat of a shape, typed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Seat {
    pub provider: Provider,
    /// The provider's own model name. `None` launches the provider's default.
    pub model: Option<String>,
    /// One of the five, by name.
    pub policy: String,
    /// Free text: what the seat is told it is for.
    pub role: String,
}

/// One approved shape.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shape {
    pub name: String,
    pub seats: Vec<Seat>,
    /// How many times an hour a session may ask for this shape.
    pub cap_per_hour: u32,
    /// The session policies allowed to ask for it.
    pub askers: Vec<String>,
}

/// Every approved shape. Empty when the human has written none.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Panels {
    pub shapes: Vec<Shape>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPanels {
    shapes: Vec<RawShape>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawShape {
    name: String,
    seats: Vec<RawSeat>,
    cap_per_hour: u32,
    askers: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSeat {
    provider: String,
    #[serde(default)]
    model: Option<String>,
    policy: String,
    role: String,
}

/// Reads the shapes at `path` against the loaded policy set.
///
/// A missing file is no shapes, not an error: a machine where the human has
/// approved nothing is the default, and every request is then refused. Any
/// other failure refuses the whole file with the field named, because a half
/// loaded list of approvals is a list nobody approved.
pub fn load(path: &Path, set: &PolicySet) -> Result<Panels> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Panels::default());
        }
        Err(err) => return Err(err).with_context(|| format!("reading {}", path.display())),
    };
    let raw: RawPanels =
        serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
    let mut names = BTreeSet::new();
    let mut shapes = Vec::with_capacity(raw.shapes.len());
    for (index, shape) in raw.shapes.into_iter().enumerate() {
        let at = format!("{}: shapes[{index}]", path.display());
        if !names.insert(shape.name.clone()) {
            bail!("{at}.name: {} names a second shape", shape.name);
        }
        shapes.push(shape.check(&at, set)?);
    }
    Ok(Panels { shapes })
}

impl RawShape {
    fn check(self, at: &str, set: &PolicySet) -> Result<Shape> {
        if self.seats.is_empty() {
            bail!("{at}.seats: a shape with no seats summons nothing");
        }
        if self.askers.is_empty() {
            bail!("{at}.askers: a shape no policy may ask for is never used");
        }
        if self.cap_per_hour == 0 {
            bail!("{at}.cap_per_hour: 0 is a shape no session may ask for");
        }
        let mut askers = Vec::with_capacity(self.askers.len());
        for (index, asker) in self.askers.iter().enumerate() {
            let policy = set
                .get(asker)
                .with_context(|| format!("{at}.askers[{index}]"))?;
            askers.push(policy);
        }
        let mut seats = Vec::with_capacity(self.seats.len());
        for (index, seat) in self.seats.into_iter().enumerate() {
            let at = format!("{at}.seats[{index}]");
            let provider: Provider = seat
                .provider
                .parse()
                .with_context(|| format!("{at}.provider"))?;
            let policy = set
                .get(&seat.policy)
                .with_context(|| format!("{at}.policy"))?;
            if let Some(asker) = askers.iter().find(|asker| !asker.contains(policy)) {
                bail!(
                    "{at}.policy: {} grants more than {}, which this shape lets ask, \
                     and a seat never gets more than the session that asked for it",
                    policy.name,
                    asker.name
                );
            }
            seats.push(Seat {
                provider,
                model: seat.model,
                policy: seat.policy,
                role: seat.role,
            });
        }
        Ok(Shape {
            name: self.name,
            seats,
            cap_per_hour: self.cap_per_hour,
            askers: self.askers,
        })
    }
}
