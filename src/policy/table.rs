//! The capability table, harness spec 7.1.
//!
//! A policy is written in capabilities and a provider speaks tool names, so
//! Hydra owns the mapping between them. The three providers share no vocabulary,
//! and a policy written in one provider's words does not survive being handed to
//! another, which is why the table is per provider and the policy is not.
//!
//! **Every name the provider offers needs a row.** 7.1 denies a tool name the
//! table does not recognise, so a table holding only the eight names that answer
//! to the six capabilities would deny the other twenty-one under `permissive`,
//! which is supposed to allow everything. Those twenty-one get
//! [`Capability::ProviderNative`] instead, one row rather than twenty-one
//! invented categories, granted by `permissive` alone.
//!
//! Splitting `ProviderNative` is a design decision and not one this table makes.
//! It costs a policy file edit the day a policy needs to grant one of them.

use crate::policy::Capability;

/// One provider tool: what it can do, and which argument says where.
pub struct Tool {
    pub name: &'static str,
    pub capability: Capability,
    /// The `tool_input` field naming the path this call acts on, when it has one.
    pub path_field: Option<&'static str>,
}

const fn tool(name: &'static str, capability: Capability) -> Tool {
    Tool {
        name,
        capability,
        path_field: None,
    }
}

const fn pathed(name: &'static str, capability: Capability, field: &'static str) -> Tool {
    Tool {
        name,
        capability,
        path_field: Some(field),
    }
}

/// `claude` 2.1.263's twenty-nine tools, read off the `system`/`init` frame on
/// 2026-09-06. 2.1.261 offered twenty-five on 2026-09-05, and the four
/// `Task*` list tools arrived between the two.
///
/// The init frame lists them on every run at no cost, so ticket 03's normalizer
/// compares this table against what the provider actually offered and a rename
/// breaks the next run loudly. Until that lands, the table is a same-day
/// measurement and nothing checks it at run time.
///
/// `mcp__*` is not here. It is a prefix rather than a name, and the gate script
/// matches it before consulting this table.
pub const CLAUDE: &[Tool] = &[
    tool("Bash", Capability::Shell),
    pathed("Read", Capability::FileRead, "file_path"),
    pathed("Write", Capability::FileWrite, "file_path"),
    pathed("Edit", Capability::FileWrite, "file_path"),
    pathed("NotebookEdit", Capability::FileWrite, "notebook_path"),
    tool("WebFetch", Capability::NetworkRead),
    tool("WebSearch", Capability::NetworkRead),
    // 7.6: denied under every policy but `permissive`, because an in-process
    // subagent gets no run id and writes no run file, and D10 makes the record
    // the source of truth for work Ouroboros learns from.
    tool("Task", Capability::SubagentSpawn),
    // The twenty-one. Each exists only inside this provider's own product
    // surface and none is granted by any policy in 7.2.
    tool("CronCreate", Capability::ProviderNative),
    tool("CronDelete", Capability::ProviderNative),
    tool("CronList", Capability::ProviderNative),
    tool("DesignSync", Capability::ProviderNative),
    tool("EnterWorktree", Capability::ProviderNative),
    tool("ExitWorktree", Capability::ProviderNative),
    tool("ListAgents", Capability::ProviderNative),
    tool("Monitor", Capability::ProviderNative),
    tool("PushNotification", Capability::ProviderNative),
    tool("RemoteTrigger", Capability::ProviderNative),
    tool("ReportFindings", Capability::ProviderNative),
    tool("ScheduleWakeup", Capability::ProviderNative),
    tool("SendMessage", Capability::ProviderNative),
    tool("Skill", Capability::ProviderNative),
    // The four `Task*` list tools are new on 2.1.263, in-process task-list tools
    // and no relation to `Task`, which spawns a subagent.
    tool("TaskCreate", Capability::ProviderNative),
    tool("TaskGet", Capability::ProviderNative),
    tool("TaskList", Capability::ProviderNative),
    tool("TaskOutput", Capability::ProviderNative),
    tool("TaskStop", Capability::ProviderNative),
    tool("TaskUpdate", Capability::ProviderNative),
    tool("ToolSearch", Capability::ProviderNative),
];

/// The `tool_input` field carrying the shell command, per provider.
pub const CLAUDE_COMMAND_FIELD: &str = "command";

/// What capability a `claude` tool name answers to, or `None` when the table
/// does not know the name.
///
/// The gate script has this function too, compiled from [`CLAUDE`], and the two
/// must agree: the gate uses it to decide and the normalizer uses it to fill
/// `tool_call.capability` (contract 7.6) and to run 7.1's table check against
/// the `system`/`init` frame. `None` is the answer 7.1 wants a reader to act on,
/// because a provider that renamed a tool should break loudly.
///
/// `mcp__` is a prefix rather than a name, so it is matched before the table,
/// exactly as `capability_of` does in `gate.py`.
pub fn claude_capability(tool: &str) -> Option<Capability> {
    if tool.starts_with("mcp__") {
        return Some(Capability::McpCall);
    }
    CLAUDE
        .iter()
        .find(|entry| entry.name == tool)
        .map(|entry| entry.capability)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_covers_every_tool_the_init_frame_listed() {
        // The twenty-nine, verbatim off the init frame of run
        // 01a07512-d36e-7651-b8d9-e9e279e5730b, measured 2026-09-06 on 2.1.263.
        // Research 20 measured twenty-five of them on 2.1.261 the day before.
        let measured = [
            "Task",
            "Bash",
            "CronCreate",
            "CronDelete",
            "CronList",
            "DesignSync",
            "Edit",
            "EnterWorktree",
            "ExitWorktree",
            "ListAgents",
            "Monitor",
            "NotebookEdit",
            "PushNotification",
            "Read",
            "RemoteTrigger",
            "ReportFindings",
            "ScheduleWakeup",
            "SendMessage",
            "Skill",
            "TaskCreate",
            "TaskGet",
            "TaskList",
            "TaskOutput",
            "TaskStop",
            "TaskUpdate",
            "ToolSearch",
            "WebFetch",
            "WebSearch",
            "Write",
        ];
        assert_eq!(measured.len(), 29);
        for name in measured {
            assert!(
                CLAUDE.iter().any(|t| t.name == name),
                "{name} is offered by the provider and has no row, so 7.1 denies it"
            );
        }
        assert_eq!(
            CLAUDE.len(),
            measured.len(),
            "the table names a tool the provider does not offer"
        );
    }

    #[test]
    fn no_tool_appears_twice() {
        let mut names: Vec<_> = CLAUDE.iter().map(|t| t.name).collect();
        names.sort_unstable();
        let before = names.len();
        names.dedup();
        assert_eq!(before, names.len());
    }

    #[test]
    fn every_write_tool_names_the_field_the_gate_checks() {
        for t in CLAUDE {
            if matches!(t.capability, Capability::FileWrite | Capability::FileRead) {
                assert!(
                    t.path_field.is_some(),
                    "{} is scoped by path and the gate has no field to read",
                    t.name
                );
            }
        }
    }
}
