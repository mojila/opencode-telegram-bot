//! Catalog + settings menu pure gates (slice 57, GREEN).
//!
//! Rust port of the pure remainder in:
//! - `src/bot/menus/command-catalog-menu.ts` (`COMMANDS_*` prefixes,
//!   `formatExecutingCommandMessage` prefix/`\n`/`/name` + args text with
//!   the `code` entity offset/length, `buildCommandPageCallback`,
//!   `parseCommandPageCallback` / `parseCommandSelectCallback` JS-`Number()`
//!   parses, `formatCommandsSelectText` first-page gate,
//!   `formatCommandButtonLabel` `"/name - desc"` with the no-description
//!   fallback and the 64-char `...` cut, `calculateCommandsPaginationRange`)
//! - `src/bot/menus/skills-catalog-menu.ts` (same shape over `skills:`:
//!   `SKILLS_*` prefixes, `formatExecutingSkillMessage`, page/select
//!   `Number()` parses, select-text gate, `"/name - desc"` label cut,
//!   `calculateSkillsPaginationRange`)
//! - `src/bot/menus/mcp-catalog-menu.ts` (`MCPS_*` prefixes, status emoji
//!   map, `formatMcpButtonLabel` emoji + 64-char `...` cut,
//!   `parseMcpSelectCallback`, the detail-keyboard toggle gate
//!   (`connected` -> disable, `disabled`/`failed` -> enable, otherwise no
//!   toggle row), the error-line / auth-notice gates behind
//!   `buildMcpsDetailText`)
//! - `src/bot/menus/settings-menu.ts` (`SETTINGS_*` callback constants,
//!   the boolean on/off value gate, the three-way TTS value gate, the
//!   V1-on/off vs V2-queue/steer/off prompt-queue value gate, the
//!   draft/edit streaming value gate, the row plan: compact-output row
//!   always, then delete-progress *or* thinking + diff-files rows, then
//!   the streaming/footer/pin/background/tts/queue rows)
//!
//! No grammy/Telegram I/O, no SDK fetches, no store/`config`/env reads live
//! here — only callback prefixes, `Number()` parses, label cuts, pagination
//! math, status maps, and row plans. Callers resolve `t(...)` locales,
//! catalog items, and keyboards.
//!
//! DEVIATIONS from TS:
//! 1. `String.length` / `slice` count UTF-16 code units in TS. The port
//!    works on `char`s; parity fixtures use ASCII-only inputs where the two
//!    coincide.
//! 2. `COMMANDS_CALLBACK_PAGE_PREFIX` / `SKILLS_CALLBACK_PAGE_PREFIX` are
//!    private in TS. The port exposes them so the build/parse round-trip is
//!    testable; the wire strings are unchanged.
//! 3. `Number(raw)` reads empty/whitespace-only as `0`, trims surrounding
//!    whitespace, and accepts hex/finite-decimal forms; only a finite
//!    non-negative integer passes the `isInteger` + `>= 0` gate. The port
//!    mirrors that; fixtures stick to decimal/empty/whitespace/negative/
//!    fractional/alpha inputs where the behaviours coincide.
//! 4. `t(...)` lookups (select texts, `no_description`, detail lines, value
//!    labels) are implicit in TS. The port takes the translated labels as
//!    explicit arguments or returns a [`CatalogSelectTextKind`] / row plan
//!    so the gates stay pure; callers render the locale.
//! 5. `formatCommandButtonLabel` / `formatSkillButtonLabel` are private in
//!    TS. The port exposes them over an explicit `no_description_label`
//!    argument instead of the `t(...)` fallback.
//! 6. `getStatusLabel` stays with the caller (locale); only the emoji map
//!    and the show-error / show-auth predicates are ported.
//! 7. `McpStatus` (`@opencode-ai/sdk/v2`) is flattened to
//!    [`McpStatusKind`]; unknown strings read as [`McpStatusKind::Unknown`].

use std::fmt::Write as _;

// ---------------------------------------------------------------------------
// Shared budgets + select-text kind
// ---------------------------------------------------------------------------

