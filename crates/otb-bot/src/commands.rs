//! Bot command registry: the 23 Telegram commands (T7 structure only).
//!
//! Node parity (`src/bot/commands/definitions.ts`, `COMMAND_DEFINITIONS`):
//! same 23 names in the same canonical display order. The `reload`/`goal`
//! V2 gate (`ACTIVE_COMMAND_DEFINITIONS`) already lives in
//! `otb_core::command_definitions_gates`; this module only lists the
//! canonical names for reporting.
//!
//! Pure lookups only. No Telegram API calls, no polling, no DB writes.
//! The live `/status` template (`render_status`, JS
//! `src/bot/commands/status-command.ts` parity) is pure too: callers build
//! a [`StatusSnapshot`] from the health probe + placeholders and render it.

/// Canonical command names in Telegram display order
/// (Node: `COMMAND_DEFINITIONS` order).
pub const COMMAND_NAMES: &[&str] = &[
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
];

/// Number of registered commands (Node parity: 23).
pub fn command_count() -> usize {
    COMMAND_NAMES.len()
}

/// All command names in canonical order.
pub fn all_commands() -> Vec<&'static str> {
    COMMAND_NAMES.to_vec()
}

/// Pure lookup: true for a known command name, with or without a leading `/`.
pub fn is_known_command(name: &str) -> bool {
    let bare = name.strip_prefix('/').unwrap_or(name);
    COMMAND_NAMES.contains(&bare)
}

/// Dispatch a command name to its scaffold ack (pure; no network).
/// Accepts `status`, `/status`, `/status@bot`, any case; unknown -> `None`.
/// The ack is a static-shape English string so the `status` CLI and unit
/// tests can verify the wiring without Telegram or OpenCode I/O.
/// Stays scaffold for all 23 commands: the live `/status` path renders a
/// [`StatusSnapshot`] via [`render_status`] instead of calling this.
pub fn dispatch(command: &str) -> Option<String> {
    let token = command.trim().strip_prefix('/').unwrap_or(command.trim());
    let token = token.split_whitespace().next().unwrap_or("");
    let bare = token.split('@').next().unwrap_or("").to_lowercase();
    if is_known_command(&bare) {
        Some(format!("ok:{bare} (scaffold-wired)"))
    } else {
        None
    }
}

/// Live `/status` snapshot (JS `status-command.ts` parity).
///
/// `model_display` already carries the `🧠` prefix (JS `modelDisplay`
/// parity); `session_title` is pre-formatted (JS `formatSessionTitle`
/// parity). `agent_display` / `project_display` / `worktree_path` /
/// `session_title` are `None` until their live fetches land (T15 follow-up);
/// `None` (or blank) renders the `*_not_selected` + hint fallbacks.
#[derive(Debug, Clone)]
pub struct StatusSnapshot {
    pub bot_version: String,
    pub healthy: bool,
    pub opencode_version: Option<String>,
    pub agent_display: Option<String>,
    pub model_display: String,
    pub project_display: Option<String>,
    pub worktree_path: Option<String>,
    pub session_title: Option<String>,
}

/// Substitute one `{name}` placeholder in an i18n template (pure).
fn fill(template: &str, name: &str, value: &str) -> String {
    template.replace(&format!("{{{name}}}"), value)
}

/// Render the `/status` reply (JS `status-command.ts` template parity).
/// `locale` selects the dictionary with `en` fallback; `None` means `en`.
///
/// Healthy: `header_running` + bot version + optional OpenCode version +
/// mode + model (with `🧠` prefix) + project/worktree or not-selected+hint +
/// session or not-selected+hint. Unhealthy: `header_unavailable` + bot
/// version + `unavailable_hint`.
pub fn render_status(snapshot: &StatusSnapshot, locale: Option<&str>) -> String {
    let locale = locale.unwrap_or(crate::locale::FALLBACK_LOCALE);
    let t = |key: &str| crate::locale::translate(locale, key);
    if !snapshot.healthy {
        return format!(
            "{}\n{}\n\n{}",
            t("status.header_unavailable"),
            fill(
                &t("status.line.bot_version"),
                "version",
                &snapshot.bot_version
            ),
            t("status.unavailable_hint"),
        );
    }
    let mut message = format!("{}\n\n", t("status.header_running"));
    message.push_str(&format!(
        "{}\n",
        fill(&t("status.line.bot_version"), "version", &snapshot.bot_version)
    ));
    if let Some(version) = snapshot
        .opencode_version
        .as_deref()
        .filter(|version| !version.is_empty())
    {
        message.push_str(&format!(
            "{}\n",
            fill(&t("status.line.version"), "version", version)
        ));
    }
    let agent = snapshot
        .agent_display
        .clone()
        .filter(|agent| !agent.is_empty())
        .unwrap_or_else(|| t("status.agent_not_set"));
    message.push_str(&format!(
        "\n{}\n",
        fill(&t("status.line.mode"), "mode", &agent)
    ));
    message.push_str(&format!(
        "{}\n",
        fill(&t("status.line.model"), "model", &snapshot.model_display)
    ));
    match snapshot
        .project_display
        .as_deref()
        .filter(|project| !project.is_empty())
    {
        Some(project) => {
            message.push_str(&format!(
                "\n{}\n",
                fill(&t("status.project_selected"), "project", project)
            ));
            if let Some(worktree) = snapshot
                .worktree_path
                .as_deref()
                .filter(|worktree| !worktree.is_empty())
            {
                message.push_str(&format!(
                    "{}\n",
                    fill(&t("status.worktree_selected"), "worktree", worktree)
                ));
            }
        }
        None => {
            message.push_str(&format!("\n{}\n", t("status.project_not_selected")));
            message.push_str(&t("status.project_hint"));
        }
    }
    match snapshot
        .session_title
        .as_deref()
        .filter(|title| !title.is_empty())
    {
        Some(title) => {
            message.push_str(&format!(
                "\n{}\n",
                fill(&t("status.session_selected"), "title", title)
            ));
        }
        None => {
            message.push_str(&format!("\n{}\n", t("status.session_not_selected")));
            message.push_str(&t("status.session_hint"));
        }
    }
    message
}

/// One session row for the `/sessions` list (JS
/// `SessionListItem` parity in `session-selection-menu.ts`). Only the fields
/// the text reply needs; the Telegram scaffold renders a numbered list
/// instead of the Node inline keyboard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionListItem {
    pub id: String,
    pub title: String,
    pub directory: String,
}

/// Build a [`SessionListItem`] from one raw OpenCode `session.list` entry.
/// Returns `None` when the entry has no usable `id` (pure; testable).
/// `title`/`directory` default to `""` (the title is formatted at render).
pub fn session_list_item_from_value(value: &serde_json::Value) -> Option<SessionListItem> {
    let id = value
        .get("id")?
        .as_str()
        .map(str::trim)
        .filter(|id| !id.is_empty())?;
    Some(SessionListItem {
        id: id.to_string(),
        title: value
            .get("title")
            .and_then(|title| title.as_str())
            .unwrap_or("")
            .to_string(),
        directory: value
            .get("directory")
            .and_then(|dir| dir.as_str())
            .unwrap_or("")
            .to_string(),
    })
}

/// Name a session is shown under (JS `formatSessionTitle` parity in
/// `session-title-formatter.ts`): blank titles fall back to the localized
/// `pinned.default_session_title` (`"new session"` in English).
pub fn format_session_title(title: &str, locale: Option<&str>) -> String {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        crate::locale::translate(
            locale.unwrap_or(crate::locale::FALLBACK_LOCALE),
            "pinned.default_session_title",
        )
    } else {
        trimmed.to_string()
    }
}

/// Render the `/sessions` reply (JS `sessions-command.ts` +
/// `buildSessionSelectionMenuView` text parity; the scaffold renders a
/// numbered text list instead of the Node inline keyboard with search and
/// paging buttons).
/// `locale` selects the dictionary with `en` fallback; `None` means `en`.
/// Empty list renders `sessions.empty`; otherwise `sessions.select` (page 0)
/// or `sessions.select_page` plus one `N. [marker ]title` line per item.
/// `current_id` marks the active session with `sessions.current_marker`.
pub fn render_sessions(
    items: &[SessionListItem],
    current_id: Option<&str>,
    page: usize,
    locale: Option<&str>,
) -> String {
    let locale = locale.unwrap_or(crate::locale::FALLBACK_LOCALE);
    if items.is_empty() {
        return crate::locale::translate(locale, "sessions.empty");
    }
    let header = if page == 0 {
        crate::locale::translate(locale, "sessions.select")
    } else {
        fill(
            &crate::locale::translate(locale, "sessions.select_page"),
            "page",
            &(page + 1).to_string(),
        )
    };
    let marker = crate::locale::translate(locale, "sessions.current_marker");
    let mut message = header;
    for (index, item) in items.iter().enumerate() {
        let is_current = current_id == Some(item.id.as_str());
        let title = format_session_title(&item.title, Some(locale));
        if is_current && !marker.is_empty() {
            message.push_str(&format!("\n{}. {marker} {title}", index + 1));
        } else {
            message.push_str(&format!("\n{}. {title}", index + 1));
        }
    }
    message
}

/// Render the session-select confirmation (JS `sessions.selected` parity in
/// `session-callback-handler.ts`: `✅ Session selected: {title}` with the
/// formatted title). `locale` selects the dictionary with `en` fallback.
pub fn render_session_selected(title: &str, locale: Option<&str>) -> String {
    let locale = locale.unwrap_or(crate::locale::FALLBACK_LOCALE);
    fill(
        &crate::locale::translate(locale, "sessions.selected"),
        "title",
        &format_session_title(title, Some(locale)),
    )
}

/// Render the `/new` success reply (JS `new.created` parity in
/// `new-command.ts`: `✅ New session created: {title}` with the formatted
/// title). `locale` selects the dictionary with `en` fallback.
pub fn render_new_created(title: &str, locale: Option<&str>) -> String {
    let locale = locale.unwrap_or(crate::locale::FALLBACK_LOCALE);
    fill(
        &crate::locale::translate(locale, "new.created"),
        "title",
        &format_session_title(title, Some(locale)),
    )
}

/// Map a canonical command name to its `cmd.description.*` i18n key (Node
/// `definitions.ts` parity: `abort` is described by
/// `cmd.description.stop`; every other command maps to
/// `cmd.description.<name>`).
fn command_description_key(name: &str) -> String {
    let leaf = if name == "abort" { "stop" } else { name };
    format!("cmd.description.{leaf}")
}

/// Render the `/help` reply (JS `formatHelpText` parity in
/// `help-command.ts`): `📖 {help title}` plus one
/// `/command - description` line per known command in canonical order plus
/// the keyboard hint. `locale` selects the dictionary with `en` fallback;
/// `None` means `en`.
///
/// ASSUMPTION: all 23 canonical commands are listed. Node filters
/// `reload`/`goal` out for V1 servers (`ACTIVE_COMMAND_DEFINITIONS`);
/// server-version filtering is a dispatcher follow-up.
pub fn render_help(locale: Option<&str>) -> String {
    let locale = locale.unwrap_or(crate::locale::FALLBACK_LOCALE);
    let t = |key: &str| crate::locale::translate(locale, key);
    let mut message = format!("📖 {}\n\n", t("cmd.description.help"));
    let lines: Vec<String> = COMMAND_NAMES
        .iter()
        .map(|name| format!("/{name} - {}", t(&command_description_key(name))))
        .collect();
    message.push_str(&lines.join("\n"));
    message.push_str(&format!("\n\n{}", t("help.keyboard_hint")));
    message
}

