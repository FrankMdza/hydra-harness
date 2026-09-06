//! The capability table, harness spec 7.1.
//!
//! A policy is written in capabilities and a provider speaks tool names, so
//! Hydra owns the mapping between them. The three providers share no vocabulary,
//! and a policy written in one provider's words does not survive being handed to
//! another, which is why the table is per provider and the policy is not.
//!
//! **Every name the provider offers needs a row.** 7.1 denies a tool name the
//! table does not recognise, so a table holding only the nine names that answer
//! to the seven capabilities would deny the other twenty under `permissive`,
//! which is supposed to allow everything. Those twenty get
//! [`Capability::ProviderNative`] instead, one row rather than twenty invented
//! categories, granted by `permissive` alone.
//!
//! `Skill` used to be the twenty-first. Ticket 39 gave it `skill_invoke` on
//! 2026-09-06, which is what splitting `ProviderNative` costs when a policy
//! needs to grant one of them: a row here and an entry in four policy files.
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
/// compares this table against what the provider actually offered and a name
/// the table does not hold comes back as a finding on the normalized run. The
/// four `Task*` rows are the first time that fired: every 2.1.263 record written
/// before they were added carries the finding.
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
    // The twenty. Each exists only inside this provider's own product surface
    // and none is granted by any policy in 7.2.
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
    // 7.1: its own capability since ticket 39. A run that is linked a skill and
    // cannot invoke it was given a listing entry rather than a skill.
    tool("Skill", Capability::SkillInvoke),
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

/// `claude` names an MCP tool `mcp__<server>__<tool>`.
pub const CLAUDE_MCP_PREFIX: &str = "mcp__";

/// `cursor-agent` 2026.09.02-c22c1a3's tool names as its `preToolUse` hook
/// spells them, measured 2026-09-06.
///
/// **There is no init frame to check this against.** `claude` and `agy` list
/// every tool at process start and Cursor does not (research 14), so the only
/// source is the hook itself: five names arrived on a real run and the rest are
/// read off the bundle's hook descriptors, each of which builds the
/// `tool_input` the hook sees. A name that arrives and is not here is denied
/// as unknown, which is 7.1's stance, and the denial is what reveals it.
///
/// Two spellings are Cursor's own and worth knowing before they surprise
/// anybody. The directory-listing tool arrives as `Grep` carrying an
/// `output_mode`, not as a tool of its own. And a `Write` fires a `Read` hook
/// for the same `tool_use_id` first, so a write under `read-only` is refused
/// on the second hook rather than the first.
pub const CURSOR: &[Tool] = &[
    // Measured live on 2026-09-06.
    tool("Shell", Capability::Shell),
    pathed("Read", Capability::FileRead, "file_path"),
    pathed("Write", Capability::FileWrite, "file_path"),
    // A delete is a write to the path it names.
    pathed("Delete", Capability::FileWrite, "file_path"),
    pathed("Grep", Capability::FileRead, "file_path"),
    // Read off the bundle's `createToolInput` table, same build.
    pathed("List", Capability::FileRead, "file_path"),
    pathed("ReadLints", Capability::FileRead, "file_path"),
    tool("Fetch", Capability::NetworkRead),
    // Feeds stdin to a background shell command. It is shell, and the floor
    // cannot see it, which is 7.4's failure mode on a second spelling.
    tool("WriteShellStdin", Capability::Shell),
    // 7.6: denied under every policy but `permissive`.
    tool("task", Capability::SubagentSpawn),
    tool("FetchMcpResource", Capability::McpCall),
    tool("ListMcpResources", Capability::McpCall),
    // Tools that exist only inside Cursor's own product, granted by
    // `permissive` alone.
    tool("ComputerUse", Capability::ProviderNative),
    tool("RecordScreen", Capability::ProviderNative),
    tool("askQuestion", Capability::ProviderNative),
    tool("createPlan", Capability::ProviderNative),
    tool("generateImage", Capability::ProviderNative),
    tool("updateTodos", Capability::ProviderNative),
];

pub const CURSOR_COMMAND_FIELD: &str = "command";

/// `cursor-agent` hands an MCP tool to `preToolUse` as `MCP:<tool>`, read off
/// the bundle on 2026-09-06 and unmeasured on a live run, because no server is
/// configured on this machine.
pub const CURSOR_MCP_PREFIX: &str = "MCP:";

/// One provider's table, with the two per-provider spellings beside it.
pub struct Table {
    pub tools: &'static [Tool],
    pub command_field: &'static str,
    pub mcp_prefix: &'static str,
}

impl Table {
    pub fn of(provider: crate::provider::Provider) -> Self {
        use crate::provider::Provider;
        match provider {
            Provider::Claude => Table {
                tools: CLAUDE,
                command_field: CLAUDE_COMMAND_FIELD,
                mcp_prefix: CLAUDE_MCP_PREFIX,
            },
            Provider::CursorAgent => Table {
                tools: CURSOR,
                command_field: CURSOR_COMMAND_FIELD,
                mcp_prefix: CURSOR_MCP_PREFIX,
            },
        }
    }

    /// What a tool name answers to, or `None` when the table does not know it.
    ///
    /// The MCP prefix is matched before the table, exactly as `capability_of`
    /// does in `gate.py`, and the two must agree: the gate uses it to decide and
    /// the normalizer uses it to fill `tool_call.capability` (contract 7.6).
    /// `None` is the answer 7.1 wants a reader to act on.
    pub fn capability(&self, tool: &str) -> Option<Capability> {
        if tool.starts_with(self.mcp_prefix) {
            return Some(Capability::McpCall);
        }
        self.tools
            .iter()
            .find(|entry| entry.name == tool)
            .map(|entry| entry.capability)
    }
}

/// What capability a `claude` tool name answers to, or `None` when the table
/// does not know the name. [`Table::capability`] for one provider.
pub fn claude_capability(tool: &str) -> Option<Capability> {
    Table::of(crate::provider::Provider::Claude).capability(tool)
}

/// The same for `cursor-agent`.
pub fn cursor_capability(tool: &str) -> Option<Capability> {
    Table::of(crate::provider::Provider::CursorAgent).capability(tool)
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
        for t in CLAUDE.iter().chain(CURSOR) {
            if matches!(t.capability, Capability::FileWrite | Capability::FileRead) {
                assert!(
                    t.path_field.is_some(),
                    "{} is scoped by path and the gate has no field to read",
                    t.name
                );
            }
        }
    }

    #[test]
    fn the_cursor_table_holds_the_names_the_hook_delivered() {
        // Off a real run on 2026-09-06, 2026.09.02-c22c1a3: six tool calls, five
        // distinct names, because the listing tool arrived as `Grep`.
        for name in ["Read", "Shell", "Write", "Grep", "Delete"] {
            assert!(
                CURSOR.iter().any(|t| t.name == name),
                "{name} arrived on the hook and has no row, so 7.1 denies it"
            );
        }
        assert_eq!(cursor_capability("MCP:ping"), Some(Capability::McpCall));
        assert_eq!(
            cursor_capability("mcp__hydra__ping"),
            None,
            "claude's prefix is not Cursor's"
        );
        assert_eq!(cursor_capability("Bash"), None);
    }

    #[test]
    fn no_cursor_tool_appears_twice() {
        let mut names: Vec<_> = CURSOR.iter().map(|t| t.name).collect();
        names.sort_unstable();
        let before = names.len();
        names.dedup();
        assert_eq!(before, names.len());
    }
}