/// Inline-button label budget shared by the catalog menus.
pub const MAX_CATALOG_BUTTON_LABEL_LENGTH: usize = 64;

/// First-page vs later-page select text (TS: `format*SelectText`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogSelectTextKind {
    FirstPage,
    Page(u64),
}

/// Which select-text branch a page takes: `0` reads as the base text,
/// anything else as the paged text (TS: `page === 0 ? select : select_page`).
pub fn catalog_select_text_kind(page: u64) -> CatalogSelectTextKind {
    if page == 0 {
        CatalogSelectTextKind::FirstPage
    } else {
        CatalogSelectTextKind::Page(page)
    }
}

/// Clamped pagination window shared by the command/skills ranges.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CatalogPaginationRange {
    pub page: i64,
    pub total_pages: i64,
    pub start_index: i64,
    pub end_index: i64,
}

/// Executing-message text plus the `code`-entity span over `/name`
/// (TS: `formatExecutingCommandMessage` / `formatExecutingSkillMessage`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutingCatalogMessage {
    pub text: String,
    pub code_offset: usize,
    pub code_length: usize,
}

// ---------------------------------------------------------------------------
// Shared JS-`Number()` gate
// ---------------------------------------------------------------------------

/// Parse a JS `Number()`-produced page: empty/whitespace-only reads as `0`;
/// otherwise the trimmed text must parse as a finite non-negative integer.
fn parse_js_number_page(raw: &str) -> Option<u64> {
    if raw.trim().is_empty() {
        return Some(0);
    }
    let trimmed = raw.trim();
    // Minimal hex/binary/octal support mirroring JS `Number()`.
    let (negative, digits) = match trimmed.strip_prefix(['+', '-']) {
        Some(rest) => (trimmed.starts_with('-'), rest),
        None => (false, trimmed),
    };
    if negative {
        // `-0` reads as `0` in the `page < 0` gate (`-0 < 0` is false).
        if let Ok(value) = trimmed.parse::<f64>() {
            if value == 0.0 {
                return Some(0);
            }
        }
        return None;
    }
    let lower = digits.to_lowercase();
    if let Some(hex) = lower.strip_prefix("0x") {
        if !hex.is_empty() && hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return u64::from_str_radix(hex, 16).ok();
        }
        return None;
    }
    if let Some(bin) = lower.strip_prefix("0b") {
        if !bin.is_empty() && bin.bytes().all(|b| b == b'0' || b == b'1') {
            return u64::from_str_radix(bin, 2).ok();
        }
        return None;
    }
    if let Some(oct) = lower.strip_prefix("0o") {
        if !oct.is_empty() && oct.bytes().all(|b| b >= b'0' && b <= b'7') {
            return u64::from_str_radix(oct, 8).ok();
        }
        return None;
    }
    match trimmed.parse::<f64>() {
        Ok(value)
            if value.is_finite()
                && value.fract() == 0.0
                && value >= 0.0
                && value <= u64::MAX as f64 =>
        {
            Some(value as u64)
        }
        _ => None,
    }
}

/// Truncate a button label to the 64-`char` budget with a `...` cut
/// (TS: `rawLabel.slice(0, 64 - 3) + "..."`).
fn truncate_catalog_label(raw: &str) -> String {
    if raw.chars().count() <= MAX_CATALOG_BUTTON_LABEL_LENGTH {
        return raw.to_string();
    }
    let kept: String = raw
        .chars()
        .take(MAX_CATALOG_BUTTON_LABEL_LENGTH - 3)
        .collect();
    format!("{kept}...")
}

/// Resolve `description?.trim() || no_description` (TS fallback).
fn resolve_catalog_description<'a>(
    description: Option<&'a str>,
    no_description_label: &'a str,
) -> &'a str {
    match description {
        Some(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                no_description_label
            } else {
                trimmed
            }
        }
        None => no_description_label,
    }
}

