//! The secret classes Hydra's write path, export and records share.
//!
//! This crate holds the table only. Each reader matches in its own way.
//! `hydra-records` counts Python code points and export counts bytes.

/// One known secret class.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Class {
    /// The substring a matcher looks for.
    pub prefix: &'static str,
    /// The name a redactor writes into `[REDACTED:<label>]`.
    pub label: &'static str,
    /// True when the write-path sweep puts this class on `secret_flag`.
    pub write_path: bool,
}

/// The twelve prefixes in the order the Python replaces them.
pub const CLASSES: &[Class] = &[
    Class {
        prefix: "AIza",
        label: "google_api_key",
        write_path: true,
    },
    Class {
        prefix: "sk-ant-",
        label: "anthropic_api_key",
        write_path: true,
    },
    Class {
        prefix: "ghp_",
        label: "github_pat",
        write_path: true,
    },
    Class {
        prefix: "github_pat_",
        label: "github_fine_grained_pat",
        write_path: false,
    },
    Class {
        prefix: "gho_",
        label: "github_oauth_token",
        write_path: false,
    },
    Class {
        prefix: "ya29.",
        label: "google_oauth_token",
        write_path: true,
    },
    Class {
        prefix: "AKIA",
        label: "aws_access_key",
        write_path: false,
    },
    Class {
        prefix: "aws_secret_access_key=",
        label: "aws_secret_access_key",
        write_path: false,
    },
    Class {
        prefix: "AWS_SECRET_ACCESS_KEY=",
        label: "aws_secret_access_key",
        write_path: false,
    },
    Class {
        prefix: "xoxb-",
        label: "slack_bot_token",
        write_path: false,
    },
    Class {
        prefix: "hooks.slack.com",
        label: "slack_webhook",
        write_path: false,
    },
    Class {
        prefix: "-----BEGIN",
        label: "pem_private_key",
        write_path: true,
    },
];

/// Body length for `sk-proj-`, `sk-svcacct-` and `sk-admin-` keys.
pub const OPENAI_PROJECT_BODY: usize = 20;

/// Body length for a legacy `sk-` key.
pub const OPENAI_LEGACY_BODY: usize = 32;

/// Kind suffixes after `sk-` for project-style keys, in matcher order.
pub const OPENAI_KINDS: [&str; 3] = ["proj-", "svcacct-", "admin-"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_path_is_exactly_the_five() {
        let flagged: Vec<_> = CLASSES
            .iter()
            .filter(|class| class.write_path)
            .map(|class| (class.prefix, class.label))
            .collect();
        assert_eq!(
            flagged,
            vec![
                ("AIza", "google_api_key"),
                ("sk-ant-", "anthropic_api_key"),
                ("ghp_", "github_pat"),
                ("ya29.", "google_oauth_token"),
                ("-----BEGIN", "pem_private_key"),
            ]
        );
    }
}
