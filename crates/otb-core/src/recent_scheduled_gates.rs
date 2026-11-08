//! Recent-selection + scheduled-task menu pure gates (slice 55, GREEN).
//!
//! Rust port of the pure remainder in:
//! - `src/bot/menus/recent-selection-menu.ts` (`RECENT_CALLBACK_PREFIX`,
//!   the `RecentStatus` glyph map, `shortenProject` 40-char `…` cut with
//!   first/last segment rules, the `projectLabels` collision
//!   disambiguation, the `projectNames` suffix-depth disambiguation,
//!   `buildRecentMenu`'s `"${glyph} [${project}] "` button label with the
//!   `…` title cut to the 64-char budget, `buildRecentCallback`
//!   (`recent:<index>`))
//! - `src/bot/callbacks/recent-callback-handler.ts` (the
//!   `parseRecentIndexCallback` gate: `/^\d+$/` digits-only index, `-1`
//!   otherwise)
//! - `src/bot/menus/scheduled-task-menu.ts` (`TASK_*` / `TASKLIST_*`
//!   callback constants, `truncateText` 64-char `...` cut with `trimEnd`,
//!   `formatTaskButtonLabel` (`"💤 "` for disabled cron, `"⏰ "`
//!   otherwise, `"[<badge>] "` prefix, whitespace-collapsed prompt),
//!   `buildTaskListKeyboard`'s `tasklist:open:<id>` rows,
//!   `buildTaskDetailsKeyboard`'s `tasklist:toggle:<on|off>:<id>` and
//!   `tasklist:delete:<id>` rows)
//! - `src/bot/callbacks/scheduled-task-callback-handler.ts` (the
//!   `handleTaskListCallback` parse gates: `tasklist:open:<id>` /
//!   `tasklist:delete:<id>` id slices, the `tasklist:toggle:<action>:<id>`
//!   two-segment destructure with the `on`/`off` + non-empty-id gate, the
//!   exact `tasklist:cancel` match)
//!
//! Step 2 (GREEN): pure sync logic ported from the TS sources above.
//! Verified by `crates/otb-core/tests/slice55_recent_scheduled_parity.rs`.
//!
//! No grammy/Telegram I/O, no timers, no `t(...)`/store/config lookups live
//! here — only callback prefixes, label truncation, project-name
//! disambiguation, and callback parses. Callers resolve locales, session
//! rows, task badges, and keyboards.
//!
//! DEVIATIONS from TS:
//! 1. `String.length` / `slice` / spread count UTF-16 code units in TS. The
//!    port works on `char`s; parity fixtures use ASCII-only inputs where the
//!    two coincide, except the `💤`/`⏰` state prefixes (see 2).
//! 2. `"💤 "` is 3 UTF-16 units (surrogate pair) but 2 `char`s; `"⏰ "` is 2
//!    in both. Truncation fixtures stick to `⏰` (or short text) so the
//!    64-char budget coincides; callers keep the same `truncateText`.
//! 3. `formatSessionTitle` (owned by `voice_cron_gates` / `extra_formatters`)
//!    and `formatTaskListBadge` (timezone/cron rendering) have no pure
//!    portable counterpart here. The port takes the preformatted
//!    `display_title` / `badge` as explicit arguments and still composes the
//!    `"${glyph} [${project}] "` / `"${state}[${badge}] "` prefixes and the
//!    `…` / `...` cuts itself.
//! 4. `getProjectFolderName` (owned by `command_session_gates`) is reused
//!    conceptually by `project_names`; the folder rule (last `/`/`\`
//!    segment, trailing separators tolerated, input unchanged when nothing
//!    but separators remains) is restated here so the gate stays
//!    self-contained.
//! 5. The toggle parse mirrors the TS destructure (`split(":")` taking the
//!    first two segments): extra `:` segments are ignored, so
//!    `"tasklist:toggle:on:a:b"` reads as action `on`, id `"a"`.
//! 6. `open`/`delete` parses return the raw id slice (possibly empty); the
//!    handler's store lookup rejects unknown ids. `None` means wrong prefix
//!    only.
//! 7. `project_labels` folder excerpt: current TS cuts the shortened last
//!    segment to 12 chars with a flat `slice(0, 12)`; the parity tests
//!    require the 12-char `…` shorten (`A×6…B×5` style) instead, so the
//!    port applies the same `shorten` gate with `max = 12` to the folder.
//! 8. `parse_recent_index_callback` returns `None` when a digits-only
//!    payload overflows `u64`. TS would produce a rounded `Number` that then
//!    misses the session-id lookup; both readings end inactive.
//! 9. Two RED-test literals were off by one versus both the TS contract and
//!    their own `…count() == 64` assertion (a 63-char literal cannot satisfy
//!    a 64-char count): the recent-title cut is 57 `t`s and the task-label
//!    cut is 45 `x`s. The test literals were corrected accordingly.

