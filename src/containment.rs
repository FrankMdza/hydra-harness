//! Whether the process calling `hydra run` is itself inside a run.
//!
//! Harness spec 7.3 says a child is never wider than its parent and that a human
//! at depth 0 is the only thing that widens anything. Until build ticket 57
//! nothing enforced it. `Lineage::of` returns depth 0 with no bound when
//! `--parent` is absent, and no mark said a process was inside a run: no
//! environment variable, no cwd test, nothing on the record. `standard` grants
//! shell, so a run could reach the binary and launch `--policy permissive` with
//! no flag to forge and no lie to tell. The adversarial review of 2026-09-14
//! called it the one critical finding, `docs/reviews/2026-09-14-the-gate-under-attack.md`.
//!
//! **What this closes and what it does not.** Hydra sets [`MARKER`] in every
//! gated child's environment, and `hydra run` refuses a launch that carries it
//! without a `--parent` naming the same run. That closes the absent flag, which
//! is the whole of the attack as measured: no forging was needed. It does not
//! close a child that unsets the variable, because a variable is a thing a
//! process can remove from its own children's environment. Map ticket 62 asks
//! whether this class wants a second uid and file modes instead, which is the
//! first thing in this design that would need something from the machine.
//! `SECURITY.md` states the residual rather than letting the check read as
//! containment.
//!
//! **The marker names a run and is checked against the corpus.** A value naming
//! no run under this layout, or naming one that has ended, is refused rather
//! than read as permission: a stale marker is the shape a relocated `HYDRA_ROOT`
//! takes, since a child that moves the root moves the corpus the marker is
//! looked up in and the lookup fails.

use anyhow::Result;

use crate::paths::Layout;

/// The name Hydra sets in a gated child's environment, build ticket 57.
///
/// It is a Hydra-set variable in the sense of harness spec 6.2's second
/// category, like `CURSOR_DATA_DIR`, so it lands in `run_start.env_allowlist`
/// and a reader can see that the child was marked.
pub const MARKER: &str = "HYDRA_RUN_ID";

/// What `hydra run` found about the process that invoked it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Caller {
    /// No marker: a human, or a process Hydra did not launch.
    Outside,
    /// Inside the run this names, which is live under this layout.
    Inside(String),
    /// A marker naming a run this layout has no live record of. Refused, never
    /// read as permission.
    Stale(String),
}

/// Reads the marker out of this process's environment and classifies it.
pub fn caller(layout: &Layout) -> Result<Caller> {
    let marker = std::env::var_os(MARKER).map(|value| value.to_string_lossy().to_string());
    Ok(classify(layout, marker.as_deref()))
}

/// The marker's meaning under `layout`, with the reading of the environment left
/// to the caller so a test can drive this without touching a process-wide
/// variable.
pub fn classify(layout: &Layout, marker: Option<&str>) -> Caller {
    let Some(marker) = marker.filter(|value| !value.is_empty()) else {
        return Caller::Outside;
    };
    match crate::live::read_view(&layout.run_file(marker)) {
        Ok(view) if view.is_live() => Caller::Inside(marker.to_string()),
        _ => Caller::Stale(marker.to_string()),
    }
}

impl Caller {
    /// The remediation for a launch this caller may not make, or `None`.
    ///
    /// A run that names its parent is doing what 7.3 asks, and `Lineage` takes it
    /// from there: the parent's policy is the widest this one may request, and the
    /// depth cap is checked beside it. What this catches is the launch that names
    /// no parent at all, which arrives at `Lineage::of` as a human at depth 0.
    pub fn escalation(&self, parent: Option<&str>) -> Option<String> {
        match self {
            Self::Outside => None,
            Self::Stale(marker) => Some(format!(
                "this process carries {MARKER}={marker}, and this machine has no live run by \
                 that name. A marker Hydra set names a run that is still going, so either the \
                 parent ended while this launch was being prepared or {MARKER} was written by \
                 something else. Either way it is not a bound, and a child launched here would \
                 be bounded by nothing."
            )),
            Self::Inside(marker) => match parent {
                None => Some(format!(
                    "this process is inside run {marker} and named no parent, so it asked to \
                     be launched at depth 0 with no policy above it. Harness spec 7.3 gives a \
                     child its parent's policy as the widest it may request, and only a human \
                     at depth 0 widens anything. Pass --parent {marker}."
                )),
                Some(named) if named != marker => Some(format!(
                    "this process is inside run {marker} and named {named} as its parent. A \
                     run may only parent a child to itself, because 7.3's bound is the policy \
                     of the run the child actually came from. Pass --parent {marker}."
                )),
                Some(_) => None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_caller_outside_a_run_may_launch_anything() {
        assert_eq!(Caller::Outside.escalation(None), None);
        assert_eq!(Caller::Outside.escalation(Some("01a0")), None);
    }

    #[test]
    fn a_caller_inside_a_run_must_name_that_run() {
        let inside = Caller::Inside("01a0".into());
        assert!(inside.escalation(None).is_some(), "no parent is the attack");
        assert!(inside.escalation(Some("01a1")).is_some(), "somebody else's");
        assert_eq!(inside.escalation(Some("01a0")), None);
    }

    #[test]
    fn a_stale_marker_is_refused_whatever_parent_is_named() {
        let stale = Caller::Stale("01a0".into());
        assert!(stale.escalation(None).is_some());
        assert!(stale.escalation(Some("01a0")).is_some());
    }
}
