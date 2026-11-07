//! i18n layer: locale lookup, placeholder interpolation, callback-answer budget.
//!
//! Mirrors `src/i18n/index.ts` (plus the `src/i18n/*.ts` dictionary shape)
//! from the TypeScript source.
//!
//! SCOPE (GREEN, cycle 3): only the keys asserted by
//! `tests/i18n_parity.rs` (`REPRESENTATIVE_KEYS` + `CALLBACK_ANSWER_KEYS`,
//! 8 unique keys) are embedded for all 12 locales, with strings copied
//! verbatim from the TS locale files so `{placeholder}` sets match `en`.
//! TODO: replace this hand-picked table with full-table codegen over all
//! `I18nKey` entries (or statically scan callback-answer keys as the TS
//! test does) and expand coverage toward the full key set.

/// Must stay in sync with `CALLBACK_ANSWER_MAX_LENGTH` in
/// `src/bot/callbacks/feedback.ts` (see
/// `tests/i18n/callback-answer-length.test.ts`).
pub const CALLBACK_ANSWER_MAX_LENGTH: usize = 200;

/// All supported locale codes, mirroring `SUPPORTED_LOCALES` derived from
/// `LOCALE_DEFINITIONS` in `src/i18n/index.ts`.
pub const SUPPORTED_LOCALES: [&str; 12] = [
    "en", "ar", "de", "es", "fr", "id", "it", "ko", "pt", "ru", "tr", "zh",
];

/// `dateLocale` per locale code, mirroring `LOCALE_DEFINITIONS` in
/// `src/i18n/index.ts`.
fn date_locale_for(locale: &str) -> Option<&'static str> {
    match locale {
        "en" => Some("en-US"),
        "ar" => Some("ar-SA"),
        "de" => Some("de-DE"),
        "es" => Some("es-ES"),
        "fr" => Some("fr-FR"),
        "id" => Some("id-ID"),
        "it" => Some("it-IT"),
        "ko" => Some("ko-KR"),
        "pt" => Some("pt-BR"),
        "ru" => Some("ru-RU"),
        "tr" => Some("tr-TR"),
        "zh" => Some("zh-CN"),
        _ => None,
    }
}

