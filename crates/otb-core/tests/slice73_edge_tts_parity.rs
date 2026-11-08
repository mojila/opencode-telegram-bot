//! Slice 73 parity: Edge TTS pure gates (RED).
//!
//! Mirrors the pure TS contract in `src/app/services/edge-tts.ts` and
//! `src/app/services/edge-tts-drm.ts`:
//! - `escapeXml` / `escapeXmlAttribute` (order: `&` first, then `<>`/`'"`)
//! - `removeIncompatibleCharacters` (rejected control ranges become spaces)
//! - `splitTextByByteLength` (byte budget, newline/space preference,
//!   continuation + entity back-up, trim, empty drop)
//! - `buildSsml` (envelope, escaped attrs, verbatim text)
//! - `buildWssUrl` / `edgeUserAgent` (handshake strings)
//! - `parseRfc2616Date` (seconds or `None`), `jsDateString` (UTC reorder)
//! - `rol16` / `rotr` / small sigmas, `generateSecMsGecV2` (5-minute window,
//!   `<16 hex>-<64 hex>` shape)
//!
//! Every test MUST FAIL until `edge_tts_gates` is ported (step 2).

use otb_core::edge_tts_gates::*;

// --- escape_xml ---

#[test]
fn slice73_escape_xml_amp_parity() {
    assert_eq!(escape_xml("a&b"), "a&amp;b");
}

#[test]
fn slice73_escape_xml_angles_parity() {
    assert_eq!(escape_xml("<tag>"), "&lt;tag&gt;");
}

#[test]
fn slice73_escape_xml_combined_parity() {
    assert_eq!(escape_xml("a&b<c>d"), "a&amp;b&lt;c&gt;d");
}

#[test]
fn slice73_escape_xml_amp_first_order_parity() {
    // `&` is escaped before `<`/`>`, so no double-escaping of the entities.
    assert_eq!(escape_xml("&<>"), "&amp;&lt;&gt;");
}

#[test]
fn slice73_escape_xml_plain_unchanged_parity() {
    assert_eq!(escape_xml("hello world"), "hello world");
}

#[test]
fn slice73_escape_xml_empty_parity() {
    assert_eq!(escape_xml(""), "");
}

// --- escape_xml_attribute ---

#[test]
fn slice73_escape_attr_single_quote_parity() {
    assert_eq!(escape_xml_attribute("o'clock"), "o&apos;clock");
}

#[test]
fn slice73_escape_attr_double_quote_parity() {
    assert_eq!(escape_xml_attribute("say \"hi\""), "say &quot;hi&quot;");
}

#[test]
fn slice73_escape_attr_combined_parity() {
    assert_eq!(
        escape_xml_attribute("a&b<c>'d\""),
        "a&amp;b&lt;c&gt;&apos;d&quot;"
    );
}

#[test]
fn slice73_escape_attr_plain_unchanged_parity() {
    assert_eq!(escape_xml_attribute("Emma"), "Emma");
}

#[test]
fn slice73_escape_attr_empty_parity() {
    assert_eq!(escape_xml_attribute(""), "");
}

// --- remove_incompatible_characters ---

#[test]
fn slice73_remove_nul_parity() {
    assert_eq!(remove_incompatible_characters("\0a"), " a");
}

#[test]
fn slice73_remove_bel_parity() {
    assert_eq!(remove_incompatible_characters("a\x07b"), "a b");
}

#[test]
fn slice73_remove_vertical_tab_form_feed_parity() {
    assert_eq!(remove_incompatible_characters("a\x0Bb\x0Cc"), "a b c");
}

#[test]
fn slice73_remove_unit_separator_parity() {
    assert_eq!(remove_incompatible_characters("a\x1Fb"), "a b");
}

#[test]
fn slice73_remove_keeps_tab_lf_cr_parity() {
    assert_eq!(remove_incompatible_characters("a\tb\nc\rd"), "a\tb\nc\rd");
}

// --- split_text_by_byte_length ---

