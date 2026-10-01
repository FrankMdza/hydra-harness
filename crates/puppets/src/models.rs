//! `models.py`: map a recorded provider and model spelling to one canonical pair, from
//! `aliases.json` as it is.
//!
//! Only the half of `sheet.py` that builds sheets calls this, and that half is not ported
//! yet, so nothing in this crate reaches it but its test.

use std::path::Path;

use anyhow::{Result, anyhow};

use crate::json::{self, Value};
use crate::py;
use crate::write::read_text;

/// A provider and a model; the model is `Null` for a provider's own default.
pub type Pair = (Value, Value);

/// `load_aliases`: each row's `(provider, recorded_model)` and the canonical pair it maps
/// to. A later row with the same recorded pair replaces the earlier one.
pub fn load_aliases(path: &Path) -> Result<Vec<(Pair, Pair)>> {
    let rows = json::loads_str(&read_text(path)?)
        .map_err(|err| anyhow!("{}: {}", err.class(), path.display()))?;
    let mut table: Vec<(Pair, Pair)> = Vec::new();
    for row in py::list(&rows)? {
        let recorded = (
            py::item(&row, "provider")?.clone(),
            py::get(&row, "recorded_model", Value::Null)?,
        );
        py::hashable(&recorded.0)?;
        py::hashable(&recorded.1)?;
        let canonical = py::item(&row, "canonical")?;
        let canonical = (
            py::item(canonical, "provider")?.clone(),
            py::get(canonical, "model", Value::Null)?,
        );
        match table.iter_mut().find(|(known, _)| same(known, &recorded)) {
            Some(slot) => slot.1 = canonical,
            None => table.push((recorded, canonical)),
        }
    }
    Ok(table)
}

fn same(a: &Pair, b: &Pair) -> bool {
    py::same_key(&a.0, &b.0) && py::same_key(&a.1, &b.1)
}

/// `canonical`: the pair a recorded spelling maps to, or the spelling itself when no row
/// names it.
pub fn canonical(provider: &Value, model: &Value, aliases: &[(Pair, Pair)]) -> Result<Pair> {
    py::hashable(provider)?;
    py::hashable(model)?;
    let recorded = (provider.clone(), model.clone());
    Ok(aliases
        .iter()
        .find(|(known, _)| same(known, &recorded))
        .map_or(recorded, |(_, canonical)| canonical.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log;

    #[test]
    fn the_repo_aliases_map_a_recorded_spelling_to_its_canonical_pair() {
        let aliases = load_aliases(&log::checkout_data_dir().join("aliases.json")).unwrap();
        let pair = |provider: &str, model: Option<&str>| {
            let model = model.map_or(Value::Null, Value::text);
            let (provider, model) = canonical(&Value::text(provider), &model, &aliases).unwrap();
            (py::str(&provider).lossy(), py::repr(&model))
        };
        assert_eq!(
            pair("claude", Some("Haiku 4.5")),
            ("claude".into(), "'haiku'".into())
        );
        assert_eq!(pair("claude", None), ("claude".into(), "None".into()));
        // A spelling no row names is its own canonical pair.
        assert_eq!(
            pair("nobody", Some("nothing")),
            ("nobody".into(), "'nothing'".into())
        );
    }
}