/// Render the `/settings` open-view text (JS `buildSettingsMenuView` text
/// parity in `settings-menu.ts`: `settings.menu.title`). `locale` selects
/// the dictionary with `en` fallback; `None` means `en`.
///
/// The per-setting toggle keyboard needs a dispatcher and stays a follow-up;
/// the scaffold sends the text only.
pub fn render_settings_view(locale: Option<&str>) -> String {
    let locale = locale.unwrap_or(crate::locale::FALLBACK_LOCALE);
    crate::locale::translate(locale, "settings.menu.title")
}

/// One scheduled task row for the `/tasklist` text reply (JS `ScheduledTask`
/// parity in `scheduled-task.ts`, minimal shape the text reply needs: the
/// scaffold renders a numbered text list instead of the Node inline
/// keyboard, mirroring `render_sessions`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduledTaskItem {
    pub id: String,
    pub prompt: String,
    pub kind: String,
    pub enabled: bool,
    pub created_at: String,
    pub next_run_at: Option<String>,
}

/// Build a [`ScheduledTaskItem`] from one raw `scheduledTasks` store entry.
/// Returns `None` when the entry has no usable `id` (pure; testable).
/// `prompt`/`createdAt` default to `""`, `kind` to `"cron"`, and `enabled`
/// to `true` (Node `cloneScheduledTask` backfill parity); a blank
/// `nextRunAt` becomes `None` (sorts last, like `sortTasks` treats it).
pub fn scheduled_task_item_from_value(value: &serde_json::Value) -> Option<ScheduledTaskItem> {
    let id = value
        .get("id")?
        .as_str()
        .map(str::trim)
        .filter(|id| !id.is_empty())?;
    Some(ScheduledTaskItem {
        id: id.to_string(),
        prompt: value
            .get("prompt")
            .and_then(|prompt| prompt.as_str())
            .unwrap_or("")
            .to_string(),
        kind: value
            .get("kind")
            .and_then(|kind| kind.as_str())
            .unwrap_or("cron")
            .to_string(),
        enabled: value
            .get("enabled")
            .and_then(|enabled| enabled.as_bool())
            .unwrap_or(true),
        created_at: value
            .get("createdAt")
            .and_then(|created| created.as_str())
            .unwrap_or("")
            .to_string(),
        next_run_at: value
            .get("nextRunAt")
            .and_then(|next| next.as_str())
            .map(str::trim)
            .filter(|next| !next.is_empty())
            .map(str::to_string),
    })
}

/// Days since the Unix epoch for a civil date (Howard Hinnant's algorithm;
/// pure). Powers [`parse_instant_millis`] without a date dependency.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = year - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn parse_two_digits(text: &str) -> Option<i64> {
    if text.len() == 2 && text.bytes().all(|b| b.is_ascii_digit()) {
        text.parse::<i64>().ok()
    } else {
        None
    }
}

/// Parse an ISO8601 instant (`YYYY-MM-DD[THH:MM[:SS[.frac]][Z|±HH[:][MM]]]`,
/// the shape `computeNextRunAt` stores) to epoch millis (pure; testable).
/// Anything else yields `None` and sorts last in [`sort_scheduled_tasks`].
fn parse_instant_millis(raw: &str) -> Option<i64> {
    let text = raw.trim();
    let (date, time) = match text.find(|c| c == 'T' || c == ' ') {
        Some(index) => (&text[..index], Some(&text[index + 1..])),
        None => (text, None),
    };
    let mut date_parts = date.split('-');
    let year: i64 = date_parts.next()?.parse().ok()?;
    let month: i64 = parse_two_digits(date_parts.next()?)?;
    let day: i64 = parse_two_digits(date_parts.next()?)?;
    if date_parts.next().is_some() || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let mut seconds = days_from_civil(year, month, day) * 86_400;
    let mut millis: i64 = 0;
    let Some(time) = time.filter(|time| !time.is_empty()) else {
        return Some(seconds * 1000);
    };
    // Split the trailing zone (`Z` or `±HH[:][MM]`) off the clock part.
    let (clock, zone) = if let Some(stripped) = time.strip_suffix('Z').or_else(|| time.strip_suffix('z')) {
        (stripped, Some(0))
    } else if let Some(index) = time.rfind(|c| c == '+' || c == '-') {
        if index == 0 {
            return None;
        }
        let (clock, zone) = (&time[..index], &time[index..]);
        let sign = if zone.starts_with('-') { -1 } else { 1 };
        let digits: String = zone[1..].chars().filter(|c| *c != ':').collect();
        let (hours, minutes) = match digits.len() {
            2 => (digits.parse::<i64>().ok()?, 0),
            4 => (digits[..2].parse::<i64>().ok()?, digits[2..].parse::<i64>().ok()?),
            _ => return None,
        };
        if hours > 23 || minutes > 59 {
            return None;
        }
        (clock, Some(sign * (hours * 3600 + minutes * 60)))
    } else {
        (time, None)
    };
    // A zone-less clock reads as UTC (stored values are UTC ISO strings).
    let offset = zone.unwrap_or(0);
    let mut clock_parts = clock.split(':');
    let hour: i64 = parse_two_digits(clock_parts.next()?)?;
    let minute: i64 = parse_two_digits(clock_parts.next()?)?;
    if hour > 23 || minute > 59 {
        return None;
    }
    seconds += hour * 3600 + minute * 60;
    if let Some(rest) = clock_parts.next() {
        let (sec, frac) = match rest.split_once('.') {
            Some((sec, frac)) => (sec, Some(frac)),
            None => (rest, None),
        };
        let second: i64 = parse_two_digits(sec)?;
        if second > 60 {
            return None;
        }
        seconds += second;
        if let Some(frac) = frac {
            if frac.is_empty() || !frac.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            let scale = frac.len().min(3);
            let digits: i64 = frac[..scale].parse().ok()?;
            millis = digits * 10_i64.pow(3 - scale as u32);
        }
    }
    if clock_parts.next().is_some() {
        return None;
    }
    Some((seconds - offset) * 1000 + millis)
}

/// Sort scheduled tasks (JS `sortTasks` parity in `tasklist-command.ts`): by
/// parsed `nextRunAt` ascending with missing/unparseable values last, then
/// by `createdAt` string order (stored ISO strings order chronologically).
///
/// Deviation: JS `Date.parse` failures produce `NaN`, whose sort placement
/// is engine-defined; here they deterministically sort last.
pub fn sort_scheduled_tasks(mut items: Vec<ScheduledTaskItem>) -> Vec<ScheduledTaskItem> {
    items.sort_by(|left, right| {
        let left_next = left.next_run_at.as_deref().and_then(parse_instant_millis);
        let right_next = right.next_run_at.as_deref().and_then(parse_instant_millis);
        match (left_next, right_next) {
            (Some(left_ms), Some(right_ms)) => left_ms
                .cmp(&right_ms)
                .then_with(|| left.created_at.cmp(&right.created_at)),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => left.created_at.cmp(&right.created_at),
        }
    });
    items
}

/// Render the `/tasklist` reply (JS `taskListCommand` text parity: empty
/// list renders `tasklist.empty`; otherwise `tasklist.select` plus one
/// `N. [state] prompt` line per task in sorted order). The state emoji
/// mirrors `formatTaskButtonLabel` (`💤` for a disabled cron task, else
/// `⏰`); the prompt collapses whitespace like the Node label, but the cron
/// badge and 64-char truncation are keyboard-label concerns and stay out of
/// the text reply. `locale` selects the dictionary with `en` fallback.
pub fn render_tasklist(items: &[ScheduledTaskItem], locale: Option<&str>) -> String {
    let locale = locale.unwrap_or(crate::locale::FALLBACK_LOCALE);
    if items.is_empty() {
        return crate::locale::translate(locale, "tasklist.empty");
    }
    let mut message = crate::locale::translate(locale, "tasklist.select");
    for (index, item) in items.iter().enumerate() {
        let state = if item.kind == "cron" && !item.enabled {
            "💤"
        } else {
            "⏰"
        };
        let prompt = item
            .prompt
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        message.push_str(&format!("\n{}. {state} {prompt}", index + 1));
    }
    message
}

/// One project row for the `/projects` list (JS `ProjectInfo` parity in
/// `src/app/types/project.ts`: only the fields the text reply needs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectListItem {
    pub id: String,
    pub worktree: String,
    pub name: String,
    pub last_updated: i64,
}

/// Build a [`ProjectListItem`] from one raw `project.list` entry.
/// Returns `None` when `id`/`worktree` are missing or blank (pure;
/// testable). `name` falls back to the worktree (JS `name || worktree`
/// parity); `time.updated` (else `0`) orders the list like Node's
/// `lastUpdated` sort.
pub fn project_list_item_from_value(value: &serde_json::Value) -> Option<ProjectListItem> {
    let id = value
        .get("id")?
        .as_str()
        .map(str::trim)
        .filter(|id| !id.is_empty())?;
    let worktree = value
        .get("worktree")?
        .as_str()
        .map(str::trim)
        .filter(|worktree| !worktree.is_empty())?;
    let name = value
        .get("name")
        .and_then(|name| name.as_str())
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or(worktree);
    let last_updated = value
        .get("time")
        .and_then(|time| time.get("updated"))
        .and_then(|updated| updated.as_i64())
        .unwrap_or(0);
    Some(ProjectListItem {
        id: id.to_string(),
        worktree: worktree.to_string(),
        name: name.to_string(),
        last_updated,
    })
}

/// Folder name for a worktree path (JS `getProjectFolderName` parity in
/// `project-selection-menu.ts`): trailing separators are trimmed and the
/// last segment wins; a blank result falls back to the raw worktree.
pub fn get_project_folder_name(worktree: &str) -> String {
    let trimmed = worktree.trim_end_matches(['/', '\\']);
    if trimmed.is_empty() {
        return worktree.to_string();
    }
    trimmed
        .rsplit(['/', '\\'])
        .find(|segment| !segment.is_empty())
        .unwrap_or(trimmed)
        .to_string()
}

