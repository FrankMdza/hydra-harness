//! `claude`, measured on 2.1.261.
//!
//! The command line is harness spec 6.7's, with one flag held back and one dropped.
//!
//! **`--permission-mode bypassPermissions` is not passed.** 6.7 justifies it by the
//! gate: a hook deny beats it, so the gate does the refusing and a detached child
//! has nobody to answer a prompt. This slice compiles no gate, so that sentence has
//! no subject. The provider's own default permission mode applies instead, and the
//! run records `policy: null` for the same reason.
//!
//! **`< /dev/null` is dropped, because a pty is not a pipe.** 6.7 adds it so
//! `claude` does not wait three seconds for stdin and write `no stdin data received
//! in 3s` into the record. Measured 2026-09-05 on 2.1.261 over a pty Hydra owned:
//! no wait, no warning, first frame in well under a second. The redirect was a
//! finding about a pipe, and Hydra owns a pty because it has to be the parent to
//! tee anyway.

use crate::provider::Observed;

/// The argv after the binary name.
pub struct Launch<'a> {
    pub task: &'a str,
    pub model: Option<&'a str>,
}

pub fn argv(launch: &Launch<'_>) -> Vec<String> {
    let mut argv = vec![
        "--print".to_string(),
        "--output-format".to_string(),
        "stream-json".to_string(),
        // Not optional: --print with stream-json refuses to start without it.
        "--verbose".to_string(),
        // Puts hook_started and hook_response frames on the stream, which is the
        // second, independent source on a permission decision. It also pulls the
        // stdout of the user's own hooks into every recorded run.
        "--include-hook-events".to_string(),
        "--forward-subagent-text".to_string(),
    ];
    if let Some(model) = launch.model {
        argv.push("--model".to_string());
        argv.push(model.to_string());
    }
    argv.push(launch.task.to_string());
    argv
}

/// Reads the model and the session id off a frame.
///
/// Both land on `system`/`init`, and both are on every frame after it. Hydra takes
/// them from the init frame and stops looking, because a reader that wants the
/// session id later takes it from `raw`, which is where it will always be.
pub fn observe(raw: &str) -> Observed {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) else {
        return Observed::default();
    };
    if value.get("type").and_then(|v| v.as_str()) != Some("system") {
        return Observed::default();
    }
    if value.get("subtype").and_then(|v| v.as_str()) != Some("init") {
        return Observed::default();
    }
    Observed {
        model: value
            .get("model")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        session_id: value
            .get("session_id")
            .and_then(|v| v.as_str())
            .map(str::to_string),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_permissive_flag_is_not_on_the_command_line() {
        let argv = argv(&Launch {
            task: "hi",
            model: None,
        });
        assert!(
            !argv.iter().any(|a| a == "bypassPermissions"),
            "the gate does the refusing, and there is no gate yet"
        );
        assert!(!argv.iter().any(|a| a == "--dangerously-skip-permissions"));
    }

    #[test]
    fn verbose_rides_with_stream_json() {
        let argv = argv(&Launch {
            task: "hi",
            model: None,
        });
        assert!(argv.iter().any(|a| a == "--verbose"));
        assert!(argv.iter().any(|a| a == "stream-json"));
    }

    #[test]
    fn the_task_is_the_last_argument() {
        let argv = argv(&Launch {
            task: "do the thing",
            model: Some("sonnet"),
        });
        assert_eq!(argv.last().unwrap(), "do the thing");
    }

    #[test]
    fn the_init_frame_names_the_model_and_the_session() {
        let observed = observe(
            r#"{"type":"system","subtype":"init","model":"claude-sonnet-5","session_id":"abc"}"#,
        );
        assert_eq!(observed.model.as_deref(), Some("claude-sonnet-5"));
        assert_eq!(observed.session_id.as_deref(), Some("abc"));
    }

    #[test]
    fn an_assistant_frame_teaches_the_envelope_nothing() {
        let observed = observe(r#"{"type":"assistant","message":{"model":"claude-sonnet-5"}}"#);
        assert!(observed.model.is_none());
    }
}
