//! The capability table, harness spec 7.1.
//!
//! A policy is written in capabilities and a provider speaks tool names, so
//! Hydra owns the mapping between them. The three providers share no vocabulary,
//! and a policy written in one provider's words does not survive being handed to
//! another, which is why the table is per provider and the policy is not.
//!
//! **Every name the provider offers needs a row.** 7.1 denies a tool name the
//! table does not recognise, so a table holding only the nine names that answer
//! to the seven capabilities would deny the other nineteen under `permissive`,
//! which is supposed to allow everything. Those nineteen get
//! [`Capability::ProviderNative`] instead, one row rather than nineteen invented
//! categories, granted by `permissive` alone.
//!
//! `Skill` and `ToolSearch` used to be in there, making twenty-one. Ticket 39
//! moved the first to `skill_invoke` on 2026-09-06 and ticket 43 moved the second
//! the same day.
//!
//! Splitting `ProviderNative` is a design decision and not one this table makes.
//! It costs a row here, plus an entry in four policy files when the capability it
//! moves to is one no policy grants yet, which is what `Skill` cost and
//! `ToolSearch` did not.

use crate::policy::Capability;

/// One provider tool: what it can do, and which argument says where.
pub struct Tool {
    pub name: &'static str,
    pub capability: Capability,
    /// The `tool_input` field naming the path this call acts on, when it has one.
    pub path_field: Option<&'static str>,
    /// When true, paths are parsed from the patch body in the provider's
    /// `command_field` and from `changes[].path` on replay, not from one path
    /// argument.
    pub patch_paths: bool,
}

const fn tool(name: &'static str, capability: Capability) -> Tool {
    Tool {
        name,
        capability,
        path_field: None,
        patch_paths: false,
    }
}

const fn pathed(name: &'static str, capability: Capability, field: &'static str) -> Tool {
    Tool {
        name,
        capability,
        path_field: Some(field),
        patch_paths: false,
    }
}

/// A `file_write` whose paths live in a patch body rather than one field.
const fn patch_write(name: &'static str) -> Tool {
    Tool {
        name,
        capability: Capability::FileWrite,
        path_field: None,
        patch_paths: true,
    }
}

/// A tool the provider runs without asking the hook. The replay records the
/// provider's own decision; the compiler skips the row, so the gate never
/// sees a name it cannot answer for.
const fn unhooked(name: &'static str) -> Tool {
    Tool {
        name,
        capability: Capability::Unhooked,
        path_field: None,
        patch_paths: false,
    }
}

/// `claude` 2.1.263's twenty-nine tools, read off the `system`/`init` frame on
/// 2026-09-06. 2.1.261 offered twenty-five on 2026-09-05, and the four
/// `Task*` list tools arrived between the two. 2.1.281 renamed `Task` to
/// `Agent` for subagent spawn; the table keeps both so records from 2.1.263
/// through 2.1.280 and 2.1.281 onward both read back (thirty rows).
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
    // the source of truth for work the rungs learn from.
    tool("Task", Capability::SubagentSpawn),
    // 2.1.281 renamed `Task` to `Agent`, measured 2026-09-24 on runs
    // 01a0d229-cd8f, 01a0d23c-ccc9 and 01a0d3e4-4245.
    tool("Agent", Capability::SubagentSpawn),
    // The nineteen. Each exists only inside this provider's own product surface
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
    // 7.1: `skill_invoke` since ticket 43. 2.1.263 hands MCP tools to a print-mode
    // run deferred, so a policy that admits a server and denies this has admitted
    // a listing entry the model cannot call. It fetches schemas and executes
    // nothing, and the gate judges the call that follows.
    tool("ToolSearch", Capability::SkillInvoke),
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
    // Map ticket 51: Cursor runs these without asking `preToolUse`. ReadLints
    // was measured 2026-09-08 under `read-only`; getMcpToolsToolCall arrived
    // the same day as an unknown name with a completed result. The compiler
    // skips the rows, so the gate's compiled config does not name them.
    unhooked("ReadLints"),
    unhooked("getMcpToolsToolCall"),
    // Stays an unhooked row for the replay: the record carries no path. The
    // hook that fires is a `Read` of the task's output under the run's own
    // `cursor-data/` (`beforeReadFile`, or `preToolUse` on `Read`), which
    // `own_read_paths` carves out; if that answer and this row differ, the
    // replay records a Disagreement. Build ticket 38.
    unhooked("awaitToolCall"),
    // Measured 2026-09-24 on cursor-agent 2026.09.23-86fc751 under `standard`.
    unhooked("WebSearch"),
    tool("Fetch", Capability::NetworkRead),
    // Feeds stdin to a background shell command, and carries no command for the
    // floor or the path check to read. It was `shell` until build ticket 59,
    // which is how "denied under every policy" went with it: under `permissive`
    // a run could feed a protected path to a shell it had already opened.
    tool("WriteShellStdin", Capability::OpaqueShell),
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