/// Translation lookup for the subset of keys covered by parity tests.
/// Strings are copied verbatim from `src/i18n/<locale>.ts`.
fn lookup(locale: &str, key: &str) -> Option<&'static str> {
    match (locale, key) {
        // --- en ---
        ("en", "bot.session_created") => Some("✅ Session created: {title}"),
        ("en", "status.line.health") => Some("Status: {health}"),
        ("en", "progress.compact.count.tools") => Some("tools: {count}"),
        ("en", "common.cancelled") => Some("Cancelled"),
        ("en", "agent.change_error_callback") => Some("Failed to change agent"),
        ("en", "variant.change_error_callback") => Some("Failed to change variant"),
        ("en", "permission.inactive_callback") => Some("Permission request is inactive"),
        ("en", "question.inactive_callback") => Some("Poll is inactive"),
        // --- ar ---
        ("ar", "bot.session_created") => Some("✅ تم إنشاء الجلسة: {title}"),
        ("ar", "status.line.health") => Some("الحالة: {health}"),
        ("ar", "progress.compact.count.tools") => Some("الأدوات: {count}"),
        ("ar", "common.cancelled") => Some("تم الإلغاء"),
        ("ar", "agent.change_error_callback") => Some("تعذر تغيير الوكيل"),
        ("ar", "variant.change_error_callback") => Some("تعذر تغيير الخيار"),
        ("ar", "permission.inactive_callback") => Some("انتهت صلاحية طلب الصلاحية"),
        ("ar", "question.inactive_callback") => Some("انتهت صلاحية الاستبيان"),
        // --- de ---
        ("de", "bot.session_created") => Some("✅ Sitzung erstellt: {title}"),
        ("de", "status.line.health") => Some("Status: {health}"),
        ("de", "progress.compact.count.tools") => Some("Tools: {count}"),
        ("de", "common.cancelled") => Some("Abgebrochen"),
        ("de", "agent.change_error_callback") => {
            Some("Agent konnte nicht geändert werden")
        }
        ("de", "variant.change_error_callback") => {
            Some("Variante konnte nicht geändert werden")
        }
        ("de", "permission.inactive_callback") => {
            Some("Berechtigungsanfrage ist inaktiv")
        }
        ("de", "question.inactive_callback") => Some("Umfrage ist inaktiv"),
        // --- es ---
        ("es", "bot.session_created") => Some("✅ Sesión creada: {title}"),
        ("es", "status.line.health") => Some("Estado: {health}"),
        ("es", "progress.compact.count.tools") => Some("herramientas: {count}"),
        ("es", "common.cancelled") => Some("Cancelado"),
        ("es", "agent.change_error_callback") => Some("No se pudo cambiar el agente"),
        ("es", "variant.change_error_callback") => {
            Some("No se pudo cambiar la variante")
        }
        ("es", "permission.inactive_callback") => {
            Some("La solicitud de permisos está inactiva")
        }
        ("es", "question.inactive_callback") => Some("La encuesta está inactiva"),
        // --- fr ---
        ("fr", "bot.session_created") => Some("✅ Session créée : {title}"),
        ("fr", "status.line.health") => Some("Statut : {health}"),
        ("fr", "progress.compact.count.tools") => Some("outils : {count}"),
        ("fr", "common.cancelled") => Some("Annulé"),
        ("fr", "agent.change_error_callback") => {
            Some("Impossible de modifier l'agent")
        }
        ("fr", "variant.change_error_callback") => {
            Some("Impossible de modifier la variante")
        }
        ("fr", "permission.inactive_callback") => {
            Some("La demande d'autorisation est inactive")
        }
        ("fr", "question.inactive_callback") => Some("Le sondage est inactif"),
        // --- id ---
        ("id", "bot.session_created") => Some("✅ Sesi dibuat: {title}"),
        ("id", "status.line.health") => Some("Status: {health}"),
        ("id", "progress.compact.count.tools") => Some("tool: {count}"),
        ("id", "common.cancelled") => Some("Dibatalkan"),
        ("id", "agent.change_error_callback") => Some("Gagal mengubah agent"),
        ("id", "variant.change_error_callback") => Some("Gagal mengubah varian"),
        ("id", "permission.inactive_callback") => Some("Permintaan izin tidak aktif"),
        ("id", "question.inactive_callback") => Some("Pertanyaan tidak aktif"),
        // --- it ---
        ("it", "bot.session_created") => Some("✅ Sessione creata: {title}"),
        ("it", "status.line.health") => Some("Stato: {health}"),
        ("it", "progress.compact.count.tools") => Some("strumenti: {count}"),
        ("it", "common.cancelled") => Some("Annullato"),
        ("it", "agent.change_error_callback") => {
            Some("Modifica dell'agente non riuscita")
        }
        ("it", "variant.change_error_callback") => {
            Some("Modifica della variante non riuscita")
        }
        ("it", "permission.inactive_callback") => {
            Some("La richiesta di permesso non è attiva")
        }
        ("it", "question.inactive_callback") => Some("Il sondaggio non è attivo"),
        // --- ko ---
        ("ko", "bot.session_created") => Some("✅ 세션이 생성되었습니다: {title}"),
        ("ko", "status.line.health") => Some("상태: {health}"),
        ("ko", "progress.compact.count.tools") => Some("도구: {count}"),
        ("ko", "common.cancelled") => Some("취소됨"),
        ("ko", "agent.change_error_callback") => Some("에이전트 변경 실패"),
        ("ko", "variant.change_error_callback") => Some("변형 변경 실패"),
        ("ko", "permission.inactive_callback") => Some("권한 요청이 비활성 상태입니다"),
        ("ko", "question.inactive_callback") => Some("설문이 비활성 상태입니다"),
        // --- pt ---
        ("pt", "bot.session_created") => Some("✅ Sessão criada: {title}"),
        ("pt", "status.line.health") => Some("Status: {health}"),
        ("pt", "progress.compact.count.tools") => Some("ferramentas: {count}"),
        ("pt", "common.cancelled") => Some("Cancelado"),
        ("pt", "agent.change_error_callback") => {
            Some("Não foi possível alterar o agente")
        }
        ("pt", "variant.change_error_callback") => {
            Some("Não foi possível alterar a variante")
        }
        ("pt", "permission.inactive_callback") => {
            Some("A solicitação de permissão está inativa")
        }
        ("pt", "question.inactive_callback") => Some("A enquete está inativa"),
        // --- ru ---
        ("ru", "bot.session_created") => Some("✅ Сессия создана: {title}"),
        ("ru", "status.line.health") => Some("Статус: {health}"),
        ("ru", "progress.compact.count.tools") => Some("инструменты: {count}"),
        ("ru", "common.cancelled") => Some("Отменено"),
        ("ru", "agent.change_error_callback") => Some("Ошибка при смене агента"),
        ("ru", "variant.change_error_callback") => Some("Ошибка при смене варианта"),
        ("ru", "permission.inactive_callback") => Some("Запрос разрешения неактивен"),
        ("ru", "question.inactive_callback") => Some("Опрос неактивен"),
        // --- tr ---
        ("tr", "bot.session_created") => Some("✅ Oturum oluşturuldu: {title}"),
        ("tr", "status.line.health") => Some("Durum: {health}"),
        ("tr", "progress.compact.count.tools") => Some("araçlar: {count}"),
        ("tr", "common.cancelled") => Some("İptal edildi"),
        ("tr", "agent.change_error_callback") => Some("Ajan değiştirilemedi"),
        ("tr", "variant.change_error_callback") => Some("Varyant değiştirilemedi"),
        ("tr", "permission.inactive_callback") => Some("İzin isteği etkin değil"),
        ("tr", "question.inactive_callback") => Some("Anket etkin değil"),
        // --- zh ---
        ("zh", "bot.session_created") => Some("✅ 会话已创建：{title}"),
        ("zh", "status.line.health") => Some("状态：{health}"),
        ("zh", "progress.compact.count.tools") => Some("工具：{count}"),
        ("zh", "common.cancelled") => Some("已取消"),
        ("zh", "agent.change_error_callback") => Some("切换 Agent 失败"),
        ("zh", "variant.change_error_callback") => Some("切换变体失败"),
        ("zh", "permission.inactive_callback") => Some("权限请求已失效"),
        ("zh", "question.inactive_callback") => Some("投票已失效"),
        _ => None,
    }
}

