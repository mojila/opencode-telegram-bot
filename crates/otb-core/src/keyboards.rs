//! Reply/inline keyboard + message-pattern + render-remainder pure logic (slice 28b, RED).
//!
//! Rust port of the pure label/layout/gating logic in:
//! - `src/bot/keyboards/queued-prompt-button.ts` (28-char preview,
//!   `formatQueuedPromptButtonLabel`, label list/find over a queue snapshot)
//! - `src/bot/keyboards/main-reply-keyboard.ts` (token counts, context line,
//!   fixed 2x2 grid with queued rows on top and one goal row above it,
//!   agent keyboard, remove payload)
//! - `src/bot/keyboards/goal-button.ts` (`GOAL_BUTTON_TEXT_PATTERN`,
//!   status -> action labels, label -> action resolve)
//! - `src/bot/message-patterns.ts` (agent/model/variant/context/queued/goal
//!   patterns + `isReplyKeyboardButtonText`)
//! - `src/bot/menus/model-selection-menu.ts` (list/provider callback
//!   build+parse, pagination range, option labels, empty gating)
//! - `src/bot/menus/agent-selection-menu.ts` (option label + callback)
//! - `src/bot/menus/variant-selection-menu.ts` (display format, option
//!   label + callback, disabled filtering, after-change show gating)
//! - `src/bot/menus/permission-menu.ts` (emoji/name lookup, outcome lines,
//!   budgeted pattern text with safe-boundary cut, button layout)
//! - `src/bot/menus/question-menu.ts` (60-char button text, keyboard rows +
//!   callbacks with submit/custom gating, fallback text, answers summary,
//!   select-callback parse, gone-request tags)
//! - render remainder: `src/bot/render/block-plain-text.ts`
//!   (inline plain text, aligned table text), `src/bot/render/text-splitter.ts`
//!   (`splitTextIntoChunks`), `src/bot/render/block-splitter.ts`
//!   (`normalizeBlockSplitLimits`), `src/bot/render/limits.ts` (budgets)
//!
//! Pure ports: no grammy/Telegram I/O lives here. Keyboards are modelled as
//! `Vec<Vec<String>>` rows (reply) or `Vec<Vec<InlineButton>>` (inline).

// ---------------------------------------------------------------------------
// Shared inline-button shape
// ---------------------------------------------------------------------------

/// One inline-keyboard button: visible text plus callback data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineButton {
    pub text: String,
    pub callback_data: String,
}

// ---------------------------------------------------------------------------
// Queued-prompt button (`src/bot/keyboards/queued-prompt-button.ts`)
// ---------------------------------------------------------------------------

/// Preview budget for a queued-prompt button label
/// (`QUEUED_PROMPT_PREVIEW_MAX_LENGTH`).
pub const QUEUED_PROMPT_PREVIEW_MAX_LENGTH: usize = 28;

/// Collapse whitespace/newlines and cut to the preview budget with `…`
/// (TS: `buildPreview`).
pub fn build_queued_prompt_preview(text: &str) -> String {
    let collapsed: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.chars().count() <= QUEUED_PROMPT_PREVIEW_MAX_LENGTH {
        return collapsed;
    }
    let preview: String = collapsed
        .chars()
        .take(QUEUED_PROMPT_PREVIEW_MAX_LENGTH - 1)
        .collect();
    format!("{preview}…")
}

/// Reply-keyboard label for a queued prompt, 1-based
/// (TS: `formatQueuedPromptButtonLabel`, i18n `keyboard.queued_prompt`).
pub fn format_queued_prompt_button_label(index: usize, text: &str) -> String {
    format!("❌ {index}. {}", build_queued_prompt_preview(text))
}

/// Labels for a queue snapshot, in order (TS: `getQueuedPromptButtonLabels`).
pub fn queued_prompt_button_labels(display_texts: &[String]) -> Vec<String> {
    display_texts
        .iter()
        .enumerate()
        .map(|(i, text)| format_queued_prompt_button_label(i + 1, text))
        .collect()
}

/// Position of the queue entry behind a button label, `None` when the label
/// no longer matches (TS: `findQueuedPromptByButtonLabel`).
pub fn find_queued_prompt_index_by_label(
    display_texts: &[String],
    label: &str,
) -> Option<usize> {
    display_texts
        .iter()
        .enumerate()
        .find(|(i, text)| format_queued_prompt_button_label(i + 1, text) == label)
        .map(|(i, _)| i)
}

// ---------------------------------------------------------------------------
// Main reply keyboard (`src/bot/keyboards/main-reply-keyboard.ts`)
// ---------------------------------------------------------------------------

/// Empty-context button label (i18n `keyboard.context_empty`).
pub const CONTEXT_EMPTY_LABEL: &str = "📊 0";