/// Shared clamped window (TS: `calculate*PaginationRange`: `pageSize` floor
/// of 1, empty list still yields one page).
fn calculate_catalog_pagination_range(
    total: i64,
    page: i64,
    page_size: i64,
) -> CatalogPaginationRange {
    let safe_page_size = page_size.max(1);
    let total_pages = if total <= 0 {
        1
    } else {
        ((total + safe_page_size - 1) / safe_page_size).max(1)
    };
    let normalized_page = page.clamp(0, total_pages - 1);
    let start_index = if total <= 0 {
        0
    } else {
        normalized_page * safe_page_size
    };
    let end_index = (normalized_page * safe_page_size + safe_page_size)
        .min(total)
        .max(0);
    let end_index = if total <= 0 { 0 } else { end_index };
    CatalogPaginationRange {
        page: normalized_page,
        total_pages,
        start_index,
        end_index,
    }
}

/// Shared `{prefix}\n/{name}[ {args}]` executing message with the `code`
/// span over `/name` (TS: `formatExecuting*Message`).
fn format_executing_catalog_message(
    executing_prefix: &str,
    name: &str,
    args: &str,
) -> ExecutingCatalogMessage {
    let command_text = format!("/{name}");
    let args_suffix = if args.is_empty() {
        String::new()
    } else {
        format!(" {args}")
    };
    ExecutingCatalogMessage {
        text: format!("{executing_prefix}\n{command_text}{args_suffix}"),
        code_offset: executing_prefix.chars().count() + 1,
        code_length: command_text.chars().count(),
    }
}

// ---------------------------------------------------------------------------
// Command catalog menu (`command-catalog-menu.ts`)
// ---------------------------------------------------------------------------

/// `commands:` callback prefix.
pub const COMMANDS_CALLBACK_PREFIX: &str = "commands:";
/// `commands:select:` callback prefix.
pub const COMMANDS_CALLBACK_SELECT_PREFIX: &str = "commands:select:";
/// `commands:page:` callback prefix (private in TS).
pub const COMMANDS_CALLBACK_PAGE_PREFIX: &str = "commands:page:";
/// `commands:cancel` callback data.
pub const COMMANDS_CALLBACK_CANCEL: &str = "commands:cancel";
/// `commands:execute` callback data.
pub const COMMANDS_CALLBACK_EXECUTE: &str = "commands:execute";

/// `{prefix}\n/{name}[ {args}]` with the `code` span over `/name`
/// (TS: `formatExecutingCommandMessage`; `offset` is
/// `prefix.length + 1`, `length` is the `/name` length).
pub fn format_executing_command_message(
    executing_prefix: &str,
    command_name: &str,
    args: &str,
) -> ExecutingCatalogMessage {
    format_executing_catalog_message(executing_prefix, command_name, args)
}

/// `commands:page:<page>` callback (TS: `buildCommandPageCallback`).
pub fn build_command_page_callback(page: u64) -> String {
    format!("{COMMANDS_CALLBACK_PAGE_PREFIX}{page}")
}

/// Page of a `commands:page:<n>` callback (TS mirrors JS `Number()`:
/// empty/whitespace-only reads as `0`; otherwise the payload must be a
/// finite non-negative integer).
pub fn parse_command_page_callback(data: &str) -> Option<u64> {
    let raw = data.strip_prefix(COMMANDS_CALLBACK_PAGE_PREFIX)?;
    parse_js_number_page(raw)
}

/// Index of a `commands:select:<n>` callback (same `Number()` gate).
pub fn parse_command_select_callback(data: &str) -> Option<u64> {
    let raw = data.strip_prefix(COMMANDS_CALLBACK_SELECT_PREFIX)?;
    parse_js_number_page(raw)
}

/// `commands:select:<index>` callback for a global list index.
pub fn command_select_callback_for_index(index: usize) -> String {
    format!("{COMMANDS_CALLBACK_SELECT_PREFIX}{index}")
}

/// `"/name - desc"` with the no-description fallback and the 64-char
/// `...` cut (TS: the private `formatCommandButtonLabel`).
pub fn format_command_button_label(
    name: &str,
    description: Option<&str>,
    no_description_label: &str,
) -> String {
    let description = resolve_catalog_description(description, no_description_label);
    truncate_catalog_label(&format!("/{name} - {description}"))
}