/// Render the `/projects` reply (JS `projects-command.ts` +
/// `buildProjectsMenuView` text parity; the scaffold renders a numbered
/// text list instead of the Node inline keyboard).
/// `current_worktree` marks the active row with a `✅` prefix like the Node
/// button label; `current_display` is the `select_with_current` project name
/// (usually the name or worktree). Empty list renders `projects.empty`;
/// otherwise `select`/`select_with_current` plus an optional `page_indicator`
/// plus one `N. [✅ ]folder [worktree]` line per item. `locale` selects the
/// dictionary with `en` fallback; `None` means `en`.
pub fn render_projects(
    items: &[ProjectListItem],
    current_worktree: Option<&str>,
    current_display: Option<&str>,
    page: usize,
    total_pages: usize,
    locale: Option<&str>,
) -> String {
    let locale = locale.unwrap_or(crate::locale::FALLBACK_LOCALE);
    if items.is_empty() {
        return crate::locale::translate(locale, "projects.empty");
    }
    let mut message = match current_display
        .map(str::trim)
        .filter(|display| !display.is_empty())
    {
        Some(display) => fill(
            &crate::locale::translate(locale, "projects.select_with_current"),
            "project",
            display,
        ),
        None => crate::locale::translate(locale, "projects.select"),
    };
    if total_pages > 1 {
        let indicator = crate::locale::translate(locale, "projects.page_indicator");
        let indicator = fill(&indicator, "current", &(page + 1).to_string());
        let indicator = fill(&indicator, "total", &total_pages.to_string());
        message.push_str(&format!("\n\n{indicator}"));
    }
    for (index, item) in items.iter().enumerate() {
        let active = current_worktree == Some(item.worktree.as_str());
        let prefix = if active { "✅ " } else { "" };
        message.push_str(&format!(
            "\n{}. {prefix}{} [{}]",
            index + 1,
            get_project_folder_name(&item.worktree),
            item.worktree
        ));
    }
    message
}

/// One git worktree row for the `/worktree` list (JS `GitWorktreeEntry`
/// parity in `src/app/types/worktree.ts`: only the fields the text reply
/// needs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeListItem {
    pub path: String,
    pub branch: Option<String>,
    pub is_current: bool,
}

/// Render the `/worktree` reply (JS `worktree-command.ts` +
/// `buildWorktreeMenuView` text parity; the scaffold renders a numbered
/// text list instead of the Node inline keyboard).
/// Empty list renders `worktree.empty`; otherwise
/// `worktree.select_with_current` plus an optional `page_indicator` plus
/// one `N. [✅ ]folder [path]` line per item (`✅` marks the current
/// worktree like the Node button label). `locale` selects the dictionary
/// with `en` fallback; `None` means `en`.
pub fn render_worktree(
    items: &[WorktreeListItem],
    page: usize,
    total_pages: usize,
    locale: Option<&str>,
) -> String {
    let locale = locale.unwrap_or(crate::locale::FALLBACK_LOCALE);
    if items.is_empty() {
        return crate::locale::translate(locale, "worktree.empty");
    }
    let mut message = crate::locale::translate(locale, "worktree.select_with_current");
    if total_pages > 1 {
        let indicator = crate::locale::translate(locale, "projects.page_indicator");
        let indicator = fill(&indicator, "current", &(page + 1).to_string());
        let indicator = fill(&indicator, "total", &total_pages.to_string());
        message.push_str(&format!("\n\n{indicator}"));
    }
    for (index, item) in items.iter().enumerate() {
        let prefix = if item.is_current { "✅ " } else { "" };
        message.push_str(&format!(
            "\n{}. {prefix}{} [{}]",
            index + 1,
            get_project_folder_name(&item.path),
            item.path
        ));
    }
    message
}

/// Liveness of a recent session row (JS `RecentStatus` parity in
/// `recent-sessions-service.ts`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecentStatus {
    Running,
    Idle,
    Question,
    Permission,
}

/// Glyph for a [`RecentStatus`] (JS `GLYPHS` parity in
/// `recent-selection-menu.ts`).
pub fn recent_status_glyph(status: RecentStatus) -> &'static str {
    match status {
        RecentStatus::Running => "⏳",
        RecentStatus::Idle => "○",
        RecentStatus::Question => "❓",
        RecentStatus::Permission => "🔐",
    }
}

/// One recent-session row for the `/recent` list (JS `RecentSession`
/// parity: only the fields the text reply needs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentListItem {
    pub id: String,
    pub title: String,
    pub directory: String,
    pub updated: i64,
    pub status: RecentStatus,
}

/// Build a [`RecentListItem`] from one raw global-session entry with an
/// explicit status (pure; testable). Returns `None` when `id`/`directory`
/// are missing or blank. `title` defaults to `""` (formatted at render);
/// `updated` reads `time.updated` (else `updatedAt`, else `0`) so the list
/// sorts newest-first like Node.
pub fn recent_item_from_value(
    value: &serde_json::Value,
    status: RecentStatus,
) -> Option<RecentListItem> {
    let id = value
        .get("id")?
        .as_str()
        .map(str::trim)
        .filter(|id| !id.is_empty())?;
    let directory = value
        .get("directory")?
        .as_str()
        .map(str::trim)
        .filter(|directory| !directory.is_empty())?;
    let updated = value
        .get("time")
        .and_then(|time| time.get("updated"))
        .and_then(|updated| updated.as_i64())
        .or_else(|| value.get("updatedAt").and_then(|updated| updated.as_i64()))
        .unwrap_or(0);
    Some(RecentListItem {
        id: id.to_string(),
        title: value
            .get("title")
            .and_then(|title| title.as_str())
            .unwrap_or("")
            .to_string(),
        directory: directory.to_string(),
        updated,
        status,
    })
}

/// Sort recent rows newest-first by `updated` (JS `loadRecentSessions`
/// `sort((a, b) => b.time.updated - a.time.updated)` parity).
pub fn sort_recent_items(mut items: Vec<RecentListItem>) -> Vec<RecentListItem> {
    items.sort_by(|left, right| right.updated.cmp(&left.updated));
    items
}

/// Render the `/recent` reply (JS `recent-command.ts` + `buildRecentMenu`
/// text parity: the heading plus the status legend; the scaffold appends a
/// numbered text list instead of the Node inline keyboard).
/// Empty list renders `recent.empty`. `locale` selects the dictionary with
/// `en` fallback; `None` means `en`.
///
/// Deviation: project disambiguation is the plain folder name (Node
/// shortens and de-dupes long shared paths in the keyboard labels).
pub fn render_recent(items: &[RecentListItem], locale: Option<&str>) -> String {
    let locale = locale.unwrap_or(crate::locale::FALLBACK_LOCALE);
    if items.is_empty() {
        return crate::locale::translate(locale, "recent.empty");
    }
    let t = |key: &str| crate::locale::translate(locale, key);
    let mut message = format!(
        "{}\n\n⏳ {} · ○ {} · ❓ {} · 🔐 {}",
        t("recent.heading"),
        t("recent.running"),
        t("recent.idle"),
        t("recent.question"),
        t("recent.permission"),
    );
    for (index, item) in items.iter().enumerate() {
        message.push_str(&format!(
            "\n{}. {} [{}] {}",
            index + 1,
            recent_status_glyph(item.status),
            get_project_folder_name(&item.directory),
            format_session_title(&item.title, Some(locale)),
        ));
    }
    message
}

/// One user message for the `/messages` list (JS `UserMessageItem` parity
/// in `message-history-service.ts`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserMessageItem {
    pub id: String,
    pub text: String,
    /// Creation time in epoch millis (JS `info.time.created`); `0` when
    /// absent. Rendered as UTC `HH:MM` (Node renders local time).
    pub created: i64,
}

/// Build a [`UserMessageItem`] from one raw `session.messages` entry.
/// Returns `None` for non-`user` roles and for entries without text parts
/// (JS `loadUserMessages` filter parity). `id` falls back to the created
/// timestamp like Node (`info.id ?? created`).
pub fn user_message_item_from_value(value: &serde_json::Value) -> Option<UserMessageItem> {
    let info = value.get("info")?;
    if info.get("role")?.as_str() != Some("user") {
        return None;
    }
    let is_summary = info
        .get("summary")
        .and_then(|summary| summary.as_bool())
        .unwrap_or(false);
    if is_summary {
        return None;
    }
    let text = value
        .get("parts")?
        .as_array()?
        .iter()
        .filter(|part| part.get("type").and_then(|kind| kind.as_str()) == Some("text"))
        .filter_map(|part| part.get("text")?.as_str())
        .collect::<Vec<_>>()
        .join("")
        .trim()
        .to_string();
    if text.is_empty() {
        return None;
    }
    let created = info
        .get("time")
        .and_then(|time| time.get("created"))
        .and_then(|created| created.as_i64())
        .unwrap_or(0);
    let id = info
        .get("id")
        .and_then(|id| id.as_str())
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| created.to_string());
    Some(UserMessageItem { id, text, created })
}

/// Sort user messages newest-first by `created` (JS `loadUserMessages`
/// `sort((a, b) => b.created - a.created)` parity).
pub fn sort_user_messages(mut items: Vec<UserMessageItem>) -> Vec<UserMessageItem> {
    items.sort_by(|left, right| right.created.cmp(&left.created));
    items
}

/// Format epoch millis as UTC `HH:MM` (pure; testable). Node renders local
/// time; the scaffold pins UTC so button labels are deterministic.
pub fn format_message_time_utc(created_ms: i64) -> String {
    let day_seconds = created_ms.div_euclid(1000).rem_euclid(86_400);
    format!("{:02}:{:02}", day_seconds / 3600, (day_seconds % 3600) / 60)
}

/// One `/messages` list line: `[HH:MM] text` with whitespace collapsed and
/// the whole line capped at 64 chars like the Node button label (pure).
pub fn format_message_line(message: &UserMessageItem) -> String {
    const LIMIT: usize = 64;
    let prefix = format!("[{}] ", format_message_time_utc(message.created));
    let prefix_len = prefix.chars().count();
    let text: String = message
        .text
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let available = LIMIT.saturating_sub(prefix_len);
    let collapsed: String = text.chars().take(available.saturating_sub(3)).collect();
    if text.chars().count() > available {
        return format!("{prefix}{collapsed}...");
    }
    format!("{prefix}{}", text.chars().take(available).collect::<String>())
}

/// Render the `/messages` list reply (JS `formatMessagesSelectText` +
/// `buildMessagesListKeyboard` text parity; the scaffold renders a
/// numbered text list instead of the Node inline keyboard).
/// Empty list renders `messages.empty`; otherwise `messages.select` (page
/// 0) or `messages.select_page` plus one `N. [HH:MM] text` line per item.
/// `locale` selects the dictionary with `en` fallback; `None` means `en`.
pub fn render_messages(
    items: &[UserMessageItem],
    page: usize,
    locale: Option<&str>,
) -> String {
    let locale = locale.unwrap_or(crate::locale::FALLBACK_LOCALE);
    if items.is_empty() {
        return crate::locale::translate(locale, "messages.empty");
    }
    let mut message = if page == 0 {
        crate::locale::translate(locale, "messages.select")
    } else {
        fill(
            &crate::locale::translate(locale, "messages.select_page"),
            "page",
            &(page + 1).to_string(),
        )
    };
    for (index, item) in items.iter().enumerate() {
        message.push_str(&format!("\n{}. {}", index + 1, format_message_line(item)));
    }
    message
}

/// Render one message detail (JS `formatMessageDetailText` parity:
/// `[HH:MM]\n\ntext`, capped at the Telegram 4096-char limit). Pure.
pub fn render_message_detail(message: &UserMessageItem) -> String {
    let prefix = format!("[{}]\n\n", format_message_time_utc(message.created));
    let limit = crate::telegram::TELEGRAM_TEXT_LIMIT;
    let available = limit.saturating_sub(prefix.len());
    let mut end = available.min(message.text.len());
    while end > 0 && !message.text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{prefix}{}", message.text[..end].trim_end())
}