/// Default variant button label (i18n `keyboard.variant_default`).
pub const VARIANT_DEFAULT_LABEL: &str = "💡 Default";

/// Token usage snapshot for the context button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContextInfo {
    pub tokens_used: u64,
    pub tokens_limit: u64,
}

/// Compact token count: `150000 -> "150K"`, `1500000 -> "1.5M"`
/// (TS: `formatTokenCount`).
pub fn format_token_count(count: u64) -> String {
    if count >= 1_000_000 {
        format!("{:.1}M", count as f64 / 1_000_000.0)
    } else if count >= 1000 {
        format!("{}K", (count as f64 / 1000.0).round() as u64)
    } else {
        count.to_string()
    }
}

/// Context button line with percent (TS: `formatContextForButton`,
/// i18n `keyboard.context`).
pub fn format_context_for_button(info: &ContextInfo) -> String {
    let used = format_token_count(info.tokens_used);
    let limit = format_token_count(info.tokens_limit);
    let percent = if info.tokens_limit == 0 {
        0
    } else {
        ((info.tokens_used as f64 / info.tokens_limit as f64) * 100.0).round() as u64
    };
    format!("📊 {used} / {limit} ({percent}%)")
}

/// Emoji for an agent name, `🤖` fallback (TS: `getAgentEmoji`).
fn agent_emoji(agent_name: &str) -> &str {
    match agent_name {
        "plan" => "📋",
        "build" => "🛠️",
        "general" => "💬",
        "explore" => "🔍",
        "title" => "📝",
        "summary" => "📄",
        "compaction" => "📦",
        _ => "🤖",
    }
}

/// Capitalize the first character (TS: display-name capitalization).
fn capitalize_first(name: &str) -> String {
    let mut chars = name.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

/// Reply-keyboard label for an agent button (TS: `getAgentButtonLabel`).
pub fn agent_button_label(agent_name: &str) -> String {
    format!("{} {} Agent", agent_emoji(agent_name), capitalize_first(agent_name))
}

/// Truncate a display part on a char boundary with `...` (TS: `substring` cut).
fn truncate_display_part(part: &str, max_len: usize, keep: usize) -> String {
    if part.chars().count() <= max_len {
        return part.to_string();
    }
    format!("{}...", part.chars().take(keep).collect::<String>())
}

/// Compact `provider/model` button text, truncating long parts
/// (TS: `formatModelForButton`).
pub fn format_model_for_button(provider_id: &str, model_id: &str) -> String {
    let display_model_id = truncate_display_part(model_id, 20, 17);
    let display_provider_id = truncate_display_part(provider_id, 15, 12);
    format!("🧠 {display_provider_id}\n{display_model_id}")
}

/// Reply keyboard as rows of button labels plus flags
/// (TS: grammy `Keyboard` with `.resized().persistent()`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MainKeyboard {
    pub rows: Vec<Vec<String>>,
    pub resized: bool,
    pub persistent: bool,
}

/// Fixed 2x2 grid with queued rows on top and one goal row above the grid
/// (TS: `createMainKeyboard`).
pub fn create_main_keyboard(
    agent_name: &str,
    provider_id: &str,
    model_id: &str,
    context: Option<&ContextInfo>,
    variant_name: Option<&str>,
    queued_labels: &[String],
    goal_labels: &[String],
) -> MainKeyboard {
    let mut rows: Vec<Vec<String>> = Vec::new();
    for label in queued_labels {
        rows.push(vec![label.clone()]);
    }
    if !goal_labels.is_empty() {
        rows.push(goal_labels.to_vec());
    }
    let context_text = context
        .map(format_context_for_button)
        .unwrap_or_else(|| CONTEXT_EMPTY_LABEL.to_string());
    rows.push(vec![agent_button_label(agent_name), context_text]);
    rows.push(vec![
        format_model_for_button(provider_id, model_id),
        variant_name
            .map(str::to_string)
            .unwrap_or_else(|| VARIANT_DEFAULT_LABEL.to_string()),
    ]);
    MainKeyboard { rows, resized: true, persistent: true }
}

/// Single-button agent keyboard (TS: `createAgentKeyboard`, deprecated).
pub fn create_agent_keyboard(agent_name: &str) -> MainKeyboard {
    MainKeyboard {
        rows: vec![vec![agent_button_label(agent_name)]],
        resized: true,
        persistent: true,
    }
}

/// Remove-keyboard payload marker (TS: `removeKeyboard`).
pub fn is_remove_keyboard_payload(remove_keyboard: bool) -> bool {
    remove_keyboard
}

// ---------------------------------------------------------------------------
// Goal button (`src/bot/keyboards/goal-button.ts`)
// ---------------------------------------------------------------------------