/// Clamped window over `total_commands` (TS:
/// `calculateCommandsPaginationRange`: `pageSize` floor of 1, empty list
/// still yields one page).
pub fn calculate_commands_pagination_range(
    total_commands: i64,
    page: i64,
    page_size: i64,
) -> CatalogPaginationRange {
    calculate_catalog_pagination_range(total_commands, page, page_size)
}

// ---------------------------------------------------------------------------
// Skills catalog menu (`skills-catalog-menu.ts`)
// ---------------------------------------------------------------------------

/// `skills:` callback prefix.
pub const SKILLS_CALLBACK_PREFIX: &str = "skills:";
/// `skills:select:` callback prefix.
pub const SKILLS_CALLBACK_SELECT_PREFIX: &str = "skills:select:";
/// `skills:page:` callback prefix (private in TS).
pub const SKILLS_CALLBACK_PAGE_PREFIX: &str = "skills:page:";
/// `skills:cancel` callback data.
pub const SKILLS_CALLBACK_CANCEL: &str = "skills:cancel";
/// `skills:execute` callback data.
pub const SKILLS_CALLBACK_EXECUTE: &str = "skills:execute";

/// `{prefix}\n/{name}[ {args}]` with the `code` span over `/name`
/// (TS: `formatExecutingSkillMessage`).
pub fn format_executing_skill_message(
    executing_prefix: &str,
    skill_name: &str,
    args: &str,
) -> ExecutingCatalogMessage {
    format_executing_catalog_message(executing_prefix, skill_name, args)
}

/// `skills:page:<page>` callback (TS: `buildSkillPageCallback`).
pub fn build_skill_page_callback(page: u64) -> String {
    format!("{SKILLS_CALLBACK_PAGE_PREFIX}{page}")
}

/// Page of a `skills:page:<n>` callback (same `Number()` gate).
pub fn parse_skill_page_callback(data: &str) -> Option<u64> {
    let raw = data.strip_prefix(SKILLS_CALLBACK_PAGE_PREFIX)?;
    parse_js_number_page(raw)
}

/// Index of a `skills:select:<n>` callback (same `Number()` gate).
pub fn parse_skill_select_callback(data: &str) -> Option<u64> {
    let raw = data.strip_prefix(SKILLS_CALLBACK_SELECT_PREFIX)?;
    parse_js_number_page(raw)
}

/// `skills:select:<index>` callback for a global list index.
pub fn skill_select_callback_for_index(index: usize) -> String {
    format!("{SKILLS_CALLBACK_SELECT_PREFIX}{index}")
}

/// `"/name - desc"` with the no-description fallback and the 64-char
/// `...` cut (TS: the private `formatSkillButtonLabel`).
pub fn format_skill_button_label(
    name: &str,
    description: Option<&str>,
    no_description_label: &str,
) -> String {
    let description = resolve_catalog_description(description, no_description_label);
    truncate_catalog_label(&format!("/{name} - {description}"))
}

/// Clamped window over `total_skills` (TS:
/// `calculateSkillsPaginationRange`).
pub fn calculate_skills_pagination_range(
    total_skills: i64,
    page: i64,
    page_size: i64,
) -> CatalogPaginationRange {
    calculate_catalog_pagination_range(total_skills, page, page_size)
}

// ---------------------------------------------------------------------------
// MCP catalog menu (`mcp-catalog-menu.ts`)
// ---------------------------------------------------------------------------

/// `mcps:` callback prefix.
pub const MCPS_CALLBACK_PREFIX: &str = "mcps:";
/// `mcps:select:` callback prefix.
pub const MCPS_CALLBACK_SELECT_PREFIX: &str = "mcps:select:";
/// `mcps:toggle` callback data.
pub const MCPS_CALLBACK_TOGGLE: &str = "mcps:toggle";
/// `mcps:back` callback data.
pub const MCPS_CALLBACK_BACK: &str = "mcps:back";
/// `mcps:cancel` callback data.
pub const MCPS_CALLBACK_CANCEL: &str = "mcps:cancel";

