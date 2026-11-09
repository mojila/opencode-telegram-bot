//! Slice 178 parity: Russian streaming-footer gates.
//!
//! Mirrors the pure TS contract in `src/i18n/ru.ts` (compact lines 74-85,
//! pinned lines 477-486 + 502-504, subagent lines 487-501, footer lines
//! 178-181) for the 12 compact-progress keys, 13 pinned-dashboard keys,
//! 15 subagent-card keys and 3 footer-label keys outside `FULL_KEYS` in
//! `crate::i18n_full`:
//! - verbatim `ru` templates for progress headers, pinned lines,
//!   subagent cards and footer labels
//! - per-key placeholder equality against `en`
//!   (`tests/i18n/placeholders.test.ts` loop)
//! - `t(key, params)` line formatting (`src/i18n/index.ts` `interpolate`
//!   behind the progress / pinned / subagent call sites)
//!
//! Already covered elsewhere and NOT re-tested here:
//! placeholder set-equality helpers (`locale_placeholder_gates`, slice 122),
//! registry / `SUPPORTED_LOCALES` / `getLocale` (`crate::i18n` +
//! `locale_registry_gates`), en/ru tables + interpolation (`crate::i18n_full`,
//! including `progress.compact.activity` and `pinned.line.goal`), stream
//! budgets / card composition / footer thresholds / pinned caps
//! (`streaming`, `subagent_card_gates`, `duration_footer_display_gates`,
//! `pinned_models`), `ru` STT lookup + formatting (`locale_ru_voice_gates`,
//! slice 175), `ru` task-detail + goal-remainder lookup + formatting
//! (`locale_ru_task_goal_gates`, slice 176), `ru` start/stop lookup +
//! formatting (`locale_ru_start_stop_gates`, slice 177).

use otb_core::locale_ru_streaming_gates::*;

// ---- constants (4) ----

#[test]
fn ru_streaming_locale_value_parity() {
    assert_eq!(RU_STREAMING_LOCALE, "ru");
}

#[test]
fn ru_streaming_compact_keys_value_parity() {
    assert_eq!(RU_COMPACT_KEYS.len(), 12);
    assert!(RU_COMPACT_KEYS.contains(&"progress.compact.working_header"));
    assert!(RU_COMPACT_KEYS.contains(&"progress.compact.waiting_permission"));
    assert!(RU_COMPACT_KEYS.contains(&"progress.compact.count.tools"));
    assert!(RU_COMPACT_KEYS.contains(&"progress.compact.count.changed_files"));
}

#[test]
fn ru_streaming_pinned_and_footer_keys_value_parity() {
    assert_eq!(RU_PINNED_KEYS.len(), 13);
    assert!(RU_PINNED_KEYS.contains(&"pinned.line.context"));
    assert!(RU_PINNED_KEYS.contains(&"pinned.line.cost"));
    assert!(RU_PINNED_KEYS.contains(&"pinned.files.more"));
    assert_eq!(RU_SUBAGENT_KEYS.len(), 15);
    assert!(RU_SUBAGENT_KEYS.contains(&"subagent.starting"));
    assert!(RU_SUBAGENT_KEYS.contains(&"subagent.working_with_details"));
    assert_eq!(RU_FOOTER_KEYS.len(), 3);
    assert!(RU_FOOTER_KEYS.contains(&"assistant.footer.interrupted"));
    assert!(RU_FOOTER_KEYS.contains(&"settings.pin_session_dashboard.label"));
}

#[test]
fn ru_streaming_templates_value_parity() {
    assert_eq!(RU_COMPACT_WORKING_HEADER, "⏳ Работаю");
    assert_eq!(RU_COMPACT_THINKING, "💭 Думаю...");
    assert_eq!(RU_COMPACT_COUNT_TOOLS, "инструменты: {count}");
    assert_eq!(
        RU_PINNED_LINE_CONTEXT,
        "Контекст: {used} / {limit} ({percent}%)"
    );
    assert_eq!(RU_PINNED_LINE_COST, "Стоимость: {cost} потрачено");
    assert_eq!(RU_PINNED_FILES_MORE, "  ... и еще {count}");
    assert_eq!(RU_SUBAGENT_STARTING, "Запуск задачи: {task}");
    assert_eq!(RU_SUBAGENT_WORKING_WITH_DETAILS, "В работе: {details}");
    assert_eq!(RU_ASSISTANT_FOOTER_INTERRUPTED, "прерван");
}

// ---- lookup (2) ----

#[test]
fn ru_streaming_lookup_subagent_starting_parity() {
    assert_eq!(
        lookup_ru_streaming_template("subagent.starting"),
        Some("Запуск задачи: {task}")
    );
}

#[test]
fn ru_streaming_lookup_unknown_returns_none_parity() {
    assert_eq!(lookup_ru_streaming_template("subagent.unknown_key"), None);
    assert_eq!(
        lookup_ru_streaming_template("progress.compact.activity"),
        None
    );
}

// ---- placeholder equality (2) ----

#[test]
fn ru_streaming_placeholders_match_en_parity() {
    assert!(is_ru_streaming_placeholder_ok(
        "Cost: {cost} spent",
        "Стоимость: {cost} потрачено"
    ));
    assert!(is_ru_streaming_placeholder_ok(
        "Subagent {agent}: {description}",
        "Сабагент {agent}: {description}"
    ));
    assert!(is_ru_streaming_placeholder_ok(
        "Context: {used} / {limit} ({percent}%)",
        "Контекст: {used} / {limit} ({percent}%)"
    ));
}

#[test]
fn ru_streaming_placeholders_reject_mismatch_parity() {
    assert!(!is_ru_streaming_placeholder_ok(
        "Files ({count}):",
        "Файлы:"
    ));
    assert!(!is_ru_streaming_placeholder_ok(
        "  {path}{diff}",
        "  {path}{diff} {extra}"
    ));
}

// ---- formatting (4) ----

#[test]
fn ru_streaming_compact_count_line_parity() {
    assert_eq!(
        format_ru_compact_count("tools", 3),
        Some("инструменты: 3".to_string())
    );
    assert_eq!(
        format_ru_compact_count("changed_files", 1),
        Some("изменённые файлы: 1".to_string())
    );
}

#[test]
fn ru_streaming_compact_count_unknown_kind_returns_none_parity() {
    assert_eq!(format_ru_compact_count("tokens", 3), None);
}

#[test]
fn ru_streaming_subagent_and_cost_lines_parity() {
    assert_eq!(
        format_ru_subagent_starting("refactor auth"),
        "Запуск задачи: refactor auth"
    );
    assert_eq!(format_ru_pinned_cost("$0.42"), "Стоимость: $0.42 потрачено");
}

#[test]
fn ru_streaming_pinned_files_more_line_parity() {
    assert_eq!(format_ru_pinned_files_more(4), "  ... и еще 4");
}