/// Goal lifecycle status relevant to the button row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoalStatus {
    Active,
    Paused,
    Complete,
}

/// Whether text looks like a goal button (TS: `GOAL_BUTTON_TEXT_PATTERN`).
pub fn matches_goal_button_text(text: &str) -> bool {
    let mut chars = text.chars();
    match chars.next() {
        Some(marker @ ('◎' | '⏸' | '▶' | '×')) => {
            let _ = marker;
            match chars.next() {
                Some(next) if next.is_whitespace() => chars.next().is_some(),
                _ => false,
            }
        }
        _ => false,
    }
}

/// Details-row action derived from a goal status: active -> pause,
/// paused -> resume, else clear (TS: `getGoalButtonLabels` action pick).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoalButtonAction {
    Details,
    Pause,
    Resume,
    Clear,
}

/// Flatten whitespace for a goal objective (TS: `flattenGoalObjective`).
fn flatten_goal_objective(objective: &str) -> String {
    objective.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Status word for a goal status (TS: `formatGoalStatus`).
fn goal_status_word(status: GoalStatus) -> &'static str {
    match status {
        GoalStatus::Active => "active",
        GoalStatus::Paused => "paused",
        GoalStatus::Complete => "complete",
    }
}

/// Button labels for a goal snapshot: `[details, action]` or `[]` when no
/// goal/session (TS: `getGoalButtonLabels`).
pub fn get_goal_button_labels(status: Option<GoalStatus>, objective: &str) -> Vec<String> {
    let Some(status) = status else { return Vec::new() };
    let flat = flatten_goal_objective(objective);
    let short = if flat.chars().count() <= 28 {
        flat
    } else {
        let cut: String = flat.chars().take(27).collect();
        format!("{}…", cut.trim_end())
    };
    let details = format!("◎ {} · {short}", goal_status_word(status));
    let action = match status {
        GoalStatus::Active => "⏸ Pause",
        GoalStatus::Paused => "▶ Continue",
        GoalStatus::Complete => "× Delete",
    };
    vec![details, action.to_string()]
}

/// Resolve a pressed label back to its action (TS: `resolveGoalButton`).
pub fn resolve_goal_button(
    status: Option<GoalStatus>,
    labels: &[String],
    label: &str,
) -> Option<GoalButtonAction> {
    if labels.first().is_some_and(|first| first == label) {
        return Some(GoalButtonAction::Details);
    }
    if !labels.get(1).is_some_and(|second| second == label) {
        return None;
    }
    Some(match status {
        Some(GoalStatus::Active) => GoalButtonAction::Pause,
        Some(GoalStatus::Paused) => GoalButtonAction::Resume,
        _ => GoalButtonAction::Clear,
    })
}

// ---------------------------------------------------------------------------
// Message patterns (`src/bot/message-patterns.ts`)
// ---------------------------------------------------------------------------

/// Agent-mode button emoji prefixes (TS: `AGENT_MODE_BUTTON_TEXT_PATTERN`).
const AGENT_MODE_EMOJIS: [&str; 8] = ["📋", "🛠️", "💬", "🔍", "📝", "📄", "📦", "🤖"];

/// Whether text looks like an agent-mode button
/// (TS: `AGENT_MODE_BUTTON_TEXT_PATTERN`).
pub fn is_agent_mode_button_text(text: &str) -> bool {
    for emoji in AGENT_MODE_EMOJIS {
        if let Some(rest) = text.strip_prefix(emoji) {
            // Emoji must be followed by whitespace, end with ` Mode`/` Agent`,
            // and carry a non-empty name in between.
            if !rest.starts_with(char::is_whitespace) {
                return false;
            }
            if rest.ends_with(" Mode") || rest.ends_with(" Agent") {
                let middle = rest.trim();
                let name_len = middle
                    .strip_suffix("Mode")
                    .or_else(|| middle.strip_suffix("Agent"))
                    .map(|s| s.trim().len())
                    .unwrap_or(0);
                return name_len > 0;
            }
            return false;
        }
    }
    false
}

/// Whether text looks like a model button (TS: `MODEL_BUTTON_TEXT_PATTERN`).
pub fn is_model_button_text(text: &str) -> bool {
    let Some(rest) = text.strip_prefix("🧠") else { return false };
    if !rest.starts_with(char::is_whitespace) {
        return false;
    }
    let body = rest.trim();
    if body.is_empty() {
        return false;
    }
    // A trailing ` Mode`/` Agent` word marks an agent button, not a model one.
    !(body.ends_with(" Mode") || body.ends_with(" Agent"))
}