/// One raw catalog row (`name`/`description`/`source`) for the command and
/// skill catalogs (JS `loadCommandCatalog` / `loadSkillsCatalog` input
/// shape). The source filter lives in
/// `otb_core::session_pick_catalog_gates`; only shaping helpers live here.
pub fn raw_catalog_item_from_value(
    value: &serde_json::Value,
) -> Option<otb_core::session_pick_catalog_gates::RawCatalogItem> {
    let object = value.as_object()?;
    Some(
        otb_core::session_pick_catalog_gates::RawCatalogItem {
            name: object
                .get("name")
                .and_then(|name| name.as_str())
                .unwrap_or("")
                .to_string(),
            description: object
                .get("description")
                .and_then(|description| description.as_str())
                .map(str::to_string),
            source: object
                .get("source")
                .and_then(|source| source.as_str())
                .unwrap_or("")
                .to_string(),
        },
    )
}

/// Render a `/commands` or `/skills` list reply (JS `formatCommandsSelectText`
/// / `formatSkillsSelectText` header parity; the scaffold appends a numbered
/// text list instead of the Node inline keyboard, mirroring
/// `render_sessions`). `prefix` is `"commands"` or `"skills"` and selects
/// the `select` / `select_page` / `empty` keys. Empty list renders `empty`;
/// otherwise `select` (page 0) or `select_page` plus one `N. /name - desc`
/// line per item. `locale` selects the dictionary with `en` fallback.
pub fn render_catalog(
    prefix: &str,
    items: &[otb_core::session_pick_catalog_gates::CatalogEntry],
    page: usize,
    locale: Option<&str>,
) -> String {
    let locale = locale.unwrap_or(crate::locale::FALLBACK_LOCALE);
    if items.is_empty() {
        return crate::locale::translate(locale, &format!("{prefix}.empty"));
    }
    let mut message = if page == 0 {
        crate::locale::translate(locale, &format!("{prefix}.select"))
    } else {
        fill(
            &crate::locale::translate(locale, &format!("{prefix}.select_page")),
            "page",
            &(page + 1).to_string(),
        )
    };
    let no_description = crate::locale::translate(locale, &format!("{prefix}.no_description"));
    for (index, item) in items.iter().enumerate() {
        let description = item
            .description
            .as_deref()
            .map(str::trim)
            .filter(|description| !description.is_empty())
            .unwrap_or(&no_description);
        message.push_str(&format!("\n{}. /{} - {description}", index + 1, item.name));
    }
    message
}

/// Glyph for an MCP server status (JS `getStatusEmoji` parity in
/// `mcp-catalog-menu.ts`).
pub fn mcp_status_glyph(
    status: otb_core::session_pick_catalog_gates::McpStatusName,
) -> &'static str {
    use otb_core::session_pick_catalog_gates::McpStatusName as Name;
    match status {
        Name::Connected => "🟢",
        Name::Disabled => "🔴",
        Name::Failed => "⚠️",
        Name::NeedsAuth | Name::NeedsClientRegistration => "🔒",
    }
}

/// Render the `/mcps` list reply (JS `mcps.select` header parity; the
/// scaffold appends a numbered text list instead of the Node inline
/// keyboard). Empty list renders `mcps.empty`; otherwise `mcps.select` plus
/// one `N. emoji name` line per server. `locale` selects the dictionary with
/// `en` fallback.
pub fn render_mcps(
    items: &[otb_core::session_pick_catalog_gates::McpCatalogServerItem],
    locale: Option<&str>,
) -> String {
    let locale = locale.unwrap_or(crate::locale::FALLBACK_LOCALE);
    if items.is_empty() {
        return crate::locale::translate(locale, "mcps.empty");
    }
    let mut message = crate::locale::translate(locale, "mcps.select");
    for (index, item) in items.iter().enumerate() {
        message.push_str(&format!(
            "\n{}. {} {}",
            index + 1,
            mcp_status_glyph(item.status.name),
            item.name
        ));
    }
    message
}

/// Escape `&<>"'` for the HTML directory headers (JS `escapeHtml` parity in
/// `file-browser-menu.ts`). Pure.
pub fn escape_html(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// Which count line the `/open` browse header appends (pure): the key
/// suffix is resolved by the caller through the locale dictionary.
fn open_count_key(total: usize) -> &'static str {
    match otb_core::file_browser_service_gates::tree_count_kind(total) {
        otb_core::file_browser_service_gates::TreeCount::Empty => "open.no_subfolders",
        otb_core::file_browser_service_gates::TreeCount::One => "open.subfolder_count",
        otb_core::file_browser_service_gates::TreeCount::Many => "open.subfolders_count",
    }
}

/// Render the `/open` roots reply (JS `open.select_root` parity): the header
/// plus one `N. 📂 display` line per browser root. Pure.
pub fn render_open_roots(roots: &[String], home: &str, locale: Option<&str>) -> String {
    let locale = locale.unwrap_or(crate::locale::FALLBACK_LOCALE);
    let mut message = crate::locale::translate(locale, "open.select_root");
    for (index, root) in roots.iter().enumerate() {
        let display = otb_core::file_browser_service_gates::path_to_display_path(root, home);
        message.push_str(&format!("\n{}. 📂 {display}", index + 1));
    }
    message
}

/// Render one `/open` browse page (JS `buildTreeHeader` + entry-label parity:
/// `📂 display [ (page/total)]`, a count line, then one `N. 📁 name` line per
/// subdirectory). Pure; `locale` selects the dictionary with `en` fallback.
pub fn render_open_browse(
    display_path: &str,
    entries: &[String],
    page: usize,
    total_pages: usize,
    total_count: usize,
    locale: Option<&str>,
) -> String {
    let locale = locale.unwrap_or(crate::locale::FALLBACK_LOCALE);
    let mut message = format!(
        "{}{display_path}",
        otb_core::file_browser_service_gates::TREE_HEADER_PREFIX
    );
    if total_pages > 1 {
        message.push_str(&format!("  ({}/{total_pages})", page + 1));
    }
    let count_key = open_count_key(total_count);
    let count_line = crate::locale::translate(locale, count_key);
    // `subfolder_count` / `subfolders_count` carry a `{count}` slot.
    let count_line = fill(&count_line, "count", &total_count.to_string());
    message.push_str(&format!("\n{count_line}"));
    for (index, name) in entries.iter().enumerate() {
        message.push_str(&format!(
            "\n{}. {}",
            index + 1,
            otb_core::file_browser_service_gates::build_entry_label(name)
        ));
    }
    message
}

/// Render one `/ls` browse page (JS `buildLsHeader` + entry-label parity:
/// `📁 {header}` + `<code>display</code>` + optional ` (page/total)` +
/// `{total}` line, then one `N. 📁|📄 name` line per entry). Pure; `locale`
/// selects the dictionary with `en` fallback.
pub fn render_ls_browse(
    display_path: &str,
    entries: &[otb_core::file_browser_service_gates::LsEntry],
    page: usize,
    total_pages: usize,
    total_count: usize,
    locale: Option<&str>,
) -> String {
    let locale = locale.unwrap_or(crate::locale::FALLBACK_LOCALE);
    let t = |key: &str| crate::locale::translate(locale, key);
    let mut message = format!("📁 {}\n<code>{}</code>", t("ls.header"), escape_html(display_path));
    if total_pages > 1 {
        message.push_str(&format!("\n({}/{total_pages})", page + 1));
    }
    message.push_str(&format!("\n{}", fill(&t("ls.total"), "count", &total_count.to_string())));
    for (index, entry) in entries.iter().enumerate() {
        let glyph = if entry.is_dir { "📁" } else { "📄" };
        message.push_str(&format!("\n{}. {glyph} {}", index + 1, entry.name));
    }
    message
}

/// Render the `/rename` prompt (JS `rename.prompt` parity:
/// `📝 ... Current: {title}` with the formatted live title). Pure.
pub fn render_rename_prompt(current_title: &str, locale: Option<&str>) -> String {
    let locale = locale.unwrap_or(crate::locale::FALLBACK_LOCALE);
    fill(
        &crate::locale::translate(locale, "rename.prompt"),
        "title",
        &format_session_title(current_title, Some(locale)),
    )
}

/// Validate a rename title (JS `text.trim()` + empty-title gate parity in
/// `handleRenameTextAnswer`): trims and rejects blanks. Pure.
pub fn validate_rename_title(raw: &str) -> Option<String> {
    let title = raw.trim();
    if title.is_empty() {
        None
    } else {
        Some(title.to_string())
    }
}

/// Render the rename success reply (JS `rename.success` parity:
/// `✅ Session renamed to: {title}`). Pure.
pub fn render_rename_success(title: &str, locale: Option<&str>) -> String {
    let locale = locale.unwrap_or(crate::locale::FALLBACK_LOCALE);
    fill(
        &crate::locale::translate(locale, "rename.success"),
        "title",
        title,
    )
}

/// Render the `/abort` no-session notice (JS `stop.no_active_session`
/// parity). Pure.
pub fn render_abort_no_session(locale: Option<&str>) -> String {
    let locale = locale.unwrap_or(crate::locale::FALLBACK_LOCALE);
    crate::locale::translate(locale, "stop.no_active_session")
}

/// Render the `/abort` outcome (JS `stop.success` parity on an accepted
/// abort, `stop.warn_local_only` when the server-side abort failed). Pure;
/// the prompt-queue/goal side effects need a dispatcher and stay out.
pub fn render_abort_result(aborted: bool, locale: Option<&str>) -> String {
    let locale = locale.unwrap_or(crate::locale::FALLBACK_LOCALE);
    crate::locale::translate(
        locale,
        if aborted {
            "stop.success"
        } else {
            "stop.warn_local_only"
        },
    )
}

/// Render the `/detach` success reply (JS `detach.success` parity:
/// `✅ Detached from session: {title}` with the formatted title). Local-only:
/// the OpenCode session itself is never stopped. Pure.
pub fn render_detach_success(title: &str, locale: Option<&str>) -> String {
    let locale = locale.unwrap_or(crate::locale::FALLBACK_LOCALE);
    fill(
        &crate::locale::translate(locale, "detach.success"),
        "title",
        &format_session_title(title, Some(locale)),
    )
}

/// Render the `/task` reply (minimal scaffold of `task-command.ts`): empty
/// args render `task.prompt.schedule` (the JS interactive schedule prompt);
/// otherwise a plain-text ack echoing the collapsed schedule/prompt preview
/// (JS `truncateTaskPrompt` parity: whitespace collapsed, 100-char cap).
/// The interactive schedule→prompt flow needs a dispatcher and stays a
/// follow-up. Pure.
pub fn render_task_ack(args: &str, locale: Option<&str>) -> String {
    let locale = locale.unwrap_or(crate::locale::FALLBACK_LOCALE);
    let collapsed = args.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.is_empty() {
        return crate::locale::translate(locale, "task.prompt.schedule");
    }
    const LIMIT: usize = 100;
    let preview = if collapsed.chars().count() > LIMIT {
        format!(
            "{}...",
            collapsed.chars().take(LIMIT - 3).collect::<String>()
        )
    } else {
        collapsed
    };
    format!(
        "⏰ Task noted: {preview}\n\nInteractive scheduling is not wired in this build; saved tasks are listed with /tasklist."
    )
}