/// `agy`'s fifty-seven tools, read off the `init` frame on 2026-09-06 under
/// `--model gemini-3.8-flash-low`, on 1.1.26 and again on the recorded 1.1.27 run:
/// the two lists are identical. Research 20 read fifty-six off a Claude id on
/// 1.1.26 two days earlier; `list_permissions` is the difference and whether it is
/// new or model-dependent is unmeasured.
///
/// **The field names are measured and the capabilities are Hydra's reading.** The
/// four in the run above arrived on a real `PreToolUse` payload:
/// `list_dir(DirectoryPath)`, `view_file(AbsolutePath)`,
/// `write_to_file(TargetFile)` and `run_command(CommandLine)`. The rest come off
/// the binary's own `jsonschema` struct tags, which is the same class of evidence
/// Cursor's rows come from: `SearchDirectory` is "The directory to search within.
/// Must be an absolute path" on `find_by_name`, `NotebookPath` is "Absolute path to the .ipynb
/// notebook file", and `TargetFile` on the editing tools is "The target file to
/// modify. Must be an absolute path".
///
/// Like `claude`, this provider lists every tool at process start, so the
/// normalizer compares this table against the init frame and a name the table
/// does not hold comes back as a finding on the normalized run.
///
/// **The browser family is `provider_native` and `execute_browser_javascript`
/// sits in it.** That tool runs code the model wrote, which no policy but
/// `permissive` grants, and the pattern floor of 7.4 cannot see inside it because
/// it is JavaScript in a page rather than a shell command. That is the floor's
/// named failure mode on a second spelling, the same one `WriteShellStdin` has on
/// Cursor.
pub const AGY: &[Tool] = &[
    // Measured on the hook payload, 2026-09-06. **No `path_field`, because a
    // command line is not a path.** The command rides in `command_field` below,
    // which is what the denied paths, their carve-outs and the floor read it
    // out of;
    // `path_fields` is the map the gate resolves a path from, and putting a
    // whole shell command in it would have the gate calling `realpath` on
    // `echo hi && rm -rf /`. `claude`'s `Bash` and Cursor's `Shell` are plain
    // rows for the same reason.
    tool("run_command", Capability::Shell),
    pathed("view_file", Capability::FileRead, "AbsolutePath"),
    pathed("list_dir", Capability::FileRead, "DirectoryPath"),
    pathed("write_to_file", Capability::FileWrite, "TargetFile"),
    // Measured on a real `PreToolUse` payload on 2026-09-08, on 1.1.27, run
    // 01a07ffe-4808: `grep_search` sends `SearchPath`, "a directory or a single
    // file", and the gate compiled with `SearchDirectory` denied it `hydra/no-path`
    // on its first search. `SearchDirectory` is `find_by_name`'s field; the
    // binary's `jsonschema` tags were read against the wrong struct.
    pathed("grep_search", Capability::FileRead, "SearchPath"),
    // Measured on a real payload the same day, run 01a08008-cb50: inside the
    // worktree allowed, `/etc` denied out-of-scope.
    pathed("find_by_name", Capability::FileRead, "SearchDirectory"),
    // Measured on a real payload 2026-09-08, run 01a0801d-77b1: inside the
    // worktree allowed, `/etc/hosts` denied out-of-scope.
    pathed("replace_file_content", Capability::FileWrite, "TargetFile"),
    pathed(
        "multi_replace_file_content",
        Capability::FileWrite,
        "TargetFile",
    ),
    // Both on the init frame's list and both answered "unknown tool" by the
    // binary when a run called them, 2026-09-08 on 1.1.27 under
    // `gemini-3.8-flash-high`. The rows stay: the gate answers a name before
    // the binary does, and a tool the list advertises may exist on another
    // model or build. Neither field name is measured.
    pathed("sed_file", Capability::FileWrite, "TargetFile"),
    pathed("notebook_edit", Capability::FileWrite, "NotebookPath"),
    // Both feed a shell facility rather than the filesystem, and neither carries
    // a command for the floor to read: `send_command_input` writes stdin to a
    // command already running and `command_status` collects its output. They
    // were both `shell` until build ticket 59. The two are not the same risk
    // and no longer carry the same capability: writing into a running command
    // is a command the gate never saw, and reading its output is not. So
    // `send_command_input` is denied under every policy and `command_status`
    // stays `shell`, where a policy that denies shell still denies it.
    tool("send_command_input", Capability::OpaqueShell),
    tool("command_status", Capability::Shell),
    // Runs the notebook's own code. Shell for the same reason.
    tool("notebook_execution", Capability::Shell),
    tool("read_url_content", Capability::NetworkRead),
    tool("search_web", Capability::NetworkRead),
    // **MCP is a tool name here rather than a prefix**, which is why this
    // provider's table carries no `mcp_prefix`. Ticket 23 read the `PreToolUse`
    // matcher as `call_mcp_tool` off the bundle.
    tool("call_mcp_tool", Capability::McpCall),
    tool("list_resources", Capability::McpCall),
    tool("read_resource", Capability::McpCall),
    // 7.6: denied under every policy but `permissive`, because an in-process
    // subagent gets no run id and writes no run file.
    tool("define_subagent", Capability::SubagentSpawn),
    tool("invoke_subagent", Capability::SubagentSpawn),
    tool("manage_subagents", Capability::SubagentSpawn),
    tool("browser_subagent", Capability::SubagentSpawn),
    // No `Skill` row: this provider has no skill-invoking tool, so `skill_invoke`
    // maps to nothing here and a linked skill reaches the model through the
    // listing alone.
    //
    // The rest exist only inside this provider's own product surface and no
    // policy in 7.2 grants them.
    tool("ask_custom_permission", Capability::ProviderNative),
    tool("ask_permission", Capability::ProviderNative),
    tool("ask_question", Capability::ProviderNative),
    tool("browser_click_element", Capability::ProviderNative),
    tool("browser_drag_pixel_to_pixel", Capability::ProviderNative),
    tool("browser_get_dom", Capability::ProviderNative),
    tool("browser_get_network_request", Capability::ProviderNative),
    tool("browser_input", Capability::ProviderNative),
    tool("browser_list_network_requests", Capability::ProviderNative),
    tool("browser_mouse_down", Capability::ProviderNative),
    tool("browser_mouse_up", Capability::ProviderNative),
    tool("browser_move_mouse", Capability::ProviderNative),
    tool("browser_press_key", Capability::ProviderNative),
    tool("browser_refresh_page", Capability::ProviderNative),
    tool("browser_resize_window", Capability::ProviderNative),
    tool("browser_scroll", Capability::ProviderNative),
    tool("browser_scroll_dom", Capability::ProviderNative),
    tool("browser_select_option", Capability::ProviderNative),
    tool("capture_browser_console_logs", Capability::ProviderNative),
    tool("capture_browser_screenshot", Capability::ProviderNative),
    tool("click_browser_pixel", Capability::ProviderNative),
    tool("delete_knowledge", Capability::ProviderNative),
    tool("execute_browser_javascript", Capability::ProviderNative),
    tool("finish", Capability::ProviderNative),
    tool("generate_image", Capability::ProviderNative),
    tool("list_browser_pages", Capability::ProviderNative),
    tool("list_permissions", Capability::ProviderNative),
    tool("manage_inbox", Capability::ProviderNative),
    tool("manage_task", Capability::ProviderNative),
    tool("open_browser_url", Capability::ProviderNative),
    tool("read_browser_page", Capability::ProviderNative),
    tool("schedule", Capability::ProviderNative),
    tool("send_message", Capability::ProviderNative),
    tool("wait", Capability::ProviderNative),
    tool("wait_5_seconds", Capability::ProviderNative),
];

