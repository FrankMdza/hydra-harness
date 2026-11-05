use super::*;

/// What the parent's record says about a child, harness spec 6.1 items 2 and 3.
///
/// **The parent is a run id and its record is the only source.** Contract 4 makes
/// `parent_run_id` an edge the parent writes at spawn and never reads back off the
/// child, so the two facts a launch needs about its parent, how deep it is and
/// what policy bounds it, come off the parent's own line 0.
///
/// **Both checks run before the child exists and only one of them writes a
/// record.** [`Lineage::of`] refuses a tree already at the cap with a typed error
/// and no run id, because contract 8.3 promises a record for the narrowing
/// refusal and for nothing else. [`Lineage::narrowing`] is the one that writes,
/// and 7.3 says why: attempted escalation is exactly the signal the corpus should
/// carry.
#[derive(Debug)]
pub struct Lineage {
    /// The run whose policy bounds this one, 7.3.
    parent: Option<Parent>,
    /// Whether `parent` spawned this run. `false` on a resume, build ticket 389:
    /// the bound is the original parent's, and the edge contract 4 makes the
    /// parent write at spawn is not there, because no parent spawned the resume.
    spawned: bool,
    /// The parent's depth plus one, and 0 for a run a human started. A resume
    /// continues its source at the source's depth.
    pub depth: u32,
}

/// The two fields a child reads off its parent's record.
#[derive(Debug)]
struct Parent {
    run_id: String,
    /// `None` only for a harness parent, contract 5.5: a `hydra evolve` pass has
    /// no policy because no gate was compiled for it, and its children are the
    /// proposer and the judges the human's own command spawns. Any other record
    /// that states no policy states no bound, and [`Lineage::of`] refuses it
    /// rather than reading the absence as permission.
    policy: Option<String>,
}

impl Lineage {
    /// Reads the parent's record and computes the child's depth, 6.1 item 2.
    ///
    /// Three ways this refuses, all of them before a run id exists and all of them
    /// errors rather than records: a parent this machine has no record of, a
    /// parent whose record states no policy, and a child that would land past
    /// [`crate::live::MAX_DEPTH`].
    pub fn of(layout: &Layout, parent: Option<&str>) -> Result<Self> {
        let Some(parent) = parent else {
            return Ok(Self {
                parent: None,
                spawned: false,
                depth: 0,
            });
        };

        let run_file = layout.run_file(parent);
        let Some(start) = crate::live::opening_line(&run_file)? else {
            anyhow::bail!(
                "there is no run record at {}, so how deep {parent} is and what policy \
                 bounds it cannot be read, and a child of it would be launched bounded by \
                 nothing. `hydra status` lists the runs this machine has.",
                run_file.display()
            );
        };

        // A harness run, contract 5.5: `command: evolve`, no provider, no policy,
        // because nothing was gated. Spec 4.10 makes it the parent of the proposer
        // and the four judges so a pass can be grouped and a Ctrl-C leaves no
        // orphans, and 7.3's bound is the human's who typed the pass, which is no
        // bound. Measured 2026-09-08 on the first real pass, 01a08127-6f58, which
        // this refusal ended before the proposer existed.
        let harness_parent = start["command"].as_str() == Some("evolve")
            && start["launch_mode"].is_null()
            && start["policy"].is_null();
        let policy = match start["policy"].as_str() {
            Some(policy) => Some(policy.to_string()),
            None if harness_parent => None,
            None => anyhow::bail!(
                "run {parent} has no policy on its record, which is a run written before \
                 the gate reached a provider or an interactive one (contract 5.6). Harness \
                 spec 7.3 bounds a child by its parent's policy and this parent states \
                 none, so it cannot delegate. Start the work at depth 0 with `hydra run`."
            ),
        };

        let depth = start["depth"].as_u64().and_then(|d| u32::try_from(d).ok());
        let Some(depth) = depth else {
            anyhow::bail!(
                "run {parent} has no usable depth on its record, and a child's depth is \
                 its parent's plus one (harness spec 6.1). The record is {}.",
                run_file.display()
            );
        };

        // Compared rather than incremented, because a record claiming a depth of
        // `u32::MAX` is a torn or hand-edited line and adding one to it panics.
        if depth >= crate::live::MAX_DEPTH {
            return Err(TooDeep {
                parent_run_id: parent.to_string(),
                parent_depth: depth,
                cap: crate::live::MAX_DEPTH,
            }
            .into());
        }

        Ok(Self {
            parent: Some(Parent {
                run_id: parent.to_string(),
                policy,
            }),
            spawned: true,
            depth: depth + 1,
        })
    }