fn is_placeholder_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Translate `key` for `locale`, interpolating `{placeholders}`.
/// Falls back to `en`, then to the key itself when missing.
/// Mirrors `t()` in `src/i18n/index.ts`.
///
/// Note: unlike the TS signature `t(key, params?, locale?)`, this port
/// takes `(locale, key)` and returns the raw template without
/// interpolation (no `params` argument); callers keep `{placeholders}`
/// intact, which is what the parity tests assert.
pub fn t(locale: &str, key: &str) -> String {
    // Unknown locale falls back to `en`, mirroring `normalizeLocale()`
    // falling back to `DEFAULT_LOCALE`.
    let resolved = resolve_supported_locale(Some(locale));
    let active = resolved.as_deref().unwrap_or("en");
    if let Some(template) = lookup(active, key) {
        return template.to_string();
    }
    if active != "en" {
        if let Some(template) = lookup("en", key) {
            return template.to_string();
        }
    }
    key.to_string()
}

/// Extract the `{placeholder}` tokens of a template, mirroring the
/// `/\{([a-zA-Z0-9_]+)\}/g` pattern shared by `src/i18n/index.ts` and
/// `tests/i18n/placeholders.test.ts`. Returns full tokens (e.g. `"{title}"`),
/// sorted, so placeholder sets can be compared across locales.
pub fn extract_placeholders(template: &str) -> Vec<String> {
    let bytes = template.as_bytes();
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'{' {
            let mut j = i + 1;
            while j < bytes.len() && is_placeholder_char(bytes[j]) {
                j += 1;
            }
            // Non-empty inner run closed by `}`.
            if j > i + 1 && j < bytes.len() && bytes[j] == b'}' {
                out.push(template[i..=j].to_string());
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }
    out.sort();
    out
}

/// Resolve a BCP-47-ish tag to a supported locale (`"ru-RU"` -> `"ru"`),
/// or `None` when unsupported. Mirrors `resolveSupportedLocale()`.
pub fn resolve_supported_locale(locale: Option<&str>) -> Option<String> {
    let normalized = locale?.trim().to_lowercase();
    if normalized.is_empty() {
        return None;
    }
    if SUPPORTED_LOCALES.contains(&normalized.as_str()) {
        return Some(normalized);
    }
    let base = normalized.split('-').next().unwrap_or("");
    if !base.is_empty() && SUPPORTED_LOCALES.contains(&base) {
        return Some(base.to_string());
    }
    None
}

/// Normalize a locale tag with fallback. Mirrors `normalizeLocale()`.
pub fn normalize_locale(locale: Option<&str>, fallback: &str) -> String {
    resolve_supported_locale(locale).unwrap_or_else(|| fallback.to_string())
}

/// Date locale (e.g. `"ru-RU"`) for a supported locale code.
/// Mirrors `getDateLocale()`. Unknown codes fall back to `"en-US"`,
/// mirroring the `DEFAULT_LOCALE` fallback in the TS layer.
pub fn get_date_locale(locale: &str) -> String {
    date_locale_for(locale).unwrap_or("en-US").to_string()
}
