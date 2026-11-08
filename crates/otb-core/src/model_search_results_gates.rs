//! Model search results view + search metadata pure gates (slice 120, GREEN).
//!
//! Rust port of the pure remainder in
//! `src/bot/callbacks/model-selection-callback-handler.ts`:
//! - `MODEL_SEARCH_RESULT_CALLBACK_PREFIX` (`"model:result:"`)
//! - `buildModelSearchResultsView` label (`"${providerID}/${modelID}"`),
//!   result callback (`prefix + index`), footer row
//!   (`model.search.search_again` + `inline.button.cancel`), text key
//!   (`model.search.no_results` when empty else
//!   `model.search.results_title`)
//! - `transitionToModelSearchResults` metadata (`flow: "model-search"`,
//!   `stage: "results"`, `messageId`, `query`, `models` with `"default"`
//!   variant reset)
//! - `parseModelSearchMetadata` gates (`kind === "custom"`,
//!   `flow === "model-search"`, `typeof stage === "string"`, numeric
//!   `messageId` else `undefined`, string `query` else `""`)
//! - `handleModelSearchResults` stale-message gate (`meta.messageId !==
//!   undefined && callbackMessageId !== meta.messageId`)
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - grammy `InlineKeyboard` / `ctx.reply` / `ctx.editMessageText` drawing,
//!   `t(...)` locale lookups (keys travel as literals here),
//!   `searchModels` fetches, `InteractionManager` snapshots/transitions.
//!
//! Already ported elsewhere and NOT repeated here:
//! - `MODEL_SEARCH_CALLBACK` / `MODEL_SEARCH_AGAIN_CALLBACK` /
//!   `MODEL_SEARCH_CANCEL_CALLBACK` constants, `parseModelItems` shape
//!   gates, `parseCallbackIndex` / `parseNonNegativeIndex`, search dispatch
//!   routing (`agent_model_variant_gates` owner)
//! - `handleModelSearchTextInput` input-stage routing
//!   (`task_model_text_input_gates` owner).
//!
//! Deviations: none intended. `None` query counts as the `""` fallback (TS
//! `typeof query === "string" ? query : ""`); `None` message id counts as
//! absent (TS `typeof messageId === "number" ? ... : undefined`).
//!
//! Step 2 (GREEN): pure port of the TS contract. Constant values match the
//! handler literals; helpers below mirror `buildModelSearchResultsView`,
//! `parseCallbackIndex`, `parseModelSearchMetadata`, and the stale-message
//! branch in `handleModelSearchResults`.

/// Search-result pick prefix (TS: `MODEL_SEARCH_RESULT_CALLBACK_PREFIX`).
pub const MODEL_SEARCH_RESULT_CALLBACK_PREFIX: &str = "model:result:";

/// Search interaction flow name (TS: `flow: "model-search"`).
pub const MODEL_SEARCH_FLOW: &str = "model-search";

/// Results stage name (TS: `stage: "results"` in `transitionToModelSearchResults`).
pub const MODEL_SEARCH_RESULTS_STAGE: &str = "results";

/// Input stage name (TS: `stage: "input"` search prompt).
pub const MODEL_SEARCH_INPUT_STAGE: &str = "input";

/// Results title locale key (TS: `t("model.search.results_title", ...)`).
pub const MODEL_SEARCH_RESULTS_TITLE_KEY: &str = "model.search.results_title";

/// Empty-results locale key (TS: `t("model.search.no_results", ...)`).
pub const MODEL_SEARCH_NO_RESULTS_KEY: &str = "model.search.no_results";

/// Search prompt locale key (TS: `t("model.search.prompt")`).
pub const MODEL_SEARCH_PROMPT_KEY: &str = "model.search.prompt";

/// Search error locale key (TS: `t("model.search.error")`).
pub const MODEL_SEARCH_ERROR_KEY: &str = "model.search.error";

/// Search-again button locale key (TS: `t("model.search.search_again")`).
pub const MODEL_SEARCH_AGAIN_KEY: &str = "model.search.search_again";

/// Inline cancel button locale key (TS: `t("inline.button.cancel")`).
pub const INLINE_CANCEL_KEY: &str = "inline.button.cancel";

/// `model:result:<index>` callback build (TS: result button in
/// `buildModelSearchResultsView`).
pub fn build_search_result_callback(index: usize) -> String {
    format!("{MODEL_SEARCH_RESULT_CALLBACK_PREFIX}{index}")
}

/// `"provider/model"` result row label (TS: result button label).
pub fn search_result_label(provider_id: &str, model_id: &str) -> String {
    format!("{provider_id}/{model_id}")
}

/// Results text locale key: empty results read the no-results key,
/// otherwise the results-title key (TS: `results.length === 0` branch).
pub fn search_results_text_key(is_empty: bool) -> &'static str {
    if is_empty {
        MODEL_SEARCH_NO_RESULTS_KEY
    } else {
        MODEL_SEARCH_RESULTS_TITLE_KEY
    }
}

/// Whether a callback starts with the search-result prefix
/// (TS: `parseCallbackIndex(data, MODEL_SEARCH_RESULT_CALLBACK_PREFIX)`
/// eligibility).
pub fn is_search_result_callback(data: &str) -> bool {
    data.starts_with(MODEL_SEARCH_RESULT_CALLBACK_PREFIX)
}

/// Index after the search-result prefix, digits-only (TS:
/// `parseCallbackIndex` on the result prefix).
pub fn parse_search_result_index(data: &str) -> Option<u64> {
    let rest = data.strip_prefix(MODEL_SEARCH_RESULT_CALLBACK_PREFIX)?;
    parse_digits(rest)
}

/// Whether the metadata interaction may hold search state
/// (TS: `state.kind === "custom" && flow === "model-search" && typeof
/// stage === "string"` in `parseModelSearchMetadata`).
pub fn is_search_metadata_eligible(kind: &str, flow: Option<&str>, has_stage: bool) -> bool {
    kind == "custom" && flow == Some(MODEL_SEARCH_FLOW) && has_stage
}

/// String query or the `""` fallback (TS: `typeof query === "string" ?
/// query : ""`).
pub fn normalize_search_query(query: Option<&str>) -> String {
    query.unwrap_or("").to_string()
}

/// Whether the results-stage callback belongs to the current message
/// (TS: `meta.messageId === undefined || callbackMessageId ===
/// meta.messageId`; the inverse answers the stale-message branch).
pub fn search_message_id_matches(
    meta_message_id: Option<u64>,
    callback_message_id: Option<u64>,
) -> bool {
    match meta_message_id {
        None => true,
        Some(meta) => Some(meta) == callback_message_id,
    }
}

/// Whether the results-stage callback is stale and must be rejected
/// (TS: `meta.messageId !== undefined && callbackMessageId !==
/// meta.messageId`).
pub fn is_stale_search_callback(
    meta_message_id: Option<u64>,
    callback_message_id: Option<u64>,
) -> bool {
    !search_message_id_matches(meta_message_id, callback_message_id)
}

fn parse_digits(text: &str) -> Option<u64> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse::<u64>().ok()
}
