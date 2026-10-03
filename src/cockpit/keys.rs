//! Every key the cockpit answers, grouped for the help overlay.
//! [`HelpEntry::bindings`] is what [`super::app::App::key`] dispatches in the
//! main branch; a [`MainKey`] with no row fails [`every_main_key_has_a_help_line`].

use std::collections::HashMap;
use std::sync::LazyLock;

use crossterm::event::KeyCode;

/// README `## Commands` groups for the cockpit key table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HelpGroup {
    Moving,
    Opening,
    Confirm,
    Views,
    Quit,
    Help,
    PolicyPane,
    Session,
}

/// Keys handled in the main branch of [`super::app::App::key`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MainKey {
    Quit,
    Policy,
    FocusForward,
    FocusBackward,
    MoveUp,
    MoveDown,
    Open,
    CloseOpened,
    ApproveAsk,
    DenyAsk,
    ReviewAsk,
    StopAsk,
    ToggleLanes,
    ToggleRawTail,
    ToggleReveal,
    ToggleHelp,
}

/// One line in the help overlay and the codes it dispatches.
#[derive(Clone, Copy, Debug)]
pub struct HelpEntry {
    pub group: HelpGroup,
    pub keys: &'static str,
    pub description: &'static str,
    pub bindings: &'static [(KeyCode, MainKey)],
}

pub const HELP: &[HelpEntry] = &[
    HelpEntry {
        group: HelpGroup::Moving,
        keys: "arrows, Tab, Shift-Tab",
        description: "move within a panel, or between panels",
        bindings: &[
            (KeyCode::Up, MainKey::MoveUp),
            (KeyCode::Down, MainKey::MoveDown),
            (KeyCode::Left, MainKey::FocusBackward),
            (KeyCode::Right, MainKey::FocusForward),
            (KeyCode::Tab, MainKey::FocusForward),
            (KeyCode::BackTab, MainKey::FocusBackward),
        ],
    },
    HelpEntry {
        group: HelpGroup::Opening,
        keys: "Enter, Esc",
        description: "open the selected run, request or step; close it",
        bindings: &[
            (KeyCode::Enter, MainKey::Open),
            (KeyCode::Esc, MainKey::CloseOpened),
        ],
    },
    HelpEntry {
        group: HelpGroup::Confirm,
        keys: "a, y",
        description: "approve the selected seat request, or run the argv a step listed",
        bindings: &[(KeyCode::Char('a'), MainKey::ApproveAsk)],
    },
    HelpEntry {
        group: HelpGroup::Confirm,
        keys: "d, y",
        description: "deny the selected seat request; a step has no deny",
        bindings: &[(KeyCode::Char('d'), MainKey::DenyAsk)],
    },
    HelpEntry {
        group: HelpGroup::Confirm,
        keys: "v, y",
        description: "start hydra-puppets review on the selected task",
        bindings: &[(KeyCode::Char('v'), MainKey::ReviewAsk)],
    },
    HelpEntry {
        group: HelpGroup::Confirm,
        keys: "m, y",
        description: "stop the selected live seat",
        bindings: &[(KeyCode::Char('m'), MainKey::StopAsk)],
    },
    HelpEntry {
        group: HelpGroup::Views,
        keys: "k",
        description: "swap the four lanes by state for one lane per agent",
        bindings: &[(KeyCode::Char('k'), MainKey::ToggleLanes)],
    },
    HelpEntry {
        group: HelpGroup::Views,
        keys: "r",
        description: "swap tool calls for the raw last twenty record lines",
        bindings: &[(KeyCode::Char('r'), MainKey::ToggleRawTail)],
    },
    HelpEntry {
        group: HelpGroup::Views,
        keys: "i",
        description: "reveal the whole input of the selected call or line",
        bindings: &[(KeyCode::Char('i'), MainKey::ToggleReveal)],
    },
    HelpEntry {
        group: HelpGroup::Views,
        keys: "p",
        description: "policy pane: the set sha and live runs; e opens the file in $EDITOR",
        bindings: &[(KeyCode::Char('p'), MainKey::Policy)],
    },
    HelpEntry {
        group: HelpGroup::Quit,
        keys: "q",
        description: "quit",
        bindings: &[(KeyCode::Char('q'), MainKey::Quit)],
    },
    HelpEntry {
        group: HelpGroup::Help,
        keys: "?",
        description: "open or close this help",
        bindings: &[(KeyCode::Char('?'), MainKey::ToggleHelp)],
    },
    HelpEntry {
        group: HelpGroup::PolicyPane,
        keys: "Up, Down",
        description: "select a policy file",
        bindings: &[],
    },
    HelpEntry {
        group: HelpGroup::PolicyPane,
        keys: "PageUp, PageDown",
        description: "scroll live runs in the policy pane",
        bindings: &[],
    },
    HelpEntry {
        group: HelpGroup::PolicyPane,
        keys: "e",
        description: "open the selected policy file in $EDITOR",
        bindings: &[],
    },
    HelpEntry {
        group: HelpGroup::PolicyPane,
        keys: "p, Esc",
        description: "close the policy pane",
        bindings: &[],
    },
    HelpEntry {
        group: HelpGroup::Session,
        keys: "F2",
        description: "move the keys between the chat and the cockpit",
        bindings: &[],
    },
    HelpEntry {
        group: HelpGroup::Session,
        keys: "Ctrl-q",
        description: "quit once the chat has exited; while it runs, ask then y hangs it up",
        bindings: &[],
    },
    HelpEntry {
        group: HelpGroup::Session,
        keys: "(with keys on the chat)",
        description: "every other key goes to the chat",
        bindings: &[],
    },
];