/// Measured on the hook payload beside `Cwd` and `WaitMsBeforeAsync`, and the
/// binary's own schema calls it "The exact command line string to execute".
pub const AGY_COMMAND_FIELD: &str = "CommandLine";

/// `codex` 0.154.0's hooked tools, read off map ticket 58 and the hooks
/// reference on 2026-09-12. There is no init frame to check this against yet:
/// item 5 still needs a logged-in recorded run before the table can be walked
/// against what the provider offered.
///
/// **Reads are `Bash`, not a file tool**, which is the whole of build ticket 48
/// item 4. **`mcp__*` is a prefix**, like `claude`, and matches before the table.
/// **Hosted tools never reach the hook** and carry [`Capability::Unhooked`].
pub const CODEX: &[Tool] = &[
    tool("Bash", Capability::Shell),
    patch_write("apply_patch"),
    tool("update_plan", Capability::ProviderNative),
    tool("spawn_agent", Capability::SubagentSpawn),
    unhooked("WebSearch"),
];

/// The `tool_input` field carrying the shell command or patch body.
pub const CODEX_COMMAND_FIELD: &str = "command";

/// `codex` names an MCP tool `mcp__<server>__<tool>`, measured on 0.154.0.
pub const CODEX_MCP_PREFIX: &str = "mcp__";