/// Flattened MCP server status (TS: `McpStatus["status"]` plus the
/// `default` -> unknown branch).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpStatusKind {
    Connected,
    Disabled,
    Failed,
    NeedsAuth,
    NeedsClientRegistration,
    Unknown,
}

/// Status glyph: `🟢` connected, `🔴` disabled, `⚠️` failed,
/// `🔒` needs_auth / needs_client_registration, `❓` otherwise
/// (TS: `getStatusEmoji`).
pub fn mcp_status_emoji(status: McpStatusKind) -> &'static str {
    match status {
        McpStatusKind::Connected => "🟢",
        McpStatusKind::Disabled => "🔴",
        McpStatusKind::Failed => "⚠️",
        McpStatusKind::NeedsAuth | McpStatusKind::NeedsClientRegistration => "🔒",
        McpStatusKind::Unknown => "❓",
    }
}

/// `{emoji} {name}` with the 64-char `...` cut (TS: the private
/// `formatMcpButtonLabel`).
pub fn format_mcp_button_label(status: McpStatusKind, name: &str) -> String {
    truncate_catalog_label(&format!("{} {name}", mcp_status_emoji(status)))
}

/// Index of an `mcps:select:<n>` callback (TS: `parseMcpSelectCallback`).
pub fn parse_mcp_select_callback(data: &str) -> Option<u64> {
    let raw = data.strip_prefix(MCPS_CALLBACK_SELECT_PREFIX)?;
    parse_js_number_page(raw)
}

/// Toggle-button label behind the detail keyboard: `connected` offers
/// disable, `disabled`/`failed` offer enable, anything else offers no
/// toggle row (TS: `buildMcpsDetailKeyboard`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpToggleLabel {
    Disable,
    Enable,
}

/// Which toggle (if any) the detail keyboard shows for a status.
pub fn mcp_detail_toggle_label(status: McpStatusKind) -> Option<McpToggleLabel> {
    match status {
        McpStatusKind::Connected => Some(McpToggleLabel::Disable),
        McpStatusKind::Disabled | McpStatusKind::Failed => Some(McpToggleLabel::Enable),
        McpStatusKind::NeedsAuth
        | McpStatusKind::NeedsClientRegistration
        | McpStatusKind::Unknown => None,
    }
}

/// Callback-data rows of the detail keyboard: with a toggle the first row
/// holds `[toggle]` and the second `[back, cancel]`; without one the only
/// row holds `[back, cancel]` (TS: `buildMcpsDetailKeyboard` layout).
pub fn mcp_detail_callback_rows(status: McpStatusKind) -> Vec<Vec<&'static str>> {
    if mcp_detail_toggle_label(status).is_some() {
        vec![
            vec![MCPS_CALLBACK_TOGGLE],
            vec![MCPS_CALLBACK_BACK, MCPS_CALLBACK_CANCEL],
        ]
    } else {
        vec![vec![MCPS_CALLBACK_BACK, MCPS_CALLBACK_CANCEL]]
    }
}

/// Only `failed` / `needs_client_registration` render the error line
/// (TS: the `detail.error` gate in `buildMcpsDetailText`).
pub fn mcp_detail_shows_error(status: McpStatusKind) -> bool {
    matches!(
        status,
        McpStatusKind::Failed | McpStatusKind::NeedsClientRegistration
    )
}

/// Only `needs_auth` / `needs_client_registration` render the
/// auth-required notice (TS: the `auth_required` gate).
pub fn mcp_detail_shows_auth_notice(status: McpStatusKind) -> bool {
    matches!(
        status,
        McpStatusKind::NeedsAuth | McpStatusKind::NeedsClientRegistration
    )
}