// ---------------------------------------------------------------------------
// Recent selection menu (`src/bot/menus/recent-selection-menu.ts`)
// ---------------------------------------------------------------------------

/// `recent:<index>` callback prefix.
pub const RECENT_CALLBACK_PREFIX: &str = "recent:";

/// Inline-button label budget shared with the other selection menus.
pub const MAX_RECENT_BUTTON_LABEL_LENGTH: usize = 64;

/// Names longer than this are shortened with `…`
/// (TS: the `shortenProject` 40-char gate).
pub const MAX_SHORT_PROJECT_LENGTH: usize = 40;

/// Status glyph shown before `[project]`
/// (TS: the `GLYPHS` record over `RecentStatus`).
pub fn recent_status_glyph(status: &str) -> Option<&'static str> {
    match status {
        "running" => Some("⏳"),
        "idle" => Some("○"),
        "question" => Some("❓"),
        "permission" => Some("🔐"),
        _ => None,
    }
}

/// `shorten` helper inside TS `shortenProject`: keep `max - tail - 1` head
/// chars + `…` + `tail` chars when the segment exceeds `max`.
fn shorten_segment(segment: &str, max: usize, tail: usize) -> String {
    let chars: Vec<char> = segment.chars().collect();
    if chars.len() <= max {
        return segment.to_string();
    }
    let head_len = max.saturating_sub(tail).saturating_sub(1);
    let head: String = chars.iter().take(head_len).collect();
    let tail_str: String = if tail == 0 {
        String::new()
    } else {
        chars[chars.len() - tail..].iter().collect()
    };
    format!("{head}…{tail_str}")
}

/// Shorten one project display name to 40 chars with `…`, keeping the
/// first/last path segments for multi-segment names
/// (TS: `shortenProject`).
pub fn shorten_project(name: &str) -> String {
    if name.chars().count() <= MAX_SHORT_PROJECT_LENGTH {
        return name.to_string();
    }
    let segments: Vec<&str> = name.split('/').collect();
    if segments.len() == 1 {
        return shorten_segment(
            name,
            MAX_SHORT_PROJECT_LENGTH,
            (MAX_SHORT_PROJECT_LENGTH - 1) / 2,
        );
    }
    let first = shorten_segment(segments[0], 24, 18);
    let last_max = if segments.len() == 2 { 15 } else { 13 };
    let last = shorten_segment(segments[segments.len() - 1], last_max, (last_max - 1) / 2);
    if segments.len() == 2 {
        format!("{first}/{last}")
    } else {
        format!("{first}/…/{last}")
    }
}

/// TS `getProjectFolderName` restated here so the gate stays
/// self-contained: strip trailing `/`/`\`, return the last remaining
/// segment, or the input unchanged when nothing but separators remains.
fn get_project_folder_name(worktree: &str) -> String {
    let normalized = worktree.trim_end_matches(['/', '\\']);
    if normalized.is_empty() {
        return worktree.to_string();
    }
    normalized
        .split(['/', '\\'])
        .filter(|segment| !segment.is_empty())
        .next_back()
        .unwrap_or(normalized)
        .to_string()
}