#[test]
fn slice73_split_short_single_chunk_parity() {
    assert_eq!(split_text_by_byte_length("hello", 4096), vec!["hello"]);
}

#[test]
fn slice73_split_whitespace_only_empty_parity() {
    assert!(split_text_by_byte_length("   ", 4096).is_empty());
}

#[test]
fn slice73_split_long_respects_byte_budget_parity() {
    let text = "word ".repeat(2000);
    let chunks = split_text_by_byte_length(&text, 4096);
    assert!(chunks.len() > 1);
    assert!(chunks.iter().all(|c| c.len() <= 4096));
}

#[test]
fn slice73_split_prefers_newline_parity() {
    let text = format!("ab\n{}", "x".repeat(5000));
    let chunks = split_text_by_byte_length(&text, 100);
    assert_eq!(chunks[0], "ab");
}

#[test]
fn slice73_split_prefers_space_without_newline_parity() {
    let text = format!("ab cd {}", "x".repeat(5000));
    let chunks = split_text_by_byte_length(&text, 10);
    assert_eq!(chunks[0], "ab cd");
}

#[test]
fn slice73_split_never_inside_multibyte_char_parity() {
    let text = "😀".repeat(2000);
    let chunks = split_text_by_byte_length(&text, 4096);
    assert_eq!(chunks.len(), 2);
    assert!(chunks.iter().all(|c| c.len() <= 4096));
    assert_eq!(chunks.concat(), text);
}

#[test]
fn slice73_split_empty_parity() {
    assert!(split_text_by_byte_length("", 4096).is_empty());
}

#[test]
fn slice73_split_exact_fit_single_chunk_parity() {
    assert_eq!(split_text_by_byte_length("abc", 3), vec!["abc"]);
}

// --- build_ssml ---

#[test]
fn slice73_ssml_voice_attr_parity() {
    let ssml = build_ssml("v", "+0%", "+0%", "+0Hz", "hi");
    assert!(ssml.contains("<voice name='v'>"));
}

#[test]
fn slice73_ssml_prosody_attrs_parity() {
    let ssml = build_ssml("v", "+0%", "+0%", "+0Hz", "hi");
    assert!(ssml.contains("pitch='+0Hz' rate='+0%' volume='+0%'"));
}

#[test]
fn slice73_ssml_text_verbatim_parity() {
    let ssml = build_ssml("v", "+0%", "+0%", "+0Hz", "hi");
    assert!(ssml.ends_with("hi</prosody></voice></speak>"));
}

#[test]
fn slice73_ssml_exact_envelope_parity() {
    assert_eq!(
        build_ssml("v", "+0%", "+0%", "+0Hz", "hi"),
        "<speak version='1.0' xmlns='http://www.w3.org/2001/10/synthesis' \
         xml:lang='en-US'><voice name='v'><prosody pitch='+0Hz' rate='+0%' \
         volume='+0%'>hi</prosody></voice></speak>"
    );
}

#[test]
fn slice73_ssml_escapes_voice_quotes_parity() {
    let ssml = build_ssml("a'b", "+0%", "+0%", "+0Hz", "hi");
    assert!(ssml.contains("<voice name='a&apos;b'>"));
}

// --- build_wss_url / edge_user_agent ---

#[test]
fn slice73_wss_url_exact_parity() {
    assert_eq!(
        build_wss_url("connid", "GEC"),
        "wss://speech.platform.bing.com/consumer/speech/synthesize/readaloud/edge/v1\
         ?TrustedClientToken=6A5AA1D4EAFF4E9FB37E23D68491D6F4\
         &ConnectionId=connid&Sec-MS-GEC=GEC&Sec-MS-GEC-Version=2-143.0.3650.75"
    );
}