    /// The lineage of the launch `request` describes, a spawn's or a resume's.
    ///
    /// **A resume continues its source, so it sits where the source sat.** Build
    /// ticket 389: `hydra resume` launches with no `--parent`, and read through
    /// [`Lineage::of`] alone a resumed depth 2 child came back at depth 0, two
    /// levels under the cap and bounded by nothing. The source's line 0 has both
    /// facts. Its envelope's `parent_run_id` names the run that bounds it, and
    /// the resume is checked against that run as if the run had spawned it; a
    /// source that was itself resumed carries no edge, so `resumed_from` is
    /// followed back to the run that was spawned. The resumed run's own record
    /// still writes no `parent_run_id` and no `parent_policy`, because contract 4
    /// makes the edge the parent's to write at spawn and no parent spawned it.
    pub fn for_request(layout: &Layout, request: &Request) -> Result<Self> {
        let (Some(source), None) = (request.resumed_from.as_deref(), request.parent.as_deref())
        else {
            return Self::of(layout, request.parent.as_deref());
        };

        let mut current = source.to_string();
        let mut seen = std::collections::HashSet::new();
        loop {
            if !seen.insert(current.clone()) {
                anyhow::bail!(
                    "run {source}'s `resumed_from` chain comes back to {current}, so where \
                     the run sits in its tree cannot be read and the resume is refused."
                );
            }
            let run_file = layout.run_file(&current);
            let envelope = crate::live::first_line(&run_file)?
                .and_then(|line| serde_json::from_str::<serde_json::Value>(&line).ok())
                .filter(|envelope| envelope["hydra"]["type"].as_str() == Some("run_start"));
            let Some(envelope) = envelope else {
                anyhow::bail!(
                    "there is no run_start at {}, so how deep resumed run {source} sits and \
                     what policy bounds it cannot be read.",
                    run_file.display()
                );
            };
            if let Some(parent) = envelope["parent_run_id"].as_str() {
                let mut lineage = Self::of(layout, Some(parent))?;
                lineage.spawned = false;
                return Ok(lineage);
            }
            match envelope["hydra"]["resumed_from"].as_str() {
                Some(previous) => current = previous.to_string(),
                None => {
                    // A run a human started, or a harness pass: no bound, and its
                    // own depth, which is 0 on every record this build writes.
                    let depth = envelope["hydra"]["depth"]
                        .as_u64()
                        .and_then(|d| u32::try_from(d).ok())
                        .with_context(|| {
                            format!(
                                "run {current} has no usable depth on its record, so resumed \
                                 run {source}'s depth cannot be read"
                            )
                        })?;
                    return Ok(Self {
                        parent: None,
                        spawned: false,
                        depth,
                    });
                }
            }
        }
    }

    /// The edge contract 4 writes, `None` on a resume.
    pub fn parent_run_id(&self) -> Option<&str> {
        self.parent
            .as_ref()
            .filter(|_| self.spawned)
            .map(|parent| parent.run_id.as_str())
    }

    /// Contract 5.1's `parent_policy`, absent where the edge is.
    pub fn parent_policy(&self) -> Option<&str> {
        self.parent
            .as_ref()
            .filter(|_| self.spawned)
            .and_then(|parent| parent.policy.as_deref())
    }

    /// 6.1 item 3, the containment test. `None` when the child may run, and the
    /// remediation a human reads when it may not.
    ///
    /// **Containment is a partial order and this is where that costs something.**
    /// `curate` writes to its checkout of the store and cannot touch a worktree,
    /// `standard` is the reverse, and neither contains the other. 7.3 refuses an
    /// incomparable pair rather than resolving it, so a `standard` parent cannot
    /// delegate a `curate` child and the remediation says which case this is.
    pub fn narrowing(
        &self,
        policies: &crate::policy::PolicySet,
        child: &str,
    ) -> Result<Option<String>> {
        let Some(parent) = &self.parent else {
            return Ok(None);
        };
        // A harness parent bounds nothing: see [`Parent::policy`].
        let Some(parent_policy) = parent.policy.as_deref() else {
            return Ok(None);
        };
        let bound = policies.get(parent_policy).with_context(|| {
            format!(
                "run {} names a policy this binary does not have, so there is nothing for \
                 7.3 to compare against",
                parent.run_id
            )
        })?;
        let asked = policies.get(child)?;
        if bound.contains(asked) {
            return Ok(None);
        }

        // Two different failures with one refusal, and the words matter to
        // whoever reads them: one is an attempted escalation, the other is 7.3's
        // accepted cost showing up in a legitimate request.
        let relation = if asked.contains(bound) {
            format!("`{child}` is wider than `{parent_policy}`")
        } else {
            format!(
                "`{child}` and `{parent_policy}` are incomparable, because each grants \
                 something the other does not"
            )
        };
        Ok(Some(format!(
            "{relation}, and harness spec 7.3 never lets a child hold a capability its \
             parent does not. Run {} is the parent{}. Ask for `{parent_policy}` or a policy \
             it contains, or start this work at depth 0 yourself.",
            parent.run_id,
            if self.spawned {
                ""
            } else {
                " of the run this resumes"
            }
        )))
    }
}

/// The depth cap refused this spawn, 6.1 item 2.
///
/// **A typed error rather than a record**, which is the shape 6.1 item 4 gives the
/// concurrency cap. Contract 8.3 promises a run record for the narrowing refusal
/// and for nothing else, and the caller here is itself a run whose record already
/// carries the `delegate` call and the error it got back. A run id minted to say
/// "this never ran" would put a row in the corpus with no policy, no task and no
/// tree under it.
#[derive(Debug)]
pub struct TooDeep {
    pub parent_run_id: String,
    pub parent_depth: u32,
    pub cap: u32,
}

impl std::fmt::Display for TooDeep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "run {} is at depth {} and the cap is {}, so a child of it would be depth {} \
             and this spawn is refused. A child that delegates is how three subscriptions \
             get drained overnight (harness spec 9), and section 7.6 closes the cap's \
             blind spot by denying in-process subagents. Start the work you wanted at \
             depth 0.",
            self.parent_run_id,
            self.parent_depth,
            self.cap,
            self.parent_depth.saturating_add(1)
        )
    }
}

impl std::error::Error for TooDeep {}