/// Disambiguate shortened project names: no collision passes through,
/// identical full names pass through, otherwise the first distinguishing
/// parent excerpt (`…<excerpt>…/<folder>`) or the `<short>…<n>` fallback
/// (TS: `projectLabels`).
pub fn project_labels(names: &[String]) -> Vec<String> {
    const FOLDER_MAX: usize = 12;
    let shortened: Vec<String> = names.iter().map(|name| shorten_project(name)).collect();
    names
        .iter()
        .enumerate()
        .map(|(index, name)| {
            let collides = shortened.iter().enumerate().any(|(other_index, other)| {
                other_index != index && *other == shortened[index] && names[other_index] != *name
            });
            if !collides {
                return shortened[index].clone();
            }
            // TS: `name.slice(0, name.lastIndexOf("/"))`; with no `/` the
            // end is `-1`, i.e. everything but the last char.
            let parent: String = match name.rfind('/') {
                Some(byte_idx) => name[..byte_idx].to_string(),
                None => {
                    let count = name.chars().count();
                    name.chars().take(count.saturating_sub(1)).collect()
                }
            };
            let others: Vec<&String> = names.iter().filter(|other| *other != name).collect();
            let last_segment = name.rsplit('/').next().unwrap_or(name);
            let folder = shorten_segment(last_segment, FOLDER_MAX, (FOLDER_MAX - 1) / 2);
            let parent_chars: Vec<char> = parent.chars().collect();
            // First pass: excerpt anchored at the first difference, widened
            // 8 chars to the left (TS: `difference - 8`, 22-char window).
            for other in &others {
                let other_chars: Vec<char> = other.chars().collect();
                let mut difference = 0;
                while difference < parent_chars.len()
                    && difference < other_chars.len()
                    && parent_chars[difference] == other_chars[difference]
                {
                    difference += 1;
                }
                let start = difference.saturating_sub(8);
                let excerpt: String = parent_chars.iter().skip(start).take(22).collect();
                if !excerpt.is_empty() && others.iter().all(|candidate| !candidate.contains(&excerpt)) {
                    return format!("…{excerpt}…/{folder}");
                }
            }
            // Second pass: widest distinguishing substring, right to left.
            let parent_len = parent_chars.len();
            for size in (1..=parent_len.min(22)).rev() {
                for start in (0..=(parent_len - size)).rev() {
                    let excerpt: String =
                        parent_chars.iter().skip(start).take(size).collect();
                    if others.iter().all(|other| !other.contains(&excerpt)) {
                        return format!("…{excerpt}…/{folder}");
                    }
                }
            }
            let head: String = shortened[index].chars().take(34).collect();
            format!("{head}…{}", index + 1)
        })
        .collect()
}

/// Shortest distinguishing trailing path for each directory: the leaf name
/// unless another directory shares the suffix, then deeper suffixes
/// (TS: `projectNames` over `getProjectFolderName`).
pub fn project_names(directories: &[String]) -> Vec<String> {
    // TS dedups directories via a `Set` first; keep that order here.
    let mut unique: Vec<&str> = Vec::new();
    for directory in directories {
        if !unique.contains(&directory.as_str()) {
            unique.push(directory);
        }
    }
    let parts: Vec<Vec<&str>> = unique
        .iter()
        .map(|directory| {
            directory
                .split(['/', '\\'])
                .filter(|segment| !segment.is_empty())
                .collect()
        })
        .collect();
    let mut names: Vec<String> = Vec::with_capacity(unique.len());
    for (i, segments) in parts.iter().enumerate() {
        let mut depth = 1usize;
        while depth < segments.len()
            && parts.iter().enumerate().any(|(j, other)| {
                j != i
                    && other.len() >= depth
                    && other[other.len() - depth..] == segments[segments.len() - depth..]
            })
        {
            depth += 1;
        }
        if depth == 1 {
            names.push(get_project_folder_name(unique[i]));
        } else {
            names.push(segments[segments.len() - depth..].join("/"));
        }
    }
    directories
        .iter()
        .map(|directory| {
            let idx = unique
                .iter()
                .position(|entry| *entry == directory.as_str())
                .expect("directory comes from the input slice");
            names[idx].clone()
        })
        .collect()
}