#[test]
fn slice73_edge_user_agent_exact_parity() {
    assert_eq!(
        edge_user_agent(),
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
         (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36 Edg/143.0.0.0"
    );
}

// --- parse_rfc2616_date ---

#[test]
fn slice73_parse_rfc2616_valid_parity() {
    assert_eq!(
        parse_rfc2616_date("Thu, 01 Jan 1970 00:01:00 GMT"),
        Some(60.0)
    );
}

#[test]
fn slice73_parse_rfc2616_invalid_none_parity() {
    assert_eq!(parse_rfc2616_date("not a date"), None);
}

#[test]
fn slice73_parse_rfc2616_empty_none_parity() {
    assert_eq!(parse_rfc2616_date(""), None);
}

#[test]
fn slice73_parse_rfc2616_epoch_parity() {
    assert_eq!(
        parse_rfc2616_date("Thu, 01 Jan 1970 00:00:00 GMT"),
        Some(0.0)
    );
}

// --- js_date_string ---

#[test]
fn slice73_js_date_string_exact_parity() {
    assert_eq!(
        js_date_string("Fri", "Jan", "01", "2024", "00:00:00"),
        "Fri Jan 01 2024 00:00:00 GMT+0000 (Coordinated Universal Time)"
    );
}

#[test]
fn slice73_js_date_string_suffix_parity() {
    let rendered = js_date_string("Mon", "Dec", "25", "2023", "12:34:56");
    assert!(rendered.ends_with("GMT+0000 (Coordinated Universal Time)"));
    assert!(rendered.starts_with("Mon Dec 25 2023 12:34:56 "));
}

// --- rol16 / rotr / sigmas ---

#[test]
fn slice73_rol16_one_parity() {
    assert_eq!(rol16(1), 65536);
}

#[test]
fn slice73_rol16_vectors_parity() {
    assert_eq!(rol16(0x1234_5678), 0x5678_1234);
}

#[test]
fn slice73_rotr_high_bit_parity() {
    assert_eq!(rotr(0x8000_0000, 1), 0x4000_0000);
}

#[test]
fn slice73_rotr_wrap_low_bit_parity() {
    assert_eq!(rotr(1, 1), 0x8000_0000);
}

#[test]
fn slice73_sigma0_zero_parity() {
    assert_eq!(sigma0_small(0), 0);
}

#[test]
fn slice73_sigma1_zero_parity() {
    assert_eq!(sigma1_small(0), 0);
}

// --- generate_sec_ms_gec_v2 ---

#[test]
fn slice73_gec_format_parity() {
    let token = generate_sec_ms_gec_v2(1_700_000_000.0);
    assert_eq!(token.len(), 81);
    assert_eq!(token.chars().nth(16), Some('-'));
    let (head, tail) = token.split_at(16);
    let tail = &tail[1..];
    assert_eq!(head.len(), 16);
    assert_eq!(tail.len(), 64);
    assert!(head.bytes().all(|b| b.is_ascii_hexdigit()));
    assert!(tail.bytes().all(|b| b.is_ascii_hexdigit()));
    assert!(!token.bytes().any(|b| b.is_ascii_lowercase()));
}

#[test]
fn slice73_gec_deterministic_parity() {
    assert_eq!(
        generate_sec_ms_gec_v2(1_700_000_000.0),
        generate_sec_ms_gec_v2(1_700_000_000.0)
    );
}

#[test]
fn slice73_gec_window_stable_parity() {
    let base = 1_700_000_000.0;
    let aligned = base - ((base + WIN_EPOCH_SECONDS) % ROUND_SECONDS) + 60.0;
    assert_eq!(
        generate_sec_ms_gec_v2(aligned),
        generate_sec_ms_gec_v2(aligned + 60.0)
    );
}

#[test]
fn slice73_gec_window_changes_parity() {
    let base = 1_700_000_000.0;
    let aligned = base - ((base + WIN_EPOCH_SECONDS) % ROUND_SECONDS) + 60.0;
    assert_ne!(
        generate_sec_ms_gec_v2(aligned),
        generate_sec_ms_gec_v2(aligned + 300.0)
    );
}

#[test]
fn slice73_gec_not_trusted_token_parity() {
    let token = generate_sec_ms_gec_v2(1_700_000_000.0);
    assert!(!token.is_empty());
    assert_ne!(token, TRUSTED_CLIENT_TOKEN);
}