/// Render the `/opencode_start` status reply (safety scaffold: the bot never
/// spawns the server). A live server renders `opencode_start.already_running`
/// (JS parity, `common.unknown` when the version is missing); otherwise a
/// plain-text guard pointing at a manual start. Pure.
pub fn render_opencode_start_status(
    healthy: bool,
    version: Option<&str>,
    locale: Option<&str>,
) -> String {
    let locale = locale.unwrap_or(crate::locale::FALLBACK_LOCALE);
    if !healthy {
        return "⚠️ Automatic server start is disabled in this build.\n\nStart OpenCode manually, then use /status to check it."
            .to_string();
    }
    fill(
        &crate::locale::translate(locale, "opencode_start.already_running"),
        "version",
        version
            .map(str::trim)
            .filter(|version| !version.is_empty())
            .unwrap_or(&crate::locale::translate(locale, "common.unknown")),
    )
}

/// Render the `/opencode_stop` status reply (safety scaffold: the bot never
/// kills the server). A silent server renders `opencode_stop.not_running`
/// (JS parity); otherwise a plain-text guard leaving the server alone. Pure.
pub fn render_opencode_stop_status(
    healthy: bool,
    version: Option<&str>,
    locale: Option<&str>,
) -> String {
    let locale = locale.unwrap_or(crate::locale::FALLBACK_LOCALE);
    if !healthy {
        return crate::locale::translate(locale, "opencode_stop.not_running");
    }
    let suffix = version
        .map(str::trim)
        .filter(|version| !version.is_empty())
        .map(|version| format!(" (v{version})"))
        .unwrap_or_default();
    format!(
        "⚠️ Automatic server stop is disabled in this build.\n\nOpenCode Server is still running{suffix}."
    )
}

/// Render the `/reload` outcome (JS `reload.success` / `reload.failed`
/// parity). The scaffold re-reads the local `settings.json` only; the server
/// config rebuild needs a dispatcher and stays a follow-up. Pure.
pub fn render_reload_result(ok: bool, locale: Option<&str>) -> String {
    let locale = locale.unwrap_or(crate::locale::FALLBACK_LOCALE);
    crate::locale::translate(
        locale,
        if ok {
            "reload.success"
        } else {
            "reload.failed"
        },
    )
}

/// Entry decision for `/goal` (JS `goal-command.ts` guard order parity:
/// project first, then session, then the session/project match, then the
/// no-arguments gate). Returns the i18n key for the reply; the happy path
/// renders `goal.input.prompt` (the interactive objective flow needs a
/// dispatcher and stays a follow-up). Pure.
pub fn goal_entry_key(
    has_project: bool,
    has_session: bool,
    session_matches_project: bool,
    has_args: bool,
) -> &'static str {
    if !has_project {
        "messages.project_not_selected"
    } else if !has_session {
        "messages.session_not_selected"
    } else if !session_matches_project {
        "messages.session_project_mismatch"
    } else if has_args {
        "goal.arguments_not_supported"
    } else {
        "goal.input.prompt"
    }
}

/// Render the command-execution confirm (JS `commands.confirm` parity:
/// `Confirm execution of command {command}...` with `/name`). Pure.
pub fn render_command_confirm(name: &str, locale: Option<&str>) -> String {
    let locale = locale.unwrap_or(crate::locale::FALLBACK_LOCALE);
    fill(
        &crate::locale::translate(locale, "commands.confirm"),
        "command",
        &format!("/{name}"),
    )
}

/// Render the skill-execution confirm (JS `skills.confirm` parity with
/// `/name`). Pure.
pub fn render_skill_confirm(name: &str, locale: Option<&str>) -> String {
    let locale = locale.unwrap_or(crate::locale::FALLBACK_LOCALE);
    fill(
        &crate::locale::translate(locale, "skills.confirm"),
        "skill",
        &format!("/{name}"),
    )
}

/// Render the command/skill executing notice (JS `executing_prefix` parity:
/// `⚡ Executing command:\n/name` plus optional ` args`). Pure.
pub fn render_executing_notice(prefix_key: &str, name: &str, args: &str) -> String {
    let locale = crate::locale::FALLBACK_LOCALE;
    let prefix = crate::locale::translate(locale, prefix_key);
    if args.trim().is_empty() {
        format!("{prefix}\n/{name}")
    } else {
        format!("{prefix}\n/{name} {}", args.trim())
    }
}

/// Render one MCP server detail (JS `buildMcpsDetailText` parity:
/// `Server: {name}`, `Status: {status}`, optional `Error: {error}`, plus
/// the auth-required notice for `needs_auth` / `needs_client_registration`).
/// Pure.
pub fn render_mcp_detail(
    item: &otb_core::session_pick_catalog_gates::McpCatalogServerItem,
    locale: Option<&str>,
) -> String {
    let locale = locale.unwrap_or(crate::locale::FALLBACK_LOCALE);
    let t = |key: &str| crate::locale::translate(locale, key);
    let status_key = match item.status.name {
        otb_core::session_pick_catalog_gates::McpStatusName::Connected => {
            "mcps.status.connected"
        }
        otb_core::session_pick_catalog_gates::McpStatusName::Disabled => "mcps.status.disabled",
        otb_core::session_pick_catalog_gates::McpStatusName::Failed => "mcps.status.failed",
        otb_core::session_pick_catalog_gates::McpStatusName::NeedsAuth => "mcps.status.needs_auth",
        otb_core::session_pick_catalog_gates::McpStatusName::NeedsClientRegistration => {
            "mcps.status.needs_client_registration"
        }
    };
    let mut message = fill(&t("mcps.detail.title"), "name", &item.name);
    message.push_str(&format!(
        "\n\n{}",
        fill(&t("mcps.detail.status"), "status", &t(status_key))
    ));
    if matches!(
        item.status.name,
        otb_core::session_pick_catalog_gates::McpStatusName::Failed
            | otb_core::session_pick_catalog_gates::McpStatusName::NeedsClientRegistration
    ) {
        let error = item.status.error.as_deref().unwrap_or("");
        message.push_str(&format!("\n{}", fill(&t("mcps.detail.error"), "error", error)));
    }
    if matches!(
        item.status.name,
        otb_core::session_pick_catalog_gates::McpStatusName::NeedsAuth
            | otb_core::session_pick_catalog_gates::McpStatusName::NeedsClientRegistration
    ) {
        message.push_str(&format!("\n\n{}", t("mcps.auth_required")));
    }
    message
}

/// One provider row for the `/model` browser (JS `ProviderInfo` parity:
/// only the fields the menus need).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderListItem {
    pub id: String,
    pub name: String,
    pub model_count: usize,
}

/// One model row for the `/model` browser (JS `FavoriteModel` parity).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelListItem {
    pub provider_id: String,
    pub model_id: String,
}

/// One agent row for the `/agent` browser (JS `AgentInfo` parity: only the
/// name the picker needs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentListItem {
    pub name: String,
}

/// Build provider rows from a raw `config.providers` body (pure; testable).
/// Accepts the V1 `{"providers":[...]}` shape (each entry
/// `{id, name?, models:{...}}`) or a bare array of the same entries;
/// anything else yields an empty list. `name` falls back to `id`;
/// `model_count` counts the `models` map keys.
pub fn provider_list_from_value(value: &serde_json::Value) -> Vec<ProviderListItem> {
    let entries: &[serde_json::Value] = match value {
        serde_json::Value::Array(items) => items,
        serde_json::Value::Object(map) => match map.get("providers") {
            Some(serde_json::Value::Array(items)) => items,
            _ => return Vec::new(),
        },
        _ => return Vec::new(),
    };
    let mut items: Vec<ProviderListItem> = entries
        .iter()
        .filter_map(|entry| {
            let id = entry
                .get("id")?
                .as_str()
                .map(str::trim)
                .filter(|id| !id.is_empty())?;
            let name = entry
                .get("name")
                .and_then(|name| name.as_str())
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .unwrap_or(id);
            let model_count = entry
                .get("models")
                .and_then(|models| models.as_object())
                .map(|models| models.len())
                .unwrap_or(0);
            Some(ProviderListItem {
                id: id.to_string(),
                name: name.to_string(),
                model_count,
            })
        })
        .collect();
    items.sort_by(|left, right| {
        left.name
            .to_lowercase()
            .cmp(&right.name.to_lowercase())
            .then_with(|| left.id.cmp(&right.id))
    });
    items
}

/// Build model rows for one provider from a raw `config.providers` body
/// (pure; testable). Reads `providers[].models` keys for the matching
/// provider id, sorted; anything missing yields an empty list.
pub fn model_list_for_provider(
    value: &serde_json::Value,
    provider_id: &str,
) -> Vec<ModelListItem> {
    let entries: &[serde_json::Value] = match value {
        serde_json::Value::Array(items) => items,
        serde_json::Value::Object(map) => match map.get("providers") {
            Some(serde_json::Value::Array(items)) => items,
            _ => return Vec::new(),
        },
        _ => return Vec::new(),
    };
    let mut models: Vec<String> = entries
        .iter()
        .find(|entry| entry.get("id").and_then(|id| id.as_str()) == Some(provider_id))
        .and_then(|entry| entry.get("models"))
        .and_then(|models| models.as_object())
        .map(|models| models.keys().cloned().collect())
        .unwrap_or_default();
    models.sort();
    models
        .into_iter()
        .map(|model_id| ModelListItem {
            provider_id: provider_id.to_string(),
            model_id,
        })
        .collect()
}