/// `title\n\nstatus[\nerror][\n\nauth]` detail text over the already
/// translated lines (TS: `buildMcpsDetailText` join; `error_line` /
/// `auth_notice_line` are `None` when their gates are closed).
pub fn build_mcp_detail_text(
    title_line: &str,
    status_line: &str,
    error_line: Option<&str>,
    auth_notice_line: Option<&str>,
) -> String {
    let mut lines: Vec<&str> = vec![title_line, "", status_line];
    if let Some(error) = error_line {
        lines.push(error);
    }
    if let Some(auth) = auth_notice_line {
        lines.push("");
        lines.push(auth);
    }
    lines.join("\n")
}

// ---------------------------------------------------------------------------
// Settings menu (`settings-menu.ts`)
// ---------------------------------------------------------------------------

/// `settings:` callback prefix.
pub const SETTINGS_CALLBACK_PREFIX: &str = "settings:";
/// `settings:compact_output` callback data.
pub const SETTINGS_COMPACT_OUTPUT_CALLBACK: &str = "settings:compact_output";
/// `settings:delete_progress_on_finish` callback data.
pub const SETTINGS_DELETE_PROGRESS_ON_FINISH_CALLBACK: &str = "settings:delete_progress_on_finish";
/// `settings:thinking_content` callback data.
pub const SETTINGS_THINKING_CONTENT_CALLBACK: &str = "settings:thinking_content";
/// `settings:response_streaming` callback data.
pub const SETTINGS_RESPONSE_STREAMING_CALLBACK: &str = "settings:response_streaming";
/// `settings:diff_files` callback data.
pub const SETTINGS_DIFF_FILES_CALLBACK: &str = "settings:diff_files";
/// `settings:assistant_footer` callback data.
pub const SETTINGS_ASSISTANT_FOOTER_CALLBACK: &str = "settings:assistant_footer";
/// `settings:pin_session_dashboard` callback data.
pub const SETTINGS_PIN_SESSION_DASHBOARD_CALLBACK: &str = "settings:pin_session_dashboard";
/// `settings:background_notifications` callback data.
pub const SETTINGS_BACKGROUND_NOTIFICATIONS_CALLBACK: &str = "settings:background_notifications";
/// `settings:tts` callback data.
pub const SETTINGS_TTS_CALLBACK: &str = "settings:tts";
/// `settings:prompt_queue` callback data.
pub const SETTINGS_PROMPT_QUEUE_CALLBACK: &str = "settings:prompt_queue";

/// One of the ten settings toggle callbacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsToggle {
    CompactOutput,
    DeleteProgressOnFinish,
    ThinkingContent,
    ResponseStreaming,
    DiffFiles,
    AssistantFooter,
    PinSessionDashboard,
    BackgroundNotifications,
    Tts,
    PromptQueue,
}

/// Which settings toggle a callback payload presses; `None` for anything
/// outside `settings:` (TS: the settings callback-handler dispatch).
pub fn classify_settings_callback(data: &str) -> Option<SettingsToggle> {
    match data {
        SETTINGS_COMPACT_OUTPUT_CALLBACK => Some(SettingsToggle::CompactOutput),
        SETTINGS_DELETE_PROGRESS_ON_FINISH_CALLBACK => Some(SettingsToggle::DeleteProgressOnFinish),
        SETTINGS_THINKING_CONTENT_CALLBACK => Some(SettingsToggle::ThinkingContent),
        SETTINGS_RESPONSE_STREAMING_CALLBACK => Some(SettingsToggle::ResponseStreaming),
        SETTINGS_DIFF_FILES_CALLBACK => Some(SettingsToggle::DiffFiles),
        SETTINGS_ASSISTANT_FOOTER_CALLBACK => Some(SettingsToggle::AssistantFooter),
        SETTINGS_PIN_SESSION_DASHBOARD_CALLBACK => Some(SettingsToggle::PinSessionDashboard),
        SETTINGS_BACKGROUND_NOTIFICATIONS_CALLBACK => Some(SettingsToggle::BackgroundNotifications),
        SETTINGS_TTS_CALLBACK => Some(SettingsToggle::Tts),
        SETTINGS_PROMPT_QUEUE_CALLBACK => Some(SettingsToggle::PromptQueue),
        _ => None,
    }
}