static MAIN_LOOKUP: LazyLock<HashMap<KeyCode, MainKey>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    for entry in HELP {
        for &(code, main) in entry.bindings {
            map.insert(code, main);
        }
    }
    map
});

/// The main dispatch for `code`, if the help table lists one.
pub fn main_key(code: KeyCode) -> Option<MainKey> {
    MAIN_LOOKUP.get(&code).copied()
}

/// Rows for the help overlay: main groups, then policy-pane keys, then session
/// lines when `session` is true.
pub fn help_rows(session: bool) -> Vec<(&'static str, &'static str)> {
    let mut out: Vec<(&'static str, &'static str)> = HELP
        .iter()
        .filter(|e| e.group != HelpGroup::PolicyPane && e.group != HelpGroup::Session)
        .map(|e| (e.keys, e.description))
        .collect();
    out.extend(
        HELP.iter()
            .filter(|e| e.group == HelpGroup::PolicyPane)
            .map(|e| (e.keys, e.description)),
    );
    if session {
        out.extend(
            HELP.iter()
                .filter(|e| e.group == HelpGroup::Session)
                .map(|e| (e.keys, e.description)),
        );
    }
    out
}

/// Every `keys` label shown in the main help table, for buffer tests.
pub fn help_key_labels() -> Vec<&'static str> {
    HELP.iter()
        .filter(|e| e.group != HelpGroup::Session)
        .map(|e| e.keys)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_MAIN_KEYS: &[MainKey] = &[
        MainKey::Quit,
        MainKey::Policy,
        MainKey::FocusForward,
        MainKey::FocusBackward,
        MainKey::MoveUp,
        MainKey::MoveDown,
        MainKey::Open,
        MainKey::CloseOpened,
        MainKey::ApproveAsk,
        MainKey::DenyAsk,
        MainKey::ReviewAsk,
        MainKey::StopAsk,
        MainKey::ToggleLanes,
        MainKey::ToggleRawTail,
        MainKey::ToggleReveal,
        MainKey::ToggleHelp,
    ];

    #[test]
    fn every_main_key_has_a_help_line() {
        for mk in ALL_MAIN_KEYS {
            assert!(
                HELP.iter()
                    .any(|e| e.bindings.iter().any(|(_, m)| *m == *mk)),
                "no help row for {mk:?}"
            );
        }
    }

    #[test]
    fn every_binding_code_is_unique() {
        let mut seen = HashMap::new();
        for entry in HELP {
            for &(code, mk) in entry.bindings {
                if let Some(prev) = seen.insert(code, mk) {
                    assert_eq!(prev, mk, "duplicate main mapping for {code:?}");
                }
            }
        }
    }

    #[test]
    fn main_lookup_matches_help() {
        for entry in HELP {
            for &(code, mk) in entry.bindings {
                assert_eq!(main_key(code), Some(mk), "{code:?}");
            }
        }
    }
}
