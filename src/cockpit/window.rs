//! A run's share of its provider's rate-limit window, build ticket 172.
//!
//! **The formula is `integrations/puppets/sheet.py`'s**, ported rather than
//! called: `build` sets a run's `window_share` to `(raw_input *
//! cache_read_weight + output_tokens) / window_budget_tokens` for every
//! canonical provider but `claude`, and to nothing when the budget is null or
//! not positive. The budget is the `vendors.json` row whose provider and model
//! match the run's once both go through `models.py`'s `canonical`, which reads
//! `aliases.json`. Nothing here writes either file.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::normalize::Counts;

/// `sheet.py`'s default, and the only value the cockpit passes.
const CACHE_READ_WEIGHT: f64 = 1.0;

type Key = (String, Option<String>);

#[derive(Deserialize)]
struct Vendor {
    provider: String,
    model: Option<String>,
    window_budget_tokens: Option<f64>,
}

#[derive(Deserialize)]
struct Alias {
    provider: String,
    recorded_model: Option<String>,
    canonical: Canonical,
}

#[derive(Deserialize)]
struct Canonical {
    provider: String,
    model: Option<String>,
}

/// `vendors.json` and `aliases.json`, read once a tick.
pub struct Budgets {
    aliases: HashMap<Key, Key>,
    /// In file order, since `sheet.py` takes the first row that matches.
    vendors: Vec<(Key, Option<f64>)>,
}

impl Budgets {
    /// Reads both files from `dir`. A missing `aliases.json` maps every
    /// spelling to itself; a missing `vendors.json` is an error.
    pub fn load(dir: &Path) -> Result<Self> {
        let path = dir.join("vendors.json");
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        let rows: Vec<Vendor> =
            serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
        let path = dir.join("aliases.json");
        let aliases: Vec<Alias> = match std::fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text)
                .with_context(|| format!("parsing {}", path.display()))?,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(err) => return Err(err).with_context(|| format!("reading {}", path.display())),
        };
        let aliases = aliases
            .into_iter()
            .map(|a| {
                (
                    (a.provider, a.recorded_model),
                    (a.canonical.provider, a.canonical.model),
                )
            })
            .collect();
        let mut budgets = Self {
            aliases,
            vendors: Vec::new(),
        };
        budgets.vendors = rows
            .into_iter()
            .map(|v| {
                let key = budgets.canonical(&v.provider, v.model.as_deref());
                (key, v.window_budget_tokens)
            })
            .collect();
        Ok(budgets)
    }

    /// `models.py`'s `canonical`.
    fn canonical(&self, provider: &str, model: Option<&str>) -> Key {
        let recorded = (provider.to_string(), model.map(str::to_string));
        self.aliases.get(&recorded).cloned().unwrap_or(recorded)
    }

    /// One run's share, or `None` where `sheet.py` writes none.
    pub fn share(&self, provider: &str, model: Option<&str>, counts: &Counts) -> Option<f64> {
        let key = self.canonical(provider, model);
        // sheet.py: claude window budgets come from utilization, not token
        // spend. It asks after `canonical`, so an alias onto claude counts.
        if key.0 == "claude" {
            return None;
        }
        let budget = self
            .vendors
            .iter()
            .find(|(row, _)| *row == key)
            .and_then(|(_, budget)| *budget)
            .filter(|budget| *budget > 0.0)?;
        Some((counts.raw_input() as f64 * CACHE_READ_WEIGHT + counts.output_tokens as f64) / budget)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The committed files parse, extra keys and all, and a `codex` run with no
    /// model is measured against the one budget `vendors.json` holds.
    #[test]
    fn the_committed_vendors_file_loads() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("integrations/puppets");
        let budgets = Budgets::load(&dir).unwrap();
        let counts = Counts {
            input_tokens: 100,
            output_tokens: 50,
            ..Counts::default()
        };
        let budget = budgets
            .vendors
            .iter()
            .find(|(key, _)| *key == ("codex".to_string(), None))
            .and_then(|(_, budget)| *budget)
            .unwrap();
        assert_eq!(budgets.share("codex", None, &counts), Some(150.0 / budget));
        assert_eq!(budgets.share("claude", Some("opus"), &counts), None);
    }
}