/// One provider's table, with the per-provider spellings beside it.
pub struct Table {
    pub tools: &'static [Tool],
    pub command_field: &'static str,
    /// The prefix that marks an MCP tool, on a provider that uses one.
    ///
    /// **`None` on `agy`, which names one tool instead.** `call_mcp_tool` is a
    /// row in the table like any other, so there is no prefix to match, and an
    /// empty string here would match every tool name and turn the whole provider
    /// into `mcp_call`.
    pub mcp_prefix: Option<&'static str>,
}

impl Table {
    pub fn of(provider: crate::provider::Provider) -> Self {
        use crate::provider::Provider;
        match provider {
            Provider::Claude => Table {
                tools: CLAUDE,
                command_field: CLAUDE_COMMAND_FIELD,
                mcp_prefix: Some(CLAUDE_MCP_PREFIX),
            },
            Provider::CursorAgent => Table {
                tools: CURSOR,
                command_field: CURSOR_COMMAND_FIELD,
                mcp_prefix: Some(CURSOR_MCP_PREFIX),
            },
            Provider::Agy => Table {
                tools: AGY,
                command_field: AGY_COMMAND_FIELD,
                mcp_prefix: None,
            },
            Provider::Codex => Table {
                tools: CODEX,
                command_field: CODEX_COMMAND_FIELD,
                mcp_prefix: Some(CODEX_MCP_PREFIX),
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
        if let Some(prefix) = self.mcp_prefix
            && tool.starts_with(prefix)
        {
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

/// The same for `agy`.
pub fn agy_capability(tool: &str) -> Option<Capability> {
    Table::of(crate::provider::Provider::Agy).capability(tool)
}

/// The same for `codex`.
pub fn codex_capability(tool: &str) -> Option<Capability> {
    Table::of(crate::provider::Provider::Codex).capability(tool)
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
        // One row beyond 2.1.263's frame: `Agent`, the name 2.1.281 gave
        // `Task`. Both stay so records on either side of the rename read back.
        // Anything else beyond the frame is a name the provider never offered.
        assert_eq!(
            CLAUDE.len(),
            measured.len() + 1,
            "the table names a tool the provider does not offer"
        );
        assert!(
            CLAUDE.iter().any(|t| t.name == "Agent"),
            "2.1.281 renamed Task to Agent and the table keeps both"
        );
    }

    #[test]
    fn agent_is_task_renamed_on_281() {
        assert_eq!(claude_capability("Agent"), Some(Capability::SubagentSpawn));
        assert_eq!(claude_capability("Task"), Some(Capability::SubagentSpawn));
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
    fn the_agy_table_covers_every_tool_its_init_frame_listed() {
        // Verbatim off the init frame of the 1.1.26 probe on 2026-09-06, under
        // `--model gemini-3.8-flash-low`.
        let measured = [
            "ask_custom_permission",
            "ask_permission",
            "ask_question",
            "browser_click_element",
            "browser_drag_pixel_to_pixel",
            "browser_get_dom",
            "browser_get_network_request",
            "browser_input",
            "browser_list_network_requests",
            "browser_mouse_down",
            "browser_mouse_up",
            "browser_move_mouse",
            "browser_press_key",
            "browser_refresh_page",
            "browser_resize_window",
            "browser_scroll",
            "browser_scroll_dom",
            "browser_select_option",
            "browser_subagent",
            "call_mcp_tool",
            "capture_browser_console_logs",
            "capture_browser_screenshot",
            "click_browser_pixel",
            "command_status",
            "define_subagent",
            "delete_knowledge",
            "execute_browser_javascript",
            "find_by_name",
            "finish",
            "generate_image",
            "grep_search",
            "invoke_subagent",
            "list_browser_pages",
            "list_dir",
            "list_permissions",
            "list_resources",
            "manage_inbox",
            "manage_subagents",
            "manage_task",
            "multi_replace_file_content",
            "notebook_edit",
            "notebook_execution",
            "open_browser_url",
            "read_browser_page",
            "read_resource",
            "read_url_content",
            "replace_file_content",
            "run_command",
            "schedule",
            "search_web",
            "sed_file",
            "send_command_input",
            "send_message",
            "view_file",
            "wait",
            "wait_5_seconds",
            "write_to_file",
        ];
        assert_eq!(measured.len(), 57);
        for name in measured {
            assert!(
                AGY.iter().any(|t| t.name == name),
                "{name} is offered by the provider and has no row, so 7.1 denies it"
            );
        }
        assert_eq!(
            AGY.len(),
            measured.len(),
            "the table names a tool the provider does not offer"
        );
    }

    #[test]
    fn agy_names_its_mcp_tool_instead_of_prefixing_it() {
        assert_eq!(agy_capability("call_mcp_tool"), Some(Capability::McpCall));
        // Neither of the other two providers' prefixes means anything here, and
        // an empty prefix would have made every name an MCP call.
        assert_eq!(agy_capability("mcp__hydra__ping"), None);
        assert_eq!(agy_capability("MCP:ping"), None);
        assert_eq!(agy_capability("run_command"), Some(Capability::Shell));
        assert!(
            Table::of(crate::provider::Provider::Agy)
                .mcp_prefix
                .is_none()
        );
    }

    #[test]
    fn no_agy_tool_appears_twice() {
        let mut names: Vec<_> = AGY.iter().map(|t| t.name).collect();
        names.sort_unstable();
        let before = names.len();
        names.dedup();
        assert_eq!(before, names.len());
    }

    #[test]
    fn every_write_tool_names_the_field_the_gate_checks() {
        for t in CLAUDE.iter().chain(CURSOR).chain(AGY) {
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
    fn unhooked_cursor_rows_are_known_and_carry_no_path() {
        for name in [
            "ReadLints",
            "getMcpToolsToolCall",
            "awaitToolCall",
            "WebSearch",
        ] {
            assert_eq!(
                cursor_capability(name),
                Some(Capability::Unhooked),
                "{name}"
            );
            let row = CURSOR.iter().find(|t| t.name == name).unwrap();
            assert!(
                row.path_field.is_none(),
                "{name} has no field the gate could read"
            );
        }
    }

    #[test]
    fn no_cursor_tool_appears_twice() {
        let mut names: Vec<_> = CURSOR.iter().map(|t| t.name).collect();
        names.sort_unstable();
        let before = names.len();
        names.dedup();
        assert_eq!(before, names.len());
    }

    #[test]
    fn apply_patch_paths_are_parsed_from_the_patch_body_and_from_changes_on_replay() {
        let row = CODEX.iter().find(|t| t.name == "apply_patch").unwrap();
        assert!(row.patch_paths);
        assert!(row.path_field.is_none());
    }

    #[test]
    fn codex_names_mcp_tools_with_claudes_prefix() {
        assert_eq!(
            codex_capability("mcp__hydra__ping"),
            Some(Capability::McpCall)
        );
        assert_eq!(codex_capability("Bash"), Some(Capability::Shell));
        assert_eq!(codex_capability("apply_patch"), Some(Capability::FileWrite));
        assert_eq!(codex_capability("WebSearch"), Some(Capability::Unhooked));
        assert_eq!(
            codex_capability("spawn_agent"),
            Some(Capability::SubagentSpawn)
        );
    }

    #[test]
    fn no_codex_tool_appears_twice() {
        let mut names: Vec<_> = CODEX.iter().map(|t| t.name).collect();
        names.sort_unstable();
        let before = names.len();
        names.dedup();
        assert_eq!(before, names.len());
    }
}
