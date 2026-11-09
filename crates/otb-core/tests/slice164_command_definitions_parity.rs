//! Slice 164 parity: bot command definitions active-filter gates.
//!
//! Mirrors the pure TS contract in `src/bot/commands/definitions.ts`
//! (`COMMAND_DEFINITIONS`, `ACTIVE_COMMAND_DEFINITIONS` filter,
//! `getLocalizedBotCommands` / `BOT_COMMANDS` order,
//! `BUILT_IN_COMMAND_NAMES`), asserted by
//! `tests/bot/commands/definitions.test.ts`:
//! - on V2 `/reload` sits right after `/opencode_stop`, the localized list
//!   matches the command order, built-ins contain `reload` and `goal`
//! - on V1 `/reload` and `/goal` are absent everywhere
//!
//! Already covered elsewhere and NOT re-tested here:
//! - per-command handlers / routers (their own gates owners)
//! - locale strings for `cmd.description.*` (i18n owners)
//! - `t(descriptionKey)` resolution and `setMyCommands` delivery (callers)

use otb_core::command_definitions_gates::*;

// ---- COMMAND_DEFINITIONS order (2) ----

#[test]
fn command_definitions_canonical_order_parity() {
    let names: Vec<&str> = COMMAND_DEFINITIONS
        .iter()
        .map(|definition| definition.command)
        .collect();
    assert_eq!(
        names,
        vec![
            "status",
            "new",
            "abort",
            "detach",
            "sessions",
            "recent",
            "messages",
            "settings",
            "projects",
            "worktree",
            "task",
            "tasklist",
            "rename",
            "commands",
            "skills",
            "mcps",
            "opencode_start",
            "opencode_stop",
            "reload",
            "goal",
            "open",
            "ls",
            "help",
        ]
    );
}

#[test]
fn command_definitions_server_version_gates_parity() {
    for definition in COMMAND_DEFINITIONS {
        match definition.command {
            "reload" | "goal" => assert_eq!(
                definition.server_version,
                Some("v2"),
                "{} should be v2-gated",
                definition.command
            ),
            _ => assert_eq!(
                definition.server_version, None,
                "{} should be ungated",
                definition.command
            ),
        }
    }
}

// ---- is_command_active (1) ----

#[test]
fn command_active_filter_parity() {
    assert!(is_command_active(None, "v1"));
    assert!(is_command_active(None, "v2"));
    assert!(is_command_active(Some("v2"), "v2"));
    assert!(!is_command_active(Some("v2"), "v1"));
}

// ---- V2 active list (3) ----

#[test]
fn command_active_v2_contains_reload_and_goal_parity() {
    let names = active_command_names("v2");
    assert!(names.contains(&"reload"));
    assert!(names.contains(&"goal"));
    assert_eq!(names.len(), COMMAND_DEFINITIONS.len());
}

#[test]
fn command_v2_reload_follows_opencode_stop_parity() {
    let names = active_command_names("v2");
    let stop = names
        .iter()
        .position(|name| *name == "opencode_stop")
        .unwrap();
    assert_eq!(names[stop + 1], "reload");
    assert!(reload_follows_opencode_stop("v2"));
}

#[test]
fn command_v2_localized_order_matches_active_parity() {
    // TS: `getLocalizedBotCommands().map(({ command }) => command)` equals
    // `BOT_COMMANDS.map(({ command }) => command)`; both follow the active
    // definitions order (description localization stays with the caller).
    let names = active_command_names("v2");
    let canonical: Vec<&str> = COMMAND_DEFINITIONS
        .iter()
        .map(|definition| definition.command)
        .collect();
    assert_eq!(names, canonical);
}

// ---- V2 built-ins (1) ----

#[test]
fn command_built_in_v2_starts_with_start_parity() {
    let built_in = built_in_command_names("v2");
    assert_eq!(built_in[0], "start");
    assert_eq!(built_in.len(), COMMAND_DEFINITIONS.len() + 1);
    assert!(built_in.contains(&"reload"));
    assert!(built_in.contains(&"goal"));
    assert_eq!(&built_in[1..], active_command_names("v2").as_slice());
}

// ---- V1 active list (2) ----

#[test]
fn command_active_v1_omits_reload_and_goal_parity() {
    let names = active_command_names("v1");
    assert!(!names.contains(&"reload"));
    assert!(!names.contains(&"goal"));
    assert_eq!(names.len(), COMMAND_DEFINITIONS.len() - 2);
    assert_eq!(
        names,
        vec![
            "status",
            "new",
            "abort",
            "detach",
            "sessions",
            "recent",
            "messages",
            "settings",
            "projects",
            "worktree",
            "task",
            "tasklist",
            "rename",
            "commands",
            "skills",
            "mcps",
            "opencode_start",
            "opencode_stop",
            "open",
            "ls",
            "help",
        ]
    );
}

#[test]
fn command_built_in_v1_omits_reload_and_goal_parity() {
    let built_in = built_in_command_names("v1");
    assert_eq!(built_in[0], "start");
    assert!(!built_in.contains(&"reload"));
    assert!(!built_in.contains(&"goal"));
    assert_eq!(&built_in[1..], active_command_names("v1").as_slice());
}

#[test]
fn command_reload_does_not_follow_on_v1_parity() {
    assert!(!reload_follows_opencode_stop("v1"));
}

// ---- index helper (1) ----

#[test]
fn command_active_index_parity() {
    assert_eq!(active_command_index("status", "v2"), Some(0));
    let stop = active_command_index("opencode_stop", "v2").unwrap();
    assert_eq!(active_command_index("reload", "v2"), Some(stop + 1));
    assert_eq!(active_command_index("goal", "v2"), Some(stop + 2));
    assert_eq!(active_command_index("reload", "v1"), None);
    assert_eq!(active_command_index("goal", "v1"), None);
    assert_eq!(active_command_index("unknown", "v2"), None);
}