/// Whether text looks like a variant button, current `💡` or legacy `💭`
/// (TS: `VARIANT_BUTTON_TEXT_PATTERN`).
pub fn is_variant_button_text(text: &str) -> bool {
    for emoji in ["💡", "💭"] {
        if let Some(rest) = text.strip_prefix(emoji) {
            return rest.starts_with(char::is_whitespace) && !rest.trim().is_empty();
        }
    }
    false
}

/// Whether text looks like the context button
/// (TS: `CONTEXT_BUTTON_TEXT_PATTERN`).
pub fn is_context_button_text(text: &str) -> bool {
    match text.strip_prefix("📊") {
        Some(rest) => rest.is_empty() || rest.starts_with(char::is_whitespace),
        None => false,
    }
}

/// Whether text looks like a queued-prompt button
/// (TS: `QUEUED_PROMPT_BUTTON_TEXT_PATTERN`).
pub fn is_queued_prompt_button_text(text: &str) -> bool {
    let Some(rest) = text.strip_prefix("❌") else { return false };
    let rest = rest.strip_prefix(char::is_whitespace).unwrap_or(rest);
    // `❌` must be followed by whitespace before the digits.
    if rest.len() == text.len() - "❌".len() {
        return false;
    }
    let digits = rest.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits == 0 {
        return false;
    }
    let after_digits = &rest[digits..];
    after_digits.starts_with('.')
        && after_digits[1..].starts_with(char::is_whitespace)
        && after_digits[1..].trim().len() > 0
}

/// Whether the text is any reply-keyboard button rather than a typed prompt
/// (TS: `isReplyKeyboardButtonText`).
pub fn is_reply_keyboard_button_text(text: &str) -> bool {
    is_agent_mode_button_text(text)
        || is_model_button_text(text)
        || is_variant_button_text(text)
        || is_context_button_text(text)
        || is_queued_prompt_button_text(text)
        || matches_goal_button_text(text)
}

// ---------------------------------------------------------------------------
// Model selection menu (`src/bot/menus/model-selection-menu.ts`)
// ---------------------------------------------------------------------------

/// Which saved list a `model:list:` callback points at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelListKind {
    Favorites,
    Recent,
}

/// Build a `model:list:<kind>:<index>` callback (TS: `buildModelListCallback`).
pub fn build_model_list_callback(kind: ModelListKind, index: usize) -> String {
    let kind_str = match kind {
        ModelListKind::Favorites => "favorites",
        ModelListKind::Recent => "recent",
    };
    format!("model:list:{kind_str}:{index}")
}

/// Parse an ASCII digits value, `None` for anything else (TS: `parseIndex`).
fn parse_callback_index(value: &str) -> Option<u64> {
    if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    value.parse::<u64>().ok()
}

/// Parse a `model:providers:<page>` callback (TS: `parseProvidersPageCallback`).
pub fn parse_providers_page_callback(data: &str) -> Option<u64> {
    parse_callback_index(data.strip_prefix("model:providers:")?)
}

/// Parsed `model:provider:<providerIndex>:<page>` callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderCallback {
    pub provider_index: usize,
    pub page: usize,
}

/// Parse a provider callback (TS: `parseProviderCallback`).
pub fn parse_provider_callback(data: &str) -> Option<ProviderCallback> {
    let rest = data.strip_prefix("model:provider:")?;
    let (index_text, page_text) = rest.split_once(':')?;
    if page_text.contains(':') {
        return None;
    }
    let provider_index = parse_callback_index(index_text)? as usize;
    let page = parse_callback_index(page_text)? as usize;
    Some(ProviderCallback { provider_index, page })
}

/// Parse a `model:pick:<index>` callback (TS: `parseProviderModelCallback`).
pub fn parse_provider_model_callback(data: &str) -> Option<usize> {
    parse_callback_index(data.strip_prefix("model:pick:")?).map(|v| v as usize)
}

/// Clamped pagination window (TS: `calculateModelsPaginationRange`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaginationRange {
    pub page: usize,
    pub total_pages: usize,
    pub start_index: usize,
    pub end_index: usize,
}

/// Clamp `page`/`pageSize` into a window over `total_items`.
pub fn calculate_models_pagination_range(
    total_items: usize,
    page: i64,
    page_size: usize,
) -> PaginationRange {
    let safe_page_size = page_size.max(1);
    let total_pages = ((total_items + safe_page_size - 1) / safe_page_size).max(1);
    let normalized_page = (page.max(0) as usize).min(total_pages - 1);
    let start_index = normalized_page * safe_page_size;
    let end_index = (start_index + safe_page_size).min(total_items);
    PaginationRange { page: normalized_page, total_pages, start_index, end_index }
}

