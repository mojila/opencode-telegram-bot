//! Bot menu registry: the 19 inline-menu ids (T7 structure only).
//!
//! Node parity (`src/bot/menus/`, 19 `*-menu.ts` files): each id is the file
//! basename without the trailing `-menu` (e.g. `settings-menu.ts` ->
//! `settings`), kept in alphabetical order mirroring the directory listing.
//!
//! Pure lookups only. No Telegram API calls, no polling, no DB writes.

/// Canonical menu ids, one per `src/bot/menus/*-menu.ts` file.
pub const MENU_IDS: &[&str] = &[
    "agent-selection",
    "command-catalog",
    "context-control",
    "file-browser",
    "inline",
    "mcp-catalog",
    "message-history",
    "model-selection",
    "permission",
    "project-selection",
    "question",
    "recent-selection",
    "rename",
    "scheduled-task",
    "session-selection",
    "settings",
    "skills-catalog",
    "variant-selection",
    "worktree-selection",
];

/// Number of registered menus (Node parity: 19).
pub fn menu_count() -> usize {
    MENU_IDS.len()
}

/// All menu ids in canonical order.
pub fn all_menus() -> Vec<&'static str> {
    MENU_IDS.to_vec()
}

/// Pure lookup: true for a known menu id (exact match).
pub fn is_known_menu(id: &str) -> bool {
    MENU_IDS.contains(&id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_19_menus() {
        assert_eq!(menu_count(), 19);
        assert_eq!(all_menus().len(), 19);
    }

    #[test]
    fn lookup_finds_known_menus() {
        assert!(is_known_menu("settings"));
        assert!(is_known_menu("inline"));
        assert!(is_known_menu("session-selection"));
        assert!(is_known_menu("worktree-selection"));
        assert!(!is_known_menu("settings-menu"));
        assert!(!is_known_menu("bogus"));
        assert!(!is_known_menu(""));
    }
}