/// Build variant ids for one model from a raw `config.providers` body
/// (pure; testable): `default` first, then the `models[model].variants` map
/// keys (minus a duplicate `default`), sorted. Missing maps yield
/// `["default"]`.
pub fn variant_list_for_model(
    value: &serde_json::Value,
    provider_id: &str,
    model_id: &str,
) -> Vec<String> {
    let entries: &[serde_json::Value] = match value {
        serde_json::Value::Array(items) => items,
        serde_json::Value::Object(map) => match map.get("providers") {
            Some(serde_json::Value::Array(items)) => items,
            _ => return vec!["default".to_string()],
        },
        _ => return vec!["default".to_string()],
    };
    let mut variants: Vec<String> = entries
        .iter()
        .find(|entry| entry.get("id").and_then(|id| id.as_str()) == Some(provider_id))
        .and_then(|entry| entry.get("models"))
        .and_then(|models| models.get(model_id))
        .and_then(|model| model.get("variants"))
        .and_then(|variants| variants.as_object())
        .map(|variants| {
            variants
                .keys()
                .filter(|id| *id != "default")
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    variants.sort();
    let mut out = vec!["default".to_string()];
    out.extend(variants);
    out
}

/// Build agent rows from raw `app.agents` values (pure; testable): keeps
/// non-hidden `primary`/`all` agents like `getAvailableAgents`, sorted by
/// name. Missing/blank names are dropped.
pub fn agent_list_from_values(values: &[serde_json::Value]) -> Vec<AgentListItem> {
    let mut items: Vec<AgentListItem> = values
        .iter()
        .filter_map(|value| {
            let name = value
                .get("name")?
                .as_str()
                .map(str::trim)
                .filter(|name| !name.is_empty())?;
            if value
                .get("hidden")
                .and_then(|hidden| hidden.as_bool())
                .unwrap_or(false)
            {
                return None;
            }
            match value.get("mode").and_then(|mode| mode.as_str()) {
                Some("primary") | Some("all") => Some(AgentListItem {
                    name: name.to_string(),
                }),
                _ => None,
            }
        })
        .collect();
    items.sort_by(|left, right| left.name.cmp(&right.name));
    items
}

/// Render the agent-changed confirm (JS `agent.changed_message` parity).
/// Pure.
pub fn render_agent_changed(name: &str, locale: Option<&str>) -> String {
    use otb_core::types_config::get_agent_display_name;
    let locale = locale.unwrap_or(crate::locale::FALLBACK_LOCALE);
    fill(
        &crate::locale::translate(locale, "agent.changed_message"),
        "name",
        &get_agent_display_name(name),
    )
}

/// Render the model-changed confirm (JS `model.changed_message` parity with
/// `provider/model (variant)` when a variant is set). Pure.
pub fn render_model_changed(
    provider: &str,
    model: &str,
    variant: Option<&str>,
    locale: Option<&str>,
) -> String {
    let locale = locale.unwrap_or(crate::locale::FALLBACK_LOCALE);
    let name = match variant.map(str::trim).filter(|variant| !variant.is_empty()) {
        Some(variant) => format!("{provider}/{model} ({variant})"),
        None => format!("{provider}/{model}"),
    };
    fill(
        &crate::locale::translate(locale, "model.changed_message"),
        "name",
        &name,
    )
}

/// Render the variant-changed confirm (JS `variant.changed_message` parity
/// with the capitalized display name). Pure.
pub fn render_variant_changed(variant: &str, locale: Option<&str>) -> String {
    let locale = locale.unwrap_or(crate::locale::FALLBACK_LOCALE);
    fill(
        &crate::locale::translate(locale, "variant.changed_message"),
        "name",
        &otb_core::agent_model_variant_gates::format_variant_for_display(variant),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_23_commands_matching_node_order() {
        assert_eq!(command_count(), 23);
        let core: Vec<&str> = otb_core::command_definitions_gates::COMMAND_DEFINITIONS
            .iter()
            .map(|definition| definition.command)
            .collect();
        assert_eq!(all_commands(), core);
    }

    #[test]
    fn lookup_accepts_plain_and_slash_names() {
        assert!(is_known_command("status"));
        assert!(is_known_command("/status"));
        assert!(is_known_command("opencode_start"));
        assert!(is_known_command("help"));
        // `start` is a Telegram built-in only, not one of the 23 definitions.
        assert!(!is_known_command("start"));
        assert!(!is_known_command("bogus"));
        assert!(!is_known_command(""));
    }

    #[test]
    fn dispatch_maps_all_23_names_to_scaffold_ack() {
        for name in COMMAND_NAMES {
            let ack = dispatch(name).expect("known command dispatches");
            assert_eq!(ack, format!("ok:{name} (scaffold-wired)"));
        }
        assert_eq!(
            dispatch("/status"),
            Some("ok:status (scaffold-wired)".to_string())
        );
        assert_eq!(dispatch("bogus"), None);
        assert_eq!(dispatch("start"), None);
    }

    fn healthy_full_snapshot() -> StatusSnapshot {
        StatusSnapshot {
            bot_version: "9.9.9".to_string(),
            healthy: true,
            opencode_version: Some("1.2.3".to_string()),
            agent_display: Some("Build".to_string()),
            model_display: "🧠 openai/gpt-4o".to_string(),
            project_display: Some("/work/proj".to_string()),
            worktree_path: Some("/work/wt-1".to_string()),
            session_title: Some("My session".to_string()),
        }
    }

    #[test]
    fn render_status_healthy_full_matches_js_template() {
        let rendered = render_status(&healthy_full_snapshot(), Some("en"));
        let expected = "🟢 OpenCode Server is running\n\
            \n\
            Bot version: 9.9.9\n\
            OpenCode version: 1.2.3\n\
            \n\
            Agent: Build\n\
            Model: 🧠 openai/gpt-4o\n\
            \n\
            Project: /work/proj\n\
            Worktree: /work/wt-1\n\
            \n\
            Current session: My session\n";
        assert_eq!(rendered, expected);
    }

    #[test]
    fn render_status_healthy_minimal_uses_not_selected_hints() {
        let snapshot = StatusSnapshot {
            bot_version: "1.0.0".to_string(),
            healthy: true,
            opencode_version: None,
            agent_display: None,
            model_display: "🧠 a/b".to_string(),
            project_display: None,
            worktree_path: None,
            session_title: None,
        };
        let rendered = render_status(&snapshot, Some("en"));
        let expected = "🟢 OpenCode Server is running\n\
            \n\
            Bot version: 1.0.0\n\
            \n\
            Agent: not set\n\
            Model: 🧠 a/b\n\
            \n\
            Project: not selected\n\
            Use /projects to select a project\n\
            Current session: not selected\n\
            Use /sessions to select one or /new to create one";
        assert_eq!(rendered, expected);
        assert!(!rendered.contains("OpenCode version"));
    }

    #[test]
    fn render_status_unhealthy_matches_js_template() {
        let snapshot = StatusSnapshot {
            bot_version: "1.0.0".to_string(),
            healthy: false,
            opencode_version: Some("1.2.3".to_string()),
            agent_display: Some("Build".to_string()),
            model_display: "🧠 a/b".to_string(),
            project_display: Some("/work/proj".to_string()),
            worktree_path: Some("/work/wt-1".to_string()),
            session_title: Some("My session".to_string()),
        };
        let rendered = render_status(&snapshot, Some("en"));
        let expected = "🔴 OpenCode Server is unavailable\n\
            Bot version: 1.0.0\n\
            \n\
            Use /opencode_start to start the server.";
        assert_eq!(rendered, expected);
    }

    #[test]
    fn render_status_unknown_locale_falls_back_to_en() {
        let snapshot = healthy_full_snapshot();
        assert_eq!(
            render_status(&snapshot, Some("xx")),
            render_status(&snapshot, Some("en"))
        );
        assert_eq!(
            render_status(&snapshot, None),
            render_status(&snapshot, Some("en"))
        );
    }

    #[test]
    fn session_item_parses_raw_list_entry() {
        let value = serde_json::json!({
            "id": "ses_1",
            "title": "Fix bug",
            "directory": "/work/proj",
        });
        assert_eq!(
            session_list_item_from_value(&value),
            Some(SessionListItem {
                id: "ses_1".to_string(),
                title: "Fix bug".to_string(),
                directory: "/work/proj".to_string(),
            })
        );
        assert!(session_list_item_from_value(&serde_json::json!({})).is_none());
        assert!(
            session_list_item_from_value(&serde_json::json!({ "id": "  " })).is_none()
        );
    }

    #[test]
    fn format_session_title_falls_back_for_blanks() {
        assert_eq!(format_session_title("Fix bug", Some("en")), "Fix bug");
        assert_eq!(format_session_title("  Fix bug  ", Some("en")), "Fix bug");
        assert_eq!(format_session_title("", Some("en")), "new session");
        assert_eq!(format_session_title("   ", Some("en")), "new session");
    }

    #[test]
    fn render_sessions_empty_matches_js_template() {
        assert_eq!(
            render_sessions(&[], None, 0, Some("en")),
            "📭 No sessions found.\n\nCreate a new session with /new."
        );
    }

    #[test]
    fn render_sessions_lists_titles_with_current_marker() {
        let items = vec![
            SessionListItem {
                id: "ses_1".to_string(),
                title: "Fix bug".to_string(),
                directory: "/work/proj".to_string(),
            },
            SessionListItem {
                id: "ses_2".to_string(),
                title: "".to_string(),
                directory: "/work/proj".to_string(),
            },
        ];
        let rendered = render_sessions(&items, Some("ses_1"), 0, Some("en"));
        assert_eq!(
            rendered,
            "Select a session:\n1. (current) Fix bug\n2. new session"
        );
    }

    #[test]
    fn render_sessions_later_page_uses_page_header() {
        let items = vec![SessionListItem {
            id: "ses_9".to_string(),
            title: "Ninth".to_string(),
            directory: "/work/proj".to_string(),
        }];
        let rendered = render_sessions(&items, None, 1, Some("en"));
        assert!(rendered.starts_with("Select a session (page 2):\n1. Ninth"));
    }

    #[test]
    fn render_session_selected_matches_js_template() {
        assert_eq!(
            render_session_selected("Fix bug", Some("en")),
            "✅ Session selected: Fix bug"
        );
        assert_eq!(
            render_session_selected("", Some("en")),
            "✅ Session selected: new session"
        );
        assert_eq!(
            render_session_selected("Fix bug", Some("xx")),
            render_session_selected("Fix bug", Some("en"))
        );
    }

    #[test]
    fn render_new_created_matches_js_template() {
        assert_eq!(
            render_new_created("Fix bug", Some("en")),
            "✅ New session created: Fix bug"
        );
        assert_eq!(
            render_new_created("", Some("en")),
            "✅ New session created: new session"
        );
    }

    #[test]
    fn render_help_matches_js_template() {
        let rendered = render_help(Some("en"));
        let expected = "📖 Help\n\
            \n\
            /status - Server and session status\n\
            /new - Create a new session\n\
            /abort - Stop current action\n\
            /detach - Detach from current session\n\
            /sessions - List sessions\n\
            /recent - Recent sessions across projects\n\
            /messages - Browse session messages\n\
            /settings - Change bot settings\n\
            /projects - List projects\n\
            /worktree - Switch git worktrees\n\
            /task - Create a scheduled task\n\
            /tasklist - List scheduled tasks\n\
            /rename - Rename current session\n\
            /commands - Custom commands\n\
            /skills - Skills catalog\n\
            /mcps - MCP servers\n\
            /opencode_start - Start OpenCode server\n\
            /opencode_stop - Stop OpenCode server\n\
            /reload - Reload OpenCode configuration\n\
            /goal - Set a session goal that continues on its own\n\
            /open - Add a project by browsing directories\n\
            /ls - List directory contents\n\
            /help - Help\n\
            \n\
            💡 Use the bottom keyboard buttons for the agent, model, variant, and context actions.";
        assert_eq!(rendered, expected);
    }

    #[test]
    fn render_help_unknown_locale_falls_back_to_en() {
        assert_eq!(render_help(Some("xx")), render_help(Some("en")));
        assert_eq!(render_help(None), render_help(Some("en")));
    }

    #[test]
    fn render_settings_view_matches_js_text() {
        assert_eq!(
            render_settings_view(Some("en")),
            "⚙️ Bot settings\nTap a setting to toggle its value:"
        );
        assert_eq!(
            render_settings_view(Some("xx")),
            render_settings_view(Some("en"))
        );
    }

    fn task_item(
        id: &str,
        prompt: &str,
        kind: &str,
        enabled: bool,
        created_at: &str,
        next_run_at: Option<&str>,
    ) -> ScheduledTaskItem {
        ScheduledTaskItem {
            id: id.to_string(),
            prompt: prompt.to_string(),
            kind: kind.to_string(),
            enabled,
            created_at: created_at.to_string(),
            next_run_at: next_run_at.map(str::to_string),
        }
    }

    #[test]
    fn scheduled_task_item_parses_and_rejects_blank_ids() {
        let value = serde_json::json!({
            "id": "task_1",
            "prompt": " ping ",
            "kind": "cron",
            "enabled": false,
            "createdAt": "2026-01-01T00:00:00Z",
            "nextRunAt": "2026-02-01T00:00:00Z",
        });
        assert_eq!(
            scheduled_task_item_from_value(&value),
            Some(task_item(
                "task_1",
                " ping ",
                "cron",
                false,
                "2026-01-01T00:00:00Z",
                Some("2026-02-01T00:00:00Z"),
            ))
        );
        // Missing id, blank id, and blank nextRunAt collapse as documented.
        assert!(scheduled_task_item_from_value(&serde_json::json!({})).is_none());
        assert!(
            scheduled_task_item_from_value(&serde_json::json!({ "id": "  " })).is_none()
        );
        let blank_next = scheduled_task_item_from_value(
            &serde_json::json!({ "id": "t", "nextRunAt": "  " }),
        )
        .expect("parses");
        assert_eq!(blank_next.next_run_at, None);
        assert_eq!(blank_next.kind, "cron");
        assert!(blank_next.enabled);
    }

    #[test]
    fn parse_instant_millis_orders_iso_and_rejects_garbage() {
        let base = parse_instant_millis("2026-02-01T00:00:00Z").expect("parses");
        // Same instant with an explicit offset compares equal.
        assert_eq!(
            parse_instant_millis("2026-02-01T02:00:00+02:00"),
            Some(base)
        );
        // Later calendar instant compares greater; date-only works.
        assert!(parse_instant_millis("2026-06-01T00:00:00Z") > Some(base));
        assert!(parse_instant_millis("2026-01-01").is_some());
        assert!(parse_instant_millis("not a date").is_none());
        assert!(parse_instant_millis("2026-13-01T00:00:00Z").is_none());
        assert!(parse_instant_millis("").is_none());
    }

    #[test]
    fn render_tasklist_empty_matches_js_template() {
        assert_eq!(
            render_tasklist(&[], Some("en")),
            "📭 No scheduled tasks yet."
        );
        assert_eq!(
            render_tasklist(&[], Some("xx")),
            render_tasklist(&[], Some("en"))
        );
    }

    #[test]
    fn render_tasklist_sorts_numbers_and_marks_disabled() {
        let items = vec![
            task_item(
                "b",
                "Second\ntask",
                "cron",
                true,
                "2026-01-01T00:00:00Z",
                Some("2026-06-01T00:00:00Z"),
            ),
            task_item(
                "a",
                "First",
                "cron",
                false,
                "2026-01-02T00:00:00Z",
                Some("2026-02-01T00:00:00Z"),
            ),
            task_item("c", "No  run", "once", true, "2026-01-03T00:00:00Z", None),
        ];
        let rendered = render_tasklist(&sort_scheduled_tasks(items), Some("en"));
        assert_eq!(
            rendered,
            "Select a scheduled task:\n1. 💤 First\n2. ⏰ Second task\n3. ⏰ No run"
        );
    }

    fn project_item(id: &str, worktree: &str, name: &str) -> ProjectListItem {
        ProjectListItem {
            id: id.to_string(),
            worktree: worktree.to_string(),
            name: name.to_string(),
            last_updated: 0,
        }
    }

    #[test]
    fn project_item_parses_raw_entry_with_fallbacks() {
        let value = serde_json::json!({
            "id": "dir_abc",
            "worktree": "/work/proj",
            "name": "proj",
            "time": { "updated": 42 },
        });
        let item = project_list_item_from_value(&value).expect("parses");
        assert_eq!(item.id, "dir_abc");
        assert_eq!(item.name, "proj");
        assert_eq!(item.last_updated, 42);
        // Missing name falls back to the worktree; missing time to 0.
        let bare = project_list_item_from_value(
            &serde_json::json!({ "id": "d", "worktree": "/w/x" }),
        )
        .expect("parses");
        assert_eq!(bare.name, "/w/x");
        assert_eq!(bare.last_updated, 0);
        assert!(project_list_item_from_value(&serde_json::json!({})).is_none());
        assert!(
            project_list_item_from_value(
                &serde_json::json!({ "id": "  ", "worktree": "/w" })
            )
            .is_none()
        );
    }

    #[test]
    fn project_folder_name_takes_last_segment() {
        assert_eq!(get_project_folder_name("/work/proj"), "proj");
        assert_eq!(get_project_folder_name("/work/proj/"), "proj");
        assert_eq!(get_project_folder_name("C:\\work\\proj"), "proj");
        assert_eq!(get_project_folder_name(""), "");
    }

    #[test]
    fn render_projects_empty_matches_js_template() {
        assert_eq!(
            render_projects(&[], None, None, 0, 0, Some("en")),
            "📭 No projects found.\n\nOpen a directory in OpenCode and create at least one session, then it will appear here."
        );
    }

    #[test]
    fn render_projects_lists_with_active_marker_and_paging() {
        let items = vec![
            project_item("a", "/work/alpha", "alpha"),
            project_item("b", "/work/beta", "beta"),
        ];
        let rendered = render_projects(
            &items,
            Some("/work/alpha"),
            Some("alpha"),
            0,
            1,
            Some("en"),
        );
        assert_eq!(
            rendered,
            "Select a project:\n\nCurrent: 🏗 alpha\n1. ✅ alpha [/work/alpha]\n2. beta [/work/beta]"
        );
        let paged = render_projects(&items, None, None, 1, 3, Some("en"));
        assert!(paged.starts_with("Select a project:\n\nPage 2/3\n1. alpha [/work/alpha]"));
        assert_eq!(
            render_projects(&items, None, None, 0, 1, Some("xx")),
            render_projects(&items, None, None, 0, 1, Some("en"))
        );
    }

    #[test]
    fn render_worktree_empty_matches_js_template() {
        assert_eq!(
            render_worktree(&[], 0, 0, Some("en")),
            "📭 No git worktrees found for the current repository."
        );
    }

    #[test]
    fn render_worktree_marks_current_row() {
        let items = vec![
            WorktreeListItem {
                path: "/work/main".to_string(),
                branch: Some("main".to_string()),
                is_current: true,
            },
            WorktreeListItem {
                path: "/work/feat".to_string(),
                branch: Some("feat".to_string()),
                is_current: false,
            },
        ];
        assert_eq!(
            render_worktree(&items, 0, 1, Some("en")),
            "Select a worktree:\n1. ✅ main [/work/main]\n2. feat [/work/feat]"
        );
    }

    #[test]
    fn recent_item_parses_and_sorts_newest_first() {
        let old = recent_item_from_value(
            &serde_json::json!({
                "id": "ses_old",
                "directory": "/work/proj",
                "title": "Old",
                "time": { "updated": 10 },
            }),
            RecentStatus::Idle,
        )
        .expect("parses");
        let new = recent_item_from_value(
            &serde_json::json!({
                "id": "ses_new",
                "directory": "/work/proj",
                "title": "",
                "time": { "updated": 20 },
            }),
            RecentStatus::Running,
        )
        .expect("parses");
        assert!(recent_item_from_value(&serde_json::json!({ "id": "x" }), RecentStatus::Idle).is_none());
        let sorted = sort_recent_items(vec![old, new]);
        assert_eq!(sorted[0].id, "ses_new");
        assert_eq!(recent_status_glyph(RecentStatus::Running), "⏳");
        assert_eq!(recent_status_glyph(RecentStatus::Permission), "🔐");
    }

    #[test]
    fn render_recent_empty_matches_js_template() {
        assert_eq!(
            render_recent(&[], Some("en")),
            "📭 No sessions found across projects."
        );
    }

    #[test]
    fn render_recent_lists_glyphs_with_heading_and_legend() {
        let items = vec![RecentListItem {
            id: "ses_1".to_string(),
            title: "".to_string(),
            directory: "/work/proj".to_string(),
            updated: 7,
            status: RecentStatus::Question,
        }];
        assert_eq!(
            render_recent(&items, Some("en")),
            "Recent sessions across projects (run /recent to refresh):\n\n⏳ running · ○ idle · ❓ awaiting an answer · 🔐 awaiting permission\n1. ❓ [proj] new session"
        );
    }

    #[test]
    fn user_message_item_filters_non_user_and_empty_text() {
        let user = serde_json::json!({
            "info": { "id": "msg_1", "role": "user", "time": { "created": 3_600_000 } },
            "parts": [{ "type": "text", "text": "  hello  " }, { "type": "image", "source": "x" }],
        });
        let item = user_message_item_from_value(&user).expect("parses");
        assert_eq!(item.text, "hello");
        assert_eq!(format_message_time_utc(item.created), "01:00");
        // Assistant roles, summaries, and text-less entries are dropped.
        assert!(
            user_message_item_from_value(&serde_json::json!({
                "info": { "id": "m", "role": "assistant", "time": { "created": 1 } },
                "parts": [{ "type": "text", "text": "hi" }],
            }))
            .is_none()
        );
        assert!(
            user_message_item_from_value(&serde_json::json!({
                "info": { "id": "m", "role": "user", "summary": true, "time": { "created": 1 } },
                "parts": [{ "type": "text", "text": "hi" }],
            }))
            .is_none()
        );
        assert!(
            user_message_item_from_value(&serde_json::json!({
                "info": { "id": "m", "role": "user", "time": { "created": 1 } },
                "parts": [{ "type": "text", "text": "   " }],
            }))
            .is_none()
        );
        // Missing id falls back to the created timestamp.
        let fallback = user_message_item_from_value(&serde_json::json!({
            "info": { "role": "user", "time": { "created": 9 } },
            "parts": [{ "type": "text", "text": "hi" }],
        }))
        .expect("parses");
        assert_eq!(fallback.id, "9");
    }

    #[test]
    fn message_time_formats_utc_midnight_and_truncates_lines() {
        assert_eq!(format_message_time_utc(0), "00:00");
        let long = UserMessageItem {
            id: "m".to_string(),
            text: "word ".repeat(30).trim().to_string(),
            created: 0,
        };
        let line = format_message_line(&long);
        assert!(line.starts_with("[00:00] "));
        assert_eq!(line.chars().count(), 64);
        assert!(line.ends_with("..."));
        let short = UserMessageItem {
            id: "m".to_string(),
            text: "hi".to_string(),
            created: 0,
        };
        assert_eq!(format_message_line(&short), "[00:00] hi");
    }

    #[test]
    fn render_messages_empty_matches_js_template() {
        assert_eq!(
            render_messages(&[], 0, Some("en")),
            "📭 No user messages in the current session."
        );
    }

    #[test]
    fn render_messages_lists_and_details_match_js_templates() {
        let items = vec![UserMessageItem {
            id: "msg_1".to_string(),
            text: "hello".to_string(),
            created: 3_600_000,
        }];
        assert_eq!(
            render_messages(&items, 0, Some("en")),
            "Choose a message:\n1. [01:00] hello"
        );
        assert_eq!(
            render_messages(&items, 1, Some("en")),
            "Choose a message (page 2):\n1. [01:00] hello"
        );
        assert_eq!(
            render_message_detail(&items[0]),
            "[01:00]\n\nhello"
        );
    }

    fn raw_catalog(name: &str, description: Option<&str>, source: &str) -> serde_json::Value {
        serde_json::json!({ "name": name, "description": description, "source": source })
    }

    #[test]
    fn raw_catalog_item_shapes_for_core_filters() {
        let raw = raw_catalog_item_from_value(&raw_catalog("review", Some("Review code"), "command"))
            .expect("parses");
        assert_eq!(raw.name, "review");
        assert_eq!(raw.description.as_deref(), Some("Review code"));
        assert_eq!(raw.source, "command");
        // Missing fields default to empty strings (the core filters drop
        // blank names / wrong sources).
        let bare = raw_catalog_item_from_value(&serde_json::json!({})).expect("parses");
        assert_eq!(bare.name, "");
        assert!(raw_catalog_item_from_value(&serde_json::json!([])).is_none());
    }

    #[test]
    fn render_catalog_lists_and_pages_match_js_templates() {
        use otb_core::session_pick_catalog_gates::{
            filter_command_catalog_items, filter_skills_catalog_items,
        };
        let raws = [
            raw_catalog_item_from_value(&raw_catalog("review", Some("Review code"), "command"))
                .expect("parses"),
            raw_catalog_item_from_value(&raw_catalog("blank", None, "command")).expect("parses"),
            // Wrong sources and the `goal` command are filtered like Node.
            raw_catalog_item_from_value(&raw_catalog("goal", Some("g"), "command")).expect("parses"),
            raw_catalog_item_from_value(&raw_catalog("helper", Some("h"), "skill")).expect("parses"),
        ];
        let commands = filter_command_catalog_items(&raws);
        assert_eq!(
            render_catalog("commands", &commands, 0, Some("en")),
            "Choose an OpenCode command:\n1. /review - Review code\n2. /blank - No description"
        );
        assert_eq!(
            render_catalog("commands", &commands, 1, Some("en")),
            "Choose an OpenCode command (page 2):\n1. /review - Review code\n2. /blank - No description"
        );
        let skills = filter_skills_catalog_items(&raws);
        assert_eq!(
            render_catalog("skills", &skills, 0, Some("en")),
            "Choose an OpenCode skill:\n1. /helper - h"
        );
        assert_eq!(
            render_catalog("commands", &[], 0, Some("en")),
            "📭 No OpenCode commands are available for this project."
        );
        assert_eq!(
            render_catalog("skills", &[], 0, Some("en")),
            "📭 No OpenCode skills are available for this project."
        );
        assert_eq!(
            render_catalog("commands", &commands, 0, Some("xx")),
            render_catalog("commands", &commands, 0, Some("en"))
        );
    }

    #[test]
    fn render_mcps_lists_status_glyphs() {
        use otb_core::session_pick_catalog_gates::{
            CatalogJson, McpCatalogServerItem, McpServerStatus, McpStatusName as Name,
            parse_mcp_catalog_servers,
        };
        // Record form (live `mcp.status` shape) with an unknown status skipped.
        let record = CatalogJson::Obj(vec![
            (
                "gh".to_string(),
                CatalogJson::Obj(vec![(
                    "status".to_string(),
                    CatalogJson::Str("connected".to_string()),
                )]),
            ),
            (
                "db".to_string(),
                CatalogJson::Obj(vec![
                    ("status".to_string(), CatalogJson::Str("failed".to_string())),
                    ("error".to_string(), CatalogJson::Str("boom".to_string())),
                ]),
            ),
            (
                "old".to_string(),
                CatalogJson::Obj(vec![(
                    "status".to_string(),
                    CatalogJson::Str("drifted".to_string()),
                )]),
            ),
        ]);
        let servers = parse_mcp_catalog_servers(&record).expect("parses");
        assert_eq!(
            render_mcps(&servers, Some("en")),
            "MCP servers:\n1. 🟢 gh\n2. ⚠️ db"
        );
        assert_eq!(render_mcps(&[], Some("en")), "📭 No MCP servers configured.");
        assert_eq!(
            mcp_status_glyph(Name::Disabled),
            "🔴",
        );
        assert_eq!(
            McpCatalogServerItem {
                name: "x".to_string(),
                status: McpServerStatus {
                    name: Name::NeedsClientRegistration,
                    error: Some(String::new()),
                },
            }
            .status
            .name,
            Name::NeedsClientRegistration
        );
        assert_eq!(
            render_mcps(&servers, Some("xx")),
            render_mcps(&servers, Some("en"))
        );
    }

    #[test]
    fn open_and_ls_renders_match_js_templates() {
        use otb_core::file_browser_service_gates::LsEntry;
        assert_eq!(
            render_open_roots(
                &["/root/a".to_string(), "/root/b".to_string()],
                "/home/user",
                Some("en")
            ),
            "📂 Select a root directory to browse:\n1. 📂 /root/a\n2. 📂 /root/b"
        );
        assert_eq!(
            render_open_browse("/work/proj", &["src".to_string()], 0, 1, 1, Some("en")),
            "📂 /work/proj\n1 subfolder\n1. 📁 src"
        );
        assert_eq!(
            render_open_browse("/work/proj", &[], 0, 1, 0, Some("en")),
            "📂 /work/proj\n📭 No subfolders"
        );
        let entries = vec![
            LsEntry {
                name: "src".to_string(),
                full_path: "/work/proj/src".to_string(),
                is_dir: true,
            },
            LsEntry {
                name: "a<b>&.txt".to_string(),
                full_path: "/work/proj/a<b>&.txt".to_string(),
                is_dir: false,
            },
        ];
        assert_eq!(
            render_ls_browse("/work/proj", &entries, 0, 1, 2, Some("en")),
            "📁 Directory Listing\n<code>/work/proj</code>\nTotal: 2 items\n1. 📁 src\n2. 📄 a<b>&.txt"
        );
        assert_eq!(escape_html("a<b>&\"'c"), "a&lt;b&gt;&amp;&quot;&#39;c");
    }

    #[test]
    fn rename_prompt_success_and_validation_match_js_templates() {
        assert_eq!(
            render_rename_prompt("Fix bug", Some("en")),
            "📝 Enter new title for session:\n\nCurrent: Fix bug"
        );
        assert_eq!(
            render_rename_prompt("", Some("en")),
            "📝 Enter new title for session:\n\nCurrent: new session"
        );
        assert_eq!(
            render_rename_success("New title", Some("en")),
            "✅ Session renamed to: New title"
        );
        assert_eq!(validate_rename_title("  New title  ").as_deref(), Some("New title"));
        assert_eq!(validate_rename_title("   "), None);
        assert_eq!(
            render_rename_prompt("Fix bug", Some("xx")),
            render_rename_prompt("Fix bug", Some("en"))
        );
    }

    #[test]
    fn abort_renders_match_js_stop_templates() {
        assert_eq!(
            render_abort_no_session(Some("en")),
            "🛑 Agent was not started\n\nCreate a session with /new or select one via /sessions."
        );
        assert_eq!(
            render_abort_result(true, Some("en")),
            "✅ Agent action interrupted. No more messages from this run will be sent."
        );
        assert_eq!(
            render_abort_result(false, Some("en")),
            "⚠️ Event stream stopped locally, but server-side abort failed."
        );
        assert_eq!(
            render_abort_result(true, Some("xx")),
            render_abort_result(true, Some("en"))
        );
    }

    #[test]
    fn detach_success_carries_formatted_title() {
        assert_eq!(
            render_detach_success("Fix bug", Some("en")),
            "✅ Detached from session: Fix bug\n\nThe OpenCode session was not stopped. If it is still running, it will continue separately. To check it later, select it again via /recent."
        );
        assert_eq!(
            render_detach_success("", Some("en")),
            "✅ Detached from session: new session\n\nThe OpenCode session was not stopped. If it is still running, it will continue separately. To check it later, select it again via /recent."
        );
    }

    #[test]
    fn task_ack_prompts_for_schedule_or_echoes_preview() {
        assert_eq!(
            render_task_ack("", Some("en")),
            "⏰ Send the task schedule in natural language.\n\nExamples:\n- every 5 minutes\n- every day at 17:00\n- tomorrow at 12:00"
        );
        assert_eq!(
            render_task_ack("   ", Some("en")),
            render_task_ack("", Some("en"))
        );
        let ack = render_task_ack("every day at 17:00 run tests", Some("en"));
        assert!(ack.starts_with("⏰ Task noted: every day at 17:00 run tests"));
        // Whitespace collapses and long input truncates at 100 chars like JS.
        let long = format!("word {}", "x".repeat(200));
        let ack = render_task_ack(&long, Some("en"));
        let preview = ack
            .strip_prefix("⏰ Task noted: ")
            .expect("ack prefix")
            .lines()
            .next()
            .expect("preview line");
        assert_eq!(preview.chars().count(), 100);
        assert!(preview.ends_with("..."));
    }

    #[test]
    fn opencode_start_stop_status_never_claims_control() {
        assert_eq!(
            render_opencode_start_status(true, Some("1.2.3"), Some("en")),
            "✅ OpenCode Server is already running\n\nVersion: 1.2.3"
        );
        assert_eq!(
            render_opencode_start_status(true, None, Some("en")),
            "✅ OpenCode Server is already running\n\nVersion: unknown"
        );
        let guard = render_opencode_start_status(false, None, Some("en"));
        assert!(guard.contains("disabled in this build"));
        assert!(!guard.contains("PID"));
        assert_eq!(
            render_opencode_stop_status(false, None, Some("en")),
            "⚠️ OpenCode Server is not running"
        );
        let guard = render_opencode_stop_status(true, Some("2.0.0"), Some("en"));
        assert!(guard.contains("disabled in this build"));
        assert!(guard.contains("still running"));
        assert!(guard.contains("2.0.0"));
        assert_eq!(
            render_opencode_start_status(true, Some("1.2.3"), Some("xx")),
            render_opencode_start_status(true, Some("1.2.3"), Some("en"))
        );
    }

    #[test]
    fn reload_result_matches_js_templates() {
        assert_eq!(
            render_reload_result(true, Some("en")),
            "✅ OpenCode configuration reloaded"
        );
        assert_eq!(
            render_reload_result(false, Some("en")),
            "🔴 Failed to reload OpenCode configuration"
        );
    }

    #[test]
    fn goal_entry_key_follows_js_guard_order() {
        // Project first, then session, then the match, then the arguments gate.
        assert_eq!(
            goal_entry_key(false, false, false, false),
            "messages.project_not_selected"
        );
        assert_eq!(
            goal_entry_key(true, false, false, false),
            "messages.session_not_selected"
        );
        assert_eq!(
            goal_entry_key(true, true, false, false),
            "messages.session_project_mismatch"
        );
        assert_eq!(
            goal_entry_key(true, true, true, true),
            "goal.arguments_not_supported"
        );
        assert_eq!(
            goal_entry_key(true, true, true, false),
            "goal.input.prompt"
        );
    }
}