/// Saved-model option label with `⭐`/`🕘` prefix and `✅` when active
/// (TS: `buildModelSelectionMenu` `addButton`).
pub fn model_option_label(
    provider_id: &str,
    model_id: &str,
    prefix: &str,
    is_active: bool,
) -> String {
    let label = format!("{prefix} {provider_id}/{model_id}");
    if is_active { format!("✅ {label}") } else { label }
}

/// Provider option label (TS: `buildProvidersMenuView` label).
pub fn provider_option_label(name: &str, model_count: usize) -> String {
    format!("{name} ({model_count})")
}

/// Whether the menu has any choice to show (TS: favorites+recent empty gate).
pub fn model_menu_has_choices(favorites: usize, recent: usize) -> bool {
    favorites > 0 || recent > 0
}

// ---------------------------------------------------------------------------
// Agent selection menu (`src/bot/menus/agent-selection-menu.ts`)
// ---------------------------------------------------------------------------

/// Agent option label with `✅` when current (TS: `buildAgentSelectionMenu`).
pub fn agent_option_label(display_name: &str, is_active: bool) -> String {
    if is_active { format!("✅ {display_name}") } else { display_name.to_string() }
}

/// Agent option callback `agent:<name>` (TS: `buildAgentSelectionMenu`).
pub fn agent_option_callback(name: &str) -> String {
    format!("agent:{name}")
}

// ---------------------------------------------------------------------------
// Variant selection menu (`src/bot/menus/variant-selection-menu.ts`)
// ---------------------------------------------------------------------------

/// Capitalized display form of a variant id (TS: `formatVariantForDisplay`).
pub fn format_variant_for_display(variant_id: &str) -> String {
    capitalize_first(variant_id)
}

/// Variant option label with `✅` when current (TS: `buildVariantSelectionMenu`).
pub fn variant_option_label(variant_id: &str, is_active: bool) -> String {
    let display = format_variant_for_display(variant_id);
    if is_active { format!("✅ {display}") } else { display }
}

/// Variant option callback `variant:<id>`.
pub fn variant_option_callback(variant_id: &str) -> String {
    format!("variant:{variant_id}")
}

/// Variant entry for visibility gating.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariantEntry {
    pub id: String,
    pub disabled: bool,
}

/// Ids of selectable (non-disabled) variants in order.
pub fn visible_variant_ids(variants: &[VariantEntry]) -> Vec<String> {
    variants
        .iter()
        .filter(|v| !v.disabled)
        .map(|v| v.id.clone())
        .collect()
}

/// Whether the menu opens after a model change: only with 2+ drawn rows
/// (TS: `showVariantSelectionMenuAfterModelChange` `drawnVariants < 2` gate).
pub fn should_show_variant_menu_after_change(variants: &[VariantEntry]) -> bool {
    visible_variant_ids(variants).len() >= 2
}

// ---------------------------------------------------------------------------
// Permission menu (`src/bot/menus/permission-menu.ts`)
// ---------------------------------------------------------------------------

/// Telegram message budget applied to permission prompts.
pub const PERMISSION_MESSAGE_LIMIT: usize = 4096;

/// Pattern-list bullet prefix (TS: `PATTERN_PREFIX`).
const PATTERN_PREFIX: &str = "• ";
/// Cut marker closing a shortened pattern list (TS: `CUT_SUFFIX`).
const CUT_SUFFIX: &str = "…\n";

/// Emoji for a permission kind, `🔐` fallback (TS: `PERMISSION_EMOJIS`).
pub fn permission_emoji(permission: &str) -> &str {
    match permission {
        "bash" => "⚡",
        "edit" => "✏️",
        "write" => "📝",
        "read" => "📖",
        "webfetch" => "🌐",
        "websearch" => "🔍",
        "glob" => "📁",
        "grep" => "🔎",
        "list" => "📂",
        "task" => "⚙️",
        "lsp" => "🔧",
        "external_directory" => "📁",
        _ => "🔐",
    }
}

/// Display name for a permission kind, falling back to the raw id
/// (TS: `PERMISSION_NAME_KEYS` lookup in `formatPermissionText`).
pub fn permission_display_name(permission: &str) -> String {
    match permission {
        "bash" => "Bash",
        "edit" => "Edit",
        "write" => "Write",
        "read" => "Read",
        "webfetch" => "Web Fetch",
        "websearch" => "Web Search",
        "glob" => "File Search",
        "grep" => "Content Search",
        "list" => "List Directory",
        "task" => "Task",
        "lsp" => "LSP",
        "external_directory" => "External Directory",
        _ => permission,
    }
    .to_string()
}

/// Permission reply kind (TS: `PermissionReply`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionReply {
    Once,
    Always,
    Reject,
}