/// `"${glyph} [${project}] ${title}"` with the title cut to the 64-char
/// budget via a trailing `…` (TS: the `buildRecentMenu` row label over the
/// preformatted `formatSessionTitle` output).
pub fn build_recent_button_label(glyph: &str, project: &str, display_title: &str) -> String {
    let prefix = format!("{glyph} [{project}] ");
    let available =
        MAX_RECENT_BUTTON_LABEL_LENGTH.saturating_sub(prefix.chars().count());
    if display_title.chars().count() > available {
        let head: String = display_title
            .chars()
            .take(available.saturating_sub(1))
            .collect();
        format!("{prefix}{head}…")
    } else {
        format!("{prefix}{display_title}")
    }
}

/// `recent:<index>` (TS: the `buildRecentMenu` callback data).
pub fn build_recent_callback(index: usize) -> String {
    format!("{RECENT_CALLBACK_PREFIX}{index}")
}

/// Index of a `recent:<n>` callback; `None` for a wrong prefix or a
/// non-digits-only payload (TS: the `handleRecentSelect` `/^\d+$/` gate,
/// where anything else reads as inactive `-1`).
pub fn parse_recent_index_callback(data: &str) -> Option<u64> {
    let rest = data.strip_prefix(RECENT_CALLBACK_PREFIX)?;
    if rest.is_empty() || !rest.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    rest.parse::<u64>().ok()
}

// ---------------------------------------------------------------------------
// Scheduled task menu (`src/bot/menus/scheduled-task-menu.ts` +
// `src/bot/callbacks/scheduled-task-callback-handler.ts`)
// ---------------------------------------------------------------------------

/// Retry-schedule button callback.
pub const TASK_RETRY_SCHEDULE_CALLBACK: &str = "task:retry-schedule";

/// Cancel button callback.
pub const TASK_CANCEL_CALLBACK: &str = "task:cancel";

/// `tasklist:` callback prefix.
pub const TASKLIST_CALLBACK_PREFIX: &str = "tasklist:";

/// `tasklist:open:<id>` callback prefix.
pub const TASKLIST_OPEN_PREFIX: &str = "tasklist:open:";

/// `tasklist:delete:<id>` callback prefix.
pub const TASKLIST_DELETE_PREFIX: &str = "tasklist:delete:";

/// `tasklist:toggle:<on|off>:<id>` callback prefix.
pub const TASKLIST_TOGGLE_PREFIX: &str = "tasklist:toggle:";

/// Task-list cancel callback.
pub const TASKLIST_CANCEL_CALLBACK: &str = "tasklist:cancel";

/// Inline-button label budget for task rows.
pub const MAX_TASK_BUTTON_LABEL_LENGTH: usize = 64;

/// Over-budget cut with `...` after a `trimEnd` (TS: `truncateText`).
pub fn truncate_task_text(text: &str, max_length: usize) -> String {
    if text.chars().count() <= max_length {
        return text.to_string();
    }
    let head: String = text.chars().take(max_length.saturating_sub(3)).collect();
    format!("{}...", head.trim_end())
}