/// `true` for any `settings:*` payload (TS: the `startsWith` guard).
pub fn is_settings_callback(data: &str) -> bool {
    data.starts_with(SETTINGS_CALLBACK_PREFIX)
}

/// On/off display value (TS: `formatBooleanSettingValue` over the
/// `settings.value.on` / `settings.value.off` labels).
pub fn format_boolean_setting_value<'a>(
    enabled: bool,
    on_label: &'a str,
    off_label: &'a str,
) -> &'a str {
    if enabled {
        on_label
    } else {
        off_label
    }
}

/// TTS mode behind the value line (TS: `TtsMode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TtsMode {
    All,
    Auto,
    Off,
}

/// `all` -> all label, `auto` -> auto label, anything else -> off label
/// (TS: `formatTtsModeValue`).
pub fn format_tts_mode_value<'a>(
    mode: TtsMode,
    all_label: &'a str,
    auto_label: &'a str,
    off_label: &'a str,
) -> &'a str {
    match mode {
        TtsMode::All => all_label,
        TtsMode::Auto => auto_label,
        TtsMode::Off => off_label,
    }
}

/// Prompt-queue mode behind the value line (TS: `PromptQueueMode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptQueueMode {
    Queue,
    Steer,
    Off,
}

/// V1 names only on/off (`mode !== "off"` reads as on); V2 names
/// queue/steer/off (TS: `formatPromptQueueModeValue`).
pub fn format_prompt_queue_mode_value<'a>(
    server_is_v2: bool,
    mode: PromptQueueMode,
    queue_label: &'a str,
    steer_label: &'a str,
    on_label: &'a str,
    off_label: &'a str,
) -> &'a str {
    if !server_is_v2 {
        return if mode == PromptQueueMode::Off {
            off_label
        } else {
            on_label
        };
    }
    match mode {
        PromptQueueMode::Queue => queue_label,
        PromptQueueMode::Steer => steer_label,
        PromptQueueMode::Off => off_label,
    }
}

/// Response-streaming mode behind the value line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseStreamingMode {
    Draft,
    Edit,
}

/// `draft` -> draft label, anything else -> edit label
/// (TS: `formatResponseStreamingModeValue`).
pub fn format_response_streaming_mode_value<'a>(
    mode: ResponseStreamingMode,
    draft_label: &'a str,
    edit_label: &'a str,
) -> &'a str {
    match mode {
        ResponseStreamingMode::Draft => draft_label,
        ResponseStreamingMode::Edit => edit_label,
    }
}

/// Ordered callback-data rows of the settings menu: the compact-output row
/// always comes first, then either the delete-progress row (compact mode)
/// or the thinking + diff-files rows, then the streaming/footer/pin/
/// background/tts/queue rows (TS: `buildSettingsMenuView` layout).
pub fn settings_menu_row_callbacks(compact_output: bool) -> Vec<&'static str> {
    let mut rows = vec![SETTINGS_COMPACT_OUTPUT_CALLBACK];
    if compact_output {
        rows.push(SETTINGS_DELETE_PROGRESS_ON_FINISH_CALLBACK);
    } else {
        rows.push(SETTINGS_THINKING_CONTENT_CALLBACK);
        rows.push(SETTINGS_DIFF_FILES_CALLBACK);
    }
    rows.extend([
        SETTINGS_RESPONSE_STREAMING_CALLBACK,
        SETTINGS_ASSISTANT_FOOTER_CALLBACK,
        SETTINGS_PIN_SESSION_DASHBOARD_CALLBACK,
        SETTINGS_BACKGROUND_NOTIFICATIONS_CALLBACK,
        SETTINGS_TTS_CALLBACK,
        SETTINGS_PROMPT_QUEUE_CALLBACK,
    ]);
    rows
}

/// Render one `label: value` settings row (TS: the `` `${label}: ${value}` ``
/// composition in `buildSettingsMenuView`).
pub fn format_settings_row(label: &str, value: &str) -> String {
    let mut out = String::new();
    let _ = write!(out, "{label}: {value}");
    out
}