/// Status line closing a prompt (TS: `formatOutcomeLine`).
pub fn format_permission_outcome_line(
    reply: PermissionReply,
    answered_outside: bool,
) -> String {
    let line = match reply {
        PermissionReply::Once => "✅ Allowed once",
        PermissionReply::Always => "🔓 Allowed always",
        PermissionReply::Reject => "❌ Rejected",
    };
    if answered_outside {
        format!("{line} · answered outside Telegram")
    } else {
        line.to_string()
    }
}

/// Full prompt text with budgeted pattern list and optional status line
/// (TS: `formatPermissionText`).
pub fn format_permission_text(
    emoji: &str,
    name: &str,
    patterns: &[String],
    grouped_count: usize,
    status_line: Option<&str>,
) -> String {
    let header = format!("{emoji} Permission request: {name}\n\n");
    let grouped = if grouped_count > 1 {
        format!("\n⚠️ {grouped_count} identical requests pending — your answer applies to all of them.\n")
    } else {
        String::new()
    };
    let status = status_line.map(|s| format!("\n{s}")).unwrap_or_default();

    // Show patterns (commands/files); cut when the text would not fit.
    let cut_line = format!("{PATTERN_PREFIX}{CUT_SUFFIX}");
    let mut budget = PERMISSION_MESSAGE_LIMIT
        .saturating_sub(header.len())
        .saturating_sub(grouped.len())
        .saturating_sub(status.len());
    let mut rendered = String::new();
    for (index, pattern) in patterns.iter().enumerate() {
        let line = format!("{PATTERN_PREFIX}{pattern}\n");
        let reserve = if index == patterns.len() - 1 { 0 } else { cut_line.len() };
        if line.len() + reserve <= budget {
            budget -= line.len();
            rendered.push_str(&line);
            continue;
        }
        // The pattern that does not fit is shortened, so the user still
        // sees what is asked.
        let room = budget.saturating_sub(cut_line.len());
        if room > 0 {
            rendered.push_str(&format!(
                "{PATTERN_PREFIX}{}{CUT_SUFFIX}",
                slice_on_safe_boundary(pattern, room)
            ));
        } else {
            rendered.push_str(&cut_line);
        }
        break;
    }

    format!("{header}{rendered}{grouped}{status}")
}

/// Cut `text` to `max_length` without splitting a surrogate pair
/// (TS: `sliceOnSafeBoundary`).
pub fn slice_on_safe_boundary(text: &str, max_length: usize) -> String {
    let mut end = text.len().min(max_length);
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_string()
}

/// Inline keyboard for a permission prompt: allow/always/reject rows
/// (TS: `buildPermissionKeyboard`).
pub fn build_permission_keyboard() -> Vec<Vec<InlineButton>> {
    vec![
        vec![InlineButton {
            text: "✅ Allow once".to_string(),
            callback_data: "permission:once".to_string(),
        }],
        vec![InlineButton {
            text: "🔓 Allow always".to_string(),
            callback_data: "permission:always".to_string(),
        }],
        vec![InlineButton {
            text: "❌ Reject".to_string(),
            callback_data: "permission:reject".to_string(),
        }],
    ]
}

// ---------------------------------------------------------------------------
// Question menu (`src/bot/menus/question-menu.ts`)
// ---------------------------------------------------------------------------

/// Button label budget (TS: `MAX_BUTTON_LENGTH`).
pub const MAX_QUESTION_BUTTON_LENGTH: usize = 60;

/// Truncate a button label with `...` and prepend a selection icon
/// (TS: `formatButtonText`).
pub fn format_question_button_text(label: &str, is_selected: bool) -> String {
    let icon = if is_selected { "✅ " } else { "" };
    let text = format!("{icon}{label}");
    if text.chars().count() > MAX_QUESTION_BUTTON_LENGTH {
        format!("{}...", text.chars().take(MAX_QUESTION_BUTTON_LENGTH - 3).collect::<String>())
    } else {
        text
    }
}

/// Inputs controlling question keyboard rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionKeyboardSpec {
    pub question_index: usize,
    pub option_count: usize,
    pub multiple: bool,
    /// `None` mirrors `custom === undefined` (button shown).
    pub custom: Option<bool>,
    pub selected: Vec<usize>,
    pub has_custom_answer: bool,
    pub custom_answer_selected: bool,
    pub custom_answer_text: String,
}