/// Collapse every whitespace run to one space and trim (TS: the
/// `task.prompt.replace(/\s+/g, " ").trim()` normalization inside
/// `formatTaskButtonLabel`).
pub fn normalize_task_prompt(prompt: &str) -> String {
    prompt.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `"💤 "` for a disabled cron task, `"⏰ "` otherwise (TS: the
/// `formatTaskButtonLabel` state prefix: `task.kind === "cron" &&
/// task.enabled === false`).
pub fn task_state_prefix(is_cron: bool, enabled: bool) -> &'static str {
    if is_cron && !enabled {
        "💤 "
    } else {
        "⏰ "
    }
}

/// `"${state}[${badge}] ${prompt}"` cut to the 64-char budget (TS:
/// `formatTaskButtonLabel`; `badge` is the preformatted
/// `formatTaskListBadge` output).
pub fn format_task_button_label(is_cron: bool, enabled: bool, badge: &str, prompt: &str) -> String {
    let state = task_state_prefix(is_cron, enabled);
    let normalized = normalize_task_prompt(prompt);
    truncate_task_text(
        &format!("{state}[{badge}] {normalized}"),
        MAX_TASK_BUTTON_LABEL_LENGTH,
    )
}

/// `tasklist:open:<id>` (TS: the `buildTaskListKeyboard` row data).
pub fn build_tasklist_open_callback(task_id: &str) -> String {
    format!("{TASKLIST_OPEN_PREFIX}{task_id}")
}

/// `tasklist:delete:<id>` (TS: the `buildTaskDetailsKeyboard` delete data).
pub fn build_tasklist_delete_callback(task_id: &str) -> String {
    format!("{TASKLIST_DELETE_PREFIX}{task_id}")
}

/// Enable/disable toggle direction (TS: the `"on"` / `"off"` action in the
/// `buildTaskDetailsKeyboard` toggle data).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToggleAction {
    On,
    Off,
}

/// `tasklist:toggle:<on|off>:<id>` (TS: the `buildTaskDetailsKeyboard`
/// toggle data; `enable` picks `"on"` vs `"off"`).
pub fn build_tasklist_toggle_callback(enable: bool, task_id: &str) -> String {
    let action = if enable { "on" } else { "off" };
    format!("{TASKLIST_TOGGLE_PREFIX}{action}:{task_id}")
}

/// Parsed `tasklist:toggle:<on|off>:<id>` callback (TS: the
/// `handleTaskListCallback` toggle destructure).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TasklistToggle {
    pub action: ToggleAction,
    pub id: String,
}

/// Id behind a `tasklist:open:<id>` callback; `None` for a wrong prefix
/// (TS: the `handleTaskListCallback` open slice; the store lookup rejects
/// unknown ids).
pub fn parse_tasklist_open_callback(data: &str) -> Option<String> {
    data.strip_prefix(TASKLIST_OPEN_PREFIX)
        .map(|id| id.to_string())
}

/// Id behind a `tasklist:delete:<id>` callback; `None` for a wrong prefix
/// (TS: the `handleTaskListCallback` delete slice).
pub fn parse_tasklist_delete_callback(data: &str) -> Option<String> {
    data.strip_prefix(TASKLIST_DELETE_PREFIX)
        .map(|id| id.to_string())
}

/// Action + id behind a `tasklist:toggle:<on|off>:<id>` callback; `None`
/// for a wrong prefix, a non-`on`/`off` action, or an empty id (TS: the
/// `handleTaskListCallback` toggle gate).
pub fn parse_tasklist_toggle_callback(data: &str) -> Option<TasklistToggle> {
    let rest = data.strip_prefix(TASKLIST_TOGGLE_PREFIX)?;
    let mut segments = rest.split(':');
    let action = match segments.next()? {
        "on" => ToggleAction::On,
        "off" => ToggleAction::Off,
        _ => return None,
    };
    let id = segments.next()?;
    if id.is_empty() {
        return None;
    }
    Some(TasklistToggle {
        action,
        id: id.to_string(),
    })
}

/// Whether the data is exactly the task-list cancel callback (TS: the
/// `data === TASKLIST_CANCEL_CALLBACK` check in `handleTaskListCallback`).
pub fn is_tasklist_cancel_callback(data: &str) -> bool {
    data == TASKLIST_CANCEL_CALLBACK
}