/// Collapse `␣*\n␣*` runs to a single space (TS: `customAnswer.replace`).
fn collapse_custom_answer(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\n' {
            while out.ends_with(' ') {
                out.pop();
            }
            if !out.is_empty() {
                out.push(' ');
            }
            while chars.peek().is_some_and(|n| n.is_whitespace()) {
                chars.next();
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Keyboard rows: one row per option, optional custom-answer row, submit iff
/// multiple, custom iff no options or `custom !== false`, then cancel
/// (TS: `buildQuestionKeyboard`).
pub fn build_question_keyboard_rows(spec: &QuestionKeyboardSpec) -> Vec<Vec<InlineButton>> {
    let mut rows: Vec<Vec<InlineButton>> = Vec::new();
    for index in 0..spec.option_count {
        let is_selected = spec.selected.contains(&index);
        rows.push(vec![InlineButton {
            text: format_question_button_text("", is_selected),
            callback_data: format!("question:select:{}:{index}", spec.question_index),
        }]);
    }
    if spec.multiple && spec.has_custom_answer {
        let collapsed = collapse_custom_answer(&spec.custom_answer_text);
        rows.push(vec![InlineButton {
            text: format_question_button_text(
                &format!("✏️ {collapsed}"),
                spec.custom_answer_selected,
            ),
            callback_data: format!("question:toggle_custom:{}", spec.question_index),
        }]);
    }
    if spec.multiple {
        rows.push(vec![InlineButton {
            text: "✅ Done".to_string(),
            callback_data: format!("question:submit:{}", spec.question_index),
        }]);
    }
    // A question without choices can only be answered with custom text.
    if spec.option_count == 0 || spec.custom != Some(false) {
        rows.push(vec![InlineButton {
            text: "🔤 Custom answer".to_string(),
            callback_data: format!("question:custom:{}", spec.question_index),
        }]);
    }
    rows.push(vec![InlineButton {
        text: "❌ Cancel".to_string(),
        callback_data: format!("question:cancel:{}", spec.question_index),
    }]);
    rows
}

/// Card fallback text: header, question, options, optional status line
/// (TS: `formatQuestionDetailsPart` `fallbackText`).
pub fn format_question_fallback_text(
    header: &str,
    question: &str,
    options: &[(String, String)],
    multiple: bool,
    progress: Option<(usize, usize)>,
    status_line: Option<&str>,
) -> String {
    let progress_text = match progress {
        Some((index, total)) if total > 0 => format!("{}/{}", index + 1, total),
        _ => String::new(),
    };
    let header_title = ["❓", &progress_text, header]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let multiple_hint = if multiple { "\n(You can select multiple options)" } else { "" };
    let question_text = format!("{question}{multiple_hint}");

    let mut segments: Vec<String> = Vec::new();
    if !header_title.is_empty() {
        segments.push(header_title);
    }
    if !question_text.is_empty() {
        segments.push(question_text);
    }
    for (label, description) in options {
        let rest = if description.is_empty() {
            String::new()
        } else if label.is_empty() {
            description.clone()
        } else {
            format!(" — {description}")
        };
        let segment = format!("{label}{rest}");
        if !segment.is_empty() {
            segments.push(segment);
        }
    }
    if let Some(status) = status_line {
        segments.push(status.to_string());
    }
    segments.join("\n\n")
}

/// Poll-completion summary over answered questions
/// (TS: `formatAnswersSummary`).
pub fn format_answers_summary(answers: &[(String, String)]) -> String {
    let mut summary = "✅ Poll completed!\n\n".to_string();
    for (index, (question, answer)) in answers.iter().enumerate() {
        summary.push_str(&format!("Question {}:\n{question}\n\n", index + 1));
        summary.push_str(&format!("Answer:\n{answer}\n\n"));
    }
    summary
}

/// Parse a `question:select:<q>:<o>` callback (TS: select-callback shape).
pub fn parse_question_select_callback(data: &str) -> Option<(usize, usize)> {
    let rest = data.strip_prefix("question:select:")?;
    let (q_text, o_text) = rest.split_once(':')?;
    if o_text.contains(':') {
        return None;
    }
    let q = parse_callback_index(q_text)? as usize;
    let o = parse_callback_index(o_text)? as usize;
    Some((q, o))
}

/// Whether an error tag means the request is gone
/// (TS: `isQuestionRequestGone`).
pub fn is_question_request_gone_tag(tag: &str) -> bool {
    tag == "QuestionNotFoundError" || tag == "FormAlreadySettledError"
}

// ---------------------------------------------------------------------------
// Render remainder (`src/bot/render/`)
// ---------------------------------------------------------------------------

/// Default per-part char budget (TS: `DEFAULT_MAX_PART_CHARS`).
pub const DEFAULT_MAX_PART_CHARS: usize = 32_000;

/// Default per-part block budget (TS: `DEFAULT_MAX_PART_BLOCKS`).
pub const DEFAULT_MAX_PART_BLOCKS: usize = 480;

/// Plain-text per-part budget (TS: `PLAIN_MAX_PART_CHARS`).
pub const PLAIN_MAX_PART_CHARS: usize = 3_800;

/// Telegram text message limit (TS: `TELEGRAM_TEXT_MESSAGE_LIMIT`).
pub const TELEGRAM_TEXT_MESSAGE_LIMIT: usize = 4096;

/// Minimal inline node for plain-text extraction
/// (TS: `InlineNode` subset in `block-plain-text.ts`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlainInlineNode {
    Text(String),
    Code(String),
    Styled(Vec<PlainInlineNode>),
    Link(Vec<PlainInlineNode>),
}

/// Concatenate inline content, dropping markup (TS: `extractInlinePlainText`).
pub fn extract_inline_plain_text(nodes: &[PlainInlineNode]) -> String {
    let mut result = String::new();
    for node in nodes {
        match node {
            PlainInlineNode::Text(text) | PlainInlineNode::Code(text) => result.push_str(text),
            PlainInlineNode::Styled(children) | PlainInlineNode::Link(children) => {
                result.push_str(&extract_inline_plain_text(children));
            }
        }
    }
    result
}

/// Pad columns, join with ` | `, insert a `---` divider after the header
/// (TS: `buildAlignedTableText`).
pub fn build_aligned_table_text(rows: &[Vec<String>]) -> String {
    if rows.is_empty() {
        return String::new();
    }
    let column_count = rows.iter().map(Vec::len).max().unwrap_or(0);
    let normalized: Vec<Vec<String>> = rows
        .iter()
        .map(|row| {
            (0..column_count)
                .map(|i| row.get(i).cloned().unwrap_or_default())
                .collect()
        })
        .collect();
    let widths: Vec<usize> = (0..column_count)
        .map(|i| normalized.iter().map(|row| row[i].len()).max().unwrap_or(0))
        .collect();
    let format_row = |row: &[String]| {
        row.iter()
            .enumerate()
            .map(|(i, cell)| {
                let pad = widths[i].saturating_sub(cell.len());
                format!("{cell}{}", " ".repeat(pad))
            })
            .collect::<Vec<_>>()
            .join(" | ")
    };
    let formatted: Vec<String> = normalized.iter().map(|r| format_row(r)).collect();
    if formatted.len() <= 1 {
        return formatted.join("\n");
    }
    let divider = widths
        .iter()
        .map(|w| "-".repeat(*w))
        .collect::<Vec<_>>()
        .join("-|-");
    format!("{}\n{divider}\n{}", formatted[0], formatted[1..].join("\n"))
}

/// Find the cut point within `limit` chars of `start`, preferring a line
/// break, then any whitespace, then any char boundary (TS: `findSplitBoundary`).
fn find_split_boundary(chars: &[char], start: usize, limit: usize) -> Option<usize> {
    let hard_end = (start + limit).min(chars.len());
    if hard_end >= chars.len() {
        return Some(chars.len());
    }
    let mut whitespace_boundary: Option<usize> = None;
    let mut fallback_boundary: Option<usize> = None;
    let mut index = hard_end;
    while index > start {
        if chars[index - 1] == '\n' {
            return Some(index);
        }
        if whitespace_boundary.is_none()
            && index > 0
            && chars[index - 1].is_whitespace()
        {
            whitespace_boundary = Some(index);
        }
        if fallback_boundary.is_none() {
            fallback_boundary = Some(index);
        }
        index -= 1;
    }
    whitespace_boundary.or(fallback_boundary)
}

/// Split text preferring newline, then whitespace, then a safe boundary
/// (TS: `splitTextIntoChunks`).
pub fn split_text_into_chunks(text: &str, max_length: usize) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }
    let limit = max_length.max(1);
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= limit {
        return vec![text.to_string()];
    }
    let mut chunks = Vec::new();
    let mut start = 0;
    while start < chars.len() {
        let end = find_split_boundary(&chars, start, limit)
            .filter(|end| *end > start)
            .expect("Unable to split text on a safe UTF-16 boundary");
        chunks.push(chars[start..end].iter().collect());
        start = end;
    }
    chunks
}

/// Block-split budgets with `>= 1` floors (TS: `BlockSplitLimits`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockSplitLimits {
    pub max_chars: usize,
    pub max_blocks: usize,
}

/// Normalize optional limits onto the defaults (TS: `normalizeBlockSplitLimits`).
pub fn normalize_block_split_limits(
    max_chars: Option<i64>,
    max_blocks: Option<i64>,
) -> BlockSplitLimits {
    BlockSplitLimits {
        max_chars: max_chars
            .map(|v| v.max(1) as usize)
            .unwrap_or(DEFAULT_MAX_PART_CHARS),
        max_blocks: max_blocks
            .map(|v| v.max(1) as usize)
            .unwrap_or(DEFAULT_MAX_PART_BLOCKS),
    }
}
