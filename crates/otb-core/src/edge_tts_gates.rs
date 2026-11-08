//! Edge TTS pure gates (slice 73, GREEN).
//!
//! Rust port of the pure remainder in
//! `src/app/services/edge-tts.ts` and `src/app/services/edge-tts-drm.ts`
//! (the WebSocket synthesis, `ws`/`crypto` randomness, clock-skew global,
//! timers, and logging stay with the caller):
//! - `escapeXml` (`&` first, then `<` / `>`)
//! - `escapeXmlAttribute` (`escapeXml` plus `'` / `"`)
//! - `removeIncompatibleCharacters` (control ranges `00-08`, `0B-0C`,
//!   `0E-1F` become spaces; `\t` / `\n` / `\r` survive)
//! - `splitTextByByteLength` (byte budget, newline-then-space preference,
//!   UTF-8 continuation back-up, XML-entity back-up, trim, empty drop)
//! - `buildSsml` (speak envelope with escaped voice/rate/volume/pitch attrs)
//! - `buildWssUrl` (readaloud endpoint + trusted token, connection id, GEC
//!   token and version)
//! - `edgeUserAgent` (Chrome/Edg `<major>.0.0.0` desktop UA)
//! - `parseRfc2616Date` (`Date.parse` ms `/1000`, `NaN` -> `None`)
//! - `jsDateString` (UTC parts reordered to the `X-Timestamp` shape)
//! - `rol16` / `rotr` (32-bit rotates)
//! - `sigma0Small` / `sigma1Small` (SHA-256 small sigmas)
//! - `generateSecMsGecV2` (Windows-epoch shift, 5-minute floor, v2
//!   `<16 hex>-<64 hex>` token)
//!
//! DEVIATIONS from TS (behavior-preserving for the covered cases):
//! 1. `Buffer` byte handling maps to the input's UTF-8 bytes (`str::as_bytes`)
//!    with chunk decode via `String::from_utf8_lossy` (the same replacement
//!    semantics as `Buffer.toString("utf-8")`); the UTF-8 continuation-byte
//!    back-up is expressed over bytes the same way. ASCII output is
//!    identical to TS.
//! 2. `generateSecMsGec` reads `Date.now()` plus a mutable clock-skew global
//!    in TS; the port takes Unix seconds as `f64` ([`generate_sec_ms_gec_v2`])
//!    so it stays pure and testable. Clock-skew bookkeeping stays with the
//!    caller.
//! 3. `jsDateString` takes `new Date()` in TS; the port takes the pre-split
//!    UTC parts (`day`, `month`, `day_num`, `year`, `time`) so no clock or
//!    locale lives here.
//! 4. `parseRfc2616Date` returns `Option<f64>` seconds (TS: `number | null`
//!    from `Date.parse(date) / 1000`). Only the IMF-fixdate shape
//!    (`"Thu, 01 Jan 1970 00:01:00 GMT"`, weekday ignored, `GMT` required)
//!    parses; anything else is `None` like `NaN` in TS.
//! 5. `Math.imul` (low 32 bits) maps to `u32::wrapping_mul`; `>>> 0` masking
//!    is inherent to `u32` arithmetic (`wrapping_add` / `wrapping_sub`).
//! 6. `generateMuid` / `connectId` (node `crypto` randomness), the WebSocket
//!    handshake/synthesis, retry-on-403, deadlines, and `Buffer.concat` audio
//!    assembly stay with the caller.
//! 7. `splitTextByByteLength` throws on `byteLength <= 0` in TS; the port
//!    takes `usize` and panics with the same message on `0` (no `Result`
//!    wrapper).
//! 8. The 5-minute-window tick string renders the rounded `f64` tick count
//!    through `u64` (exact for integral `f64` values); JS template rendering
//!    prints the same decimal digits below `1e21`.
//! 9. Chunk/remaining trimming uses Rust `str::trim` (Unicode whitespace)
//!    where TS uses `String.trim()`; the two agree on the covered
//!    space/newline inputs.

/// Default Edge voice (`EDGE_DEFAULT_VOICE`).
pub const EDGE_DEFAULT_VOICE: &str = "en-US-EmmaMultilingualNeural";

/// Full Chromium version the GEC version and UA derive from.
pub const CHROMIUM_FULL_VERSION: &str = "143.0.3650.75";

/// GEC version header value (`SEC_MS_GEC_VERSION` = `2-<Chromium version>`).
pub const SEC_MS_GEC_VERSION: &str = "2-143.0.3650.75";

/// Total synthesis budget in ms (`SYNTHESIS_TIMEOUT_MS`).
pub const SYNTHESIS_TIMEOUT_MS: u64 = 60_000;

/// Per-chunk UTF-8 byte budget (`MAX_CHUNK_BYTES`).
pub const MAX_CHUNK_BYTES: usize = 4096;

/// Client token hashed into every GEC token (`TRUSTED_CLIENT_TOKEN`).
pub const TRUSTED_CLIENT_TOKEN: &str = "6A5AA1D4EAFF4E9FB37E23D68491D6F4";

/// Seconds between the Windows file-time epoch and the Unix epoch.
pub const WIN_EPOCH_SECONDS: f64 = 11_644_473_600.0;

/// GEC window in seconds (tokens floor to the nearest 5 minutes).
pub const ROUND_SECONDS: f64 = 300.0;

/// 100-ns ticks per second (Windows file time resolution).
pub const TICKS_PER_UNIX_SECOND: f64 = 1e7;

/// Escapes `&`, `<`, `>` for SSML text (TS: `escapeXml`).
pub fn escape_xml(text: &str) -> String {
    // `&` first so the `&` of the `&lt;` / `&gt;` entities is not re-escaped.
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Escapes a single-quoted XML attribute value (TS: `escapeXmlAttribute`).
pub fn escape_xml_attribute(value: &str) -> String {
    escape_xml(value)
        .replace('\'', "&apos;")
        .replace('"', "&quot;")
}

/// Replaces service-rejected control characters with spaces
/// (TS: `removeIncompatibleCharacters`).
pub fn remove_incompatible_characters(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '\u{0}'..='\u{8}' | '\u{B}' | '\u{C}' | '\u{E}'..='\u{1F}' => ' ',
            _ => c,
        })
        .collect()
}

/// Moves a split point back past an unterminated `&...` run so a chunk never
/// ends inside an XML entity (TS: `adjustForXmlEntity`).
fn adjust_for_xml_entity(buf: &[u8], split_at: usize) -> usize {
    let mut result = split_at.min(buf.len());
    while result > 0 {
        let amp = buf[..result].iter().rposition(|&b| b == b'&');
        let Some(amp) = amp else { break };
        if buf[amp..result].contains(&b';') {
            break;
        }
        result = amp;
    }
    result
}

/// Splits text into chunks of at most `byte_length` UTF-8 bytes
/// (TS: `splitTextByByteLength`; panics on `0` like TS throws on `<= 0`).
pub fn split_text_by_byte_length(text: &str, byte_length: usize) -> Vec<String> {
    assert!(
        byte_length > 0,
        "byteLength must be greater than 0"
    );
    let mut rest: &[u8] = text.as_bytes();
    let mut chunks: Vec<String> = Vec::new();
    while rest.len() > byte_length {
        // `Buffer.lastIndexOf(byte, byteLength - 1)`: last offset strictly
        // below `byte_length`, newlines preferred over spaces.
        let found = rest[..byte_length]
            .iter()
            .rposition(|&b| b == b'\n')
            .or_else(|| rest[..byte_length].iter().rposition(|&b| b == b' '));
        let mut split_at = match found {
            Some(index) => index,
            None => {
                // No preferred break: cut at the budget, backing up past
                // UTF-8 continuation bytes (`10xxxxxx`) so the cut never
                // lands inside a multi-byte character.
                let mut split_at = byte_length;
                while split_at > 0 {
                    let byte = rest[split_at];
                    if byte & 0xc0 != 0x80 {
                        break;
                    }
                    split_at -= 1;
                }
                split_at
            }
        };
        split_at = adjust_for_xml_entity(rest, split_at);
        if split_at == 0 {
            split_at = 1;
        }
        let chunk = String::from_utf8_lossy(&rest[..split_at]).trim().to_owned();
        if !chunk.is_empty() {
            chunks.push(chunk);
        }
        rest = &rest[split_at..];
    }
    let remaining = String::from_utf8_lossy(rest).trim().to_owned();
    if !remaining.is_empty() {
        chunks.push(remaining);
    }
    chunks
}

/// Builds the SSML document for one chunk (TS: `buildSsml`).
pub fn build_ssml(voice: &str, rate: &str, volume: &str, pitch: &str, text: &str) -> String {
    format!(
        "<speak version='1.0' xmlns='http://www.w3.org/2001/10/synthesis' xml:lang='en-US'>\
         <voice name='{}'>\
         <prosody pitch='{}' rate='{}' volume='{}'>\
         {}\
         </prosody></voice></speak>",
        escape_xml_attribute(voice),
        escape_xml_attribute(pitch),
        escape_xml_attribute(rate),
        escape_xml_attribute(volume),
        text,
    )
}

/// Builds the Edge readaloud WebSocket URL for one chunk attempt
/// (TS: the inline `WSS_URL + ConnectionId + Sec-MS-GEC` template).
pub fn build_wss_url(connection_id: &str, gec: &str) -> String {
    format!(
        "wss://speech.platform.bing.com/consumer/speech/synthesize/readaloud/edge/v1\
         ?TrustedClientToken={TRUSTED_CLIENT_TOKEN}\
         &ConnectionId={connection_id}\
         &Sec-MS-GEC={gec}\
         &Sec-MS-GEC-Version={SEC_MS_GEC_VERSION}"
    )
}

/// Desktop Edge user agent sent on the WebSocket handshake
/// (TS: the `User-Agent` entry of `WSS_HEADERS`).
pub fn edge_user_agent() -> String {
    let major = CHROMIUM_FULL_VERSION.split('.').next().unwrap_or("143");
    format!(
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
         (KHTML, like Gecko) Chrome/{major}.0.0.0 Safari/537.36 Edg/{major}.0.0.0"
    )
}

/// Days in a month for the civil-date conversion (TS: none, `Date.parse`
/// rejects impossible dates instead).
fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        _ => 0,
    }
}

/// Days since the Unix epoch for a civil date (Howard Hinnant's algorithm).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (month + if month > 2 { -3 } else { 9 }) as i64;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// Parses an RFC 2616 date to Unix seconds (TS: `parseRfc2616Date`).
pub fn parse_rfc2616_date(date: &str) -> Option<f64> {
    // IMF-fixdate subset, e.g. `"Thu, 01 Jan 1970 00:01:00 GMT"`.
    let parts: Vec<&str> = date.split_whitespace().collect();
    if parts.len() != 6 || !parts[0].ends_with(',') || parts[5] != "GMT" {
        return None;
    }
    let day: i64 = parts[1].parse().ok()?;
    let month: i64 = match parts[2] {
        "Jan" => 1,
        "Feb" => 2,
        "Mar" => 3,
        "Apr" => 4,
        "May" => 5,
        "Jun" => 6,
        "Jul" => 7,
        "Aug" => 8,
        "Sep" => 9,
        "Oct" => 10,
        "Nov" => 11,
        "Dec" => 12,
        _ => return None,
    };
    let year: i64 = parts[3].parse().ok()?;
    let time: Vec<&str> = parts[4].split(':').collect();
    if time.len() != 3 {
        return None;
    }
    let hour: i64 = time[0].parse().ok()?;
    let minute: i64 = time[1].parse().ok()?;
    let second: i64 = time[2].parse().ok()?;
    if !(1..=31).contains(&day) || day > days_in_month(year, month) {
        return None;
    }
    if !(0..=23).contains(&hour) || !(0..=59).contains(&minute) || !(0..=60).contains(&second) {
        return None;
    }
    let days = days_from_civil(year, month, day);
    Some(days as f64 * 86_400.0 + hour as f64 * 3_600.0 + minute as f64 * 60.0 + second as f64)
}

/// Formats UTC parts like JS `Date.toString()` in UTC
/// (TS: `jsDateString`, the `X-Timestamp` shape).
pub fn js_date_string(day: &str, month: &str, day_num: &str, year: &str, time: &str) -> String {
    format!("{day} {month} {day_num} {year} {time} GMT+0000 (Coordinated Universal Time)")
}

/// Rotates a 32-bit value left by 16 bits (TS: `rol16`).
pub fn rol16(value: u32) -> u32 {
    value.rotate_left(16)
}

/// Rotates a 32-bit value right by `n` bits (TS: `rotr`).
pub fn rotr(value: u32, n: u32) -> u32 {
    value.rotate_right(n)
}

/// SHA-256 small sigma zero (TS: `sigma0`).
pub fn sigma0_small(value: u32) -> u32 {
    (value >> 7) ^ (value << 25) ^ (value >> 18) ^ (value << 14) ^ (value >> 3)
}

/// SHA-256 small sigma one (TS: `sigma1`).
pub fn sigma1_small(value: u32) -> u32 {
    (value >> 17) ^ (value << 15) ^ (value >> 19) ^ (value << 13) ^ (value >> 10)
}

/// SHA-256 round constants (TS: `SHA256_ROUND_CONSTANTS`).
const SHA256_ROUND_CONSTANTS: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

/// Standard constant with index 14 replaced, as the reference does.
const SHA256_ADJUSTMENT: u32 = 0xc9dc06a7;

/// Block constants: round constants with index 14 replaced
/// (TS: `SHA256_BLOCK_CONSTANTS`).
const fn sha256_block_constants() -> [u32; 64] {
    let mut out = SHA256_ROUND_CONSTANTS;
    out[14] = SHA256_ADJUSTMENT;
    out
}
const SHA256_BLOCK_CONSTANTS: [u32; 64] = sha256_block_constants();

const SHA256_SIGMA0_ADJUSTMENT: u32 = 0x059c0b80;
const SHA256_SIGMA1_ADJUSTMENT: u32 = 0x000b92c0;

/// Modified SHA-256 initial state (TS: `SHA256_INITIAL`).
const SHA256_INITIAL: [u32; 8] = [
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
];

/// Precomputed mid-block state the reference resumes from (TS: `SHA256_STATE`).
const SHA256_STATE: [u32; 8] = [
    0xfd9fbea2, 0x91243ce7, 0xbe499094, 0xcd2e1e0a, 0x6c006f49, 0x04bd35ab, 0xfeb0a7e4, 0x03effbd2,
];

/// Schedule seed words (TS: `SHA256_SCHEDULE`).
const SHA256_SCHEDULE: [u32; 14] = [
    0xeb24bf9e, 0x1558691f, 0xa0c09ee2, 0x0cfa32ba, 0xa622ee8c, 0xd1dd9491, 0xc0dd25a0, 0xf9e70ab6,
    0xf0c31573, 0x7058ce4e, 0x7188e606, 0x00e5ab33, 0xd258b060, 0x72766564,
];

/// Keyed-hash coefficients and offsets (TS: `CS64_COEFFICIENTS`).
const CS64_COEFFICIENTS: [(u32, u32); 14] = [
    (0xe82e91b1, 0xb32700d0),
    (0x44de24c6, 0x88dff51d),
    (0x3beb65bc, 0x29514038),
    (0xc89c5933, 0xb328cbdf),
    (0x8d1ec211, 0xd405b4d5),
    (0x314ef5d5, 0x26e55f3c),
    (0xe92491b1, 0x607801c6),
    (0xf0a5b9d5, 0x82b2c1fb),
    (0x06064cf3, 0xfa51a3da),
    (0xda79fa60, 0x207cc464),
    (0xbed20560, 0x3c3c74f3),
    (0x89f9c9f8, 0xd2d14b61),
    (0x35fa38c6, 0xbb04a2c1),
    (0xcb49e3c6, 0xc5626167),
];
const CS64_FACTOR: u32 = 0x82893933;
const CS64_ADJUSTMENT0: u32 = 0xf2f90b1a;
const CS64_ADJUSTMENT1: u32 = 0x0b1a0000;

/// SHA-256 rounds. `first_round` resumes mid-block: the reference precomputes
/// the first fourteen rounds into the initial state and continues from 14
/// (TS: `sha256Rounds`).
fn sha256_rounds(state: &[u32; 8], words: &[u32], first_round: usize) -> [u32; 8] {
    let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = *state;
    let constants = if first_round == 14 {
        &SHA256_BLOCK_CONSTANTS
    } else {
        &SHA256_ROUND_CONSTANTS
    };
    for index in first_round..64 {
        let t1 = h
            .wrapping_add(rotr(e, 6) ^ rotr(e, 11) ^ rotr(e, 25))
            .wrapping_add(g ^ (e & (f ^ g)))
            .wrapping_add(constants[index])
            .wrapping_add(words[index]);
        let t2 = (rotr(a, 2) ^ rotr(a, 13) ^ rotr(a, 22))
            .wrapping_add((a & b) | (c & (a | b)));
        h = g;
        g = f;
        f = e;
        e = d.wrapping_add(t1);
        d = c;
        c = b;
        b = a;
        a = t1.wrapping_add(t2);
    }
    [a, b, c, d, e, f, g, h]
}

/// SHA-256 message schedule expansion (TS: `expandSchedule`).
fn expand_schedule(words: &mut Vec<u32>, first_word: usize) {
    for index in first_word..64 {
        words.push(
            sigma1_small(words[index - 2])
                .wrapping_add(words[index - 7])
                .wrapping_add(sigma0_small(words[index - 15]))
                .wrapping_add(words[index - 16]),
        );
    }
}

/// Builds the modified SHA-256 initial state the reference derives from the
/// 7-byte salt (TS: `sha256Init`).
fn sha256_init(data: &[u8]) -> [u32; 8] {
    let mut words: Vec<u32> = vec![0; 14];
    words.push(
        (u32::from(data[0]) << 16) | (u32::from(data[1]) << 8) | u32::from(data[2]),
    );
    words.push(
        (u32::from(data[3]) << 24)
            | (u32::from(data[4]) << 16)
            | (u32::from(data[5]) << 8)
            | u32::from(data[6]),
    );
    for index in 16..30 {
        let mut value = SHA256_SCHEDULE[index - 16];
        if index - 2 >= 14 {
            let sigma = sigma1_small(words[index - 2]);
            value = value.wrapping_add(if index == 16 {
                sigma ^ SHA256_SIGMA1_ADJUSTMENT
            } else {
                sigma
            });
        }
        if index - 7 >= 14 {
            value = value.wrapping_add(words[index - 7]);
        }
        if index - 15 >= 14 {
            let sigma = sigma0_small(words[index - 15]);
            value = value.wrapping_add(if index == 29 {
                sigma ^ SHA256_SIGMA0_ADJUSTMENT
            } else {
                sigma
            });
        }
        words.push(value);
    }
    words.push(
        sigma1_small(words[28])
            .wrapping_add(words[23])
            .wrapping_add(sigma0_small(words[15]))
            .wrapping_add(words[14])
            .wrapping_add(SHA256_ADJUSTMENT)
            .wrapping_sub(SHA256_ROUND_CONSTANTS[14]),
    );
    expand_schedule(&mut words, 31);
    let working = sha256_rounds(&SHA256_STATE, &words, 14);
    core::array::from_fn(|index| working[index].wrapping_add(SHA256_INITIAL[index]))
}

/// Modified SHA-256 digest: the first seven bytes seed the state, not the
/// block (TS: `sha256Digest`).
fn sha256_digest(data: &[u8]) -> [u8; 32] {
    let length = 57 + data.len();
    let padding = (55i64 - length as i64).rem_euclid(64) as usize;
    let mut padded: Vec<u8> = Vec::with_capacity(data.len() + 1 + padding + 8);
    padded.extend_from_slice(&data[7..]);
    padded.push(0x80);
    padded.extend(core::iter::repeat(0).take(padding));
    padded.extend_from_slice(&[0u8; 8]);
    let bit_len = (length as u64).wrapping_mul(8).to_be_bytes();
    let tail = padded.len() - 8;
    padded[tail..].copy_from_slice(&bit_len);

    let mut state = sha256_init(&data[..7]);
    for offset in (0..padded.len()).step_by(64) {
        let mut words: Vec<u32> = Vec::with_capacity(64);
        for word in 0..16 {
            let o = offset + word * 4;
            words.push(u32::from_be_bytes([
                padded[o],
                padded[o + 1],
                padded[o + 2],
                padded[o + 3],
            ]));
        }
        expand_schedule(&mut words, 16);
        let working = sha256_rounds(&state, &words, 0);
        for (slot, next) in state.iter_mut().zip(working) {
            *slot = slot.wrapping_add(next);
        }
    }

    let mut digest = [0u8; 32];
    for (index, value) in state.iter().enumerate() {
        digest[index * 4..index * 4 + 4].copy_from_slice(&value.to_be_bytes());
    }
    digest
}

/// Splits data into little-endian 32-bit words, zero-padded to an even count
/// (TS: `wordsLe4Even`).
fn words_le4_even(data: &[u8]) -> Vec<u32> {
    let mut padded = data.to_vec();
    padded.extend(core::iter::repeat(0).take((4 - data.len() % 4) % 4));
    let mut words = Vec::with_capacity(padded.len() / 4 + 1);
    for chunk in padded.chunks_exact(4) {
        words.push(u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
    }
    if words.len() % 2 == 1 {
        words.push(0);
    }
    words
}

/// CS64 digest: the custom keyed hash half of the v2 token
/// (TS: `cs64Digest`).
fn cs64_digest(data: &[u8], digest: &[u8; 32]) -> (u32, u32) {
    let key_a =
        u32::from_le_bytes([digest[0], digest[1], digest[2], digest[3]]) | 1;
    let key_b =
        u32::from_le_bytes([digest[4], digest[5], digest[6], digest[7]]) | 1;
    let factors = [
        key_a.wrapping_mul(CS64_FACTOR),
        key_b.wrapping_mul(CS64_FACTOR),
    ];

    let mut terms: Vec<u32> = Vec::new();
    let mut offsets: Vec<u32> = Vec::new();
    for (index, (coefficient, offset)) in CS64_COEFFICIENTS.iter().enumerate() {
        terms.push(coefficient.wrapping_mul(factors[index % 2]));
        offsets.push(*offset);
    }

    let mut prefixed: Vec<u8> = Vec::with_capacity(data.len() + 1);
    prefixed.push(0);
    prefixed.extend_from_slice(data);
    for (index, word) in words_le4_even(&prefixed).iter().enumerate() {
        let (key, mixer) = if index % 2 == 0 {
            (key_a, 0xef0569fbu32)
        } else {
            (key_b, 0xc31713dbu32)
        };
        let mut term = word.wrapping_mul(key);
        let mut offset = rol16(*word).wrapping_mul(mixer);
        if index == 0 {
            term = term.wrapping_add(CS64_ADJUSTMENT0.wrapping_mul(factors[0]));
            offset = offset.wrapping_add(CS64_ADJUSTMENT1);
        }
        terms.push(term);
        offsets.push(offset);
    }

    let mut swap_acc: u32 = 0;
    let mut swap_last: u32 = 0;
    let mut state: u32 = 0;
    let mut acc: u32 = 0;
    for index in (0..terms.len()).step_by(2) {
        let x = terms[index];
        let y = terms[index + 1];

        let rotated = rol16(offsets[index].wrapping_add(x));
        let q = rotated.wrapping_mul(0x76f50f34);
        swap_acc = swap_acc
            .wrapping_add(rol16(q).wrapping_mul(0xc3efea97))
            .wrapping_add(q);

        let rotated = rol16(offsets[index + 1].wrapping_add(y));
        let q = rotated.wrapping_mul(0x1881ce3c);
        swap_last = rol16(q).wrapping_mul(0x35bd1ec9).wrapping_add(q);
        swap_acc = swap_acc.wrapping_add(swap_last);

        state = state.wrapping_mul(key_a).wrapping_add(x);
        state = rol16(state).wrapping_mul(0xcf98b111);
        state = rol16(state).wrapping_mul(0x87085b9f);
        state = rol16(state).wrapping_mul(0x12ceb96d);
        state = rol16(state).wrapping_mul(0x257e1d83);
        acc = acc.wrapping_add(state);

        let rotated = state.wrapping_mul(key_b).wrapping_add(y);
        let rotated = rol16(rotated).wrapping_mul(0xa27416f5);
        let rotated = rol16(rotated).wrapping_mul(0xd38396ff);
        state = rol16(rol16(rotated).wrapping_mul(0x7c932b89)).wrapping_mul(0xbfa49f69);
        acc = acc.wrapping_add(state);
    }

    (swap_last ^ state, swap_acc ^ acc)
}

/// Joins the CS64 and modified-SHA-256 digests as `<16 hex>-<64 hex>`
/// (TS: `tokenHash`).
fn token_hash(data: &[u8]) -> String {
    let digest = sha256_digest(data);
    let (first, second) = cs64_digest(data, &digest);
    let mut cs64 = [0u8; 8];
    cs64[0..4].copy_from_slice(&first.to_le_bytes());
    cs64[4..8].copy_from_slice(&second.to_le_bytes());
    let head: String = cs64.iter().map(|b| format!("{b:02X}")).collect();
    let tail: String = digest.iter().map(|b| format!("{b:02X}")).collect();
    format!("{head}-{tail}")
}

/// Generates a v2 `Sec-MS-GEC` token for Unix seconds
/// (TS: `generateSecMsGecV2`).
pub fn generate_sec_ms_gec_v2(unix_timestamp_seconds: f64) -> String {
    let mut ticks = unix_timestamp_seconds + WIN_EPOCH_SECONDS;
    ticks -= ticks % ROUND_SECONDS;
    let ticks_int = (ticks * TICKS_PER_UNIX_SECOND).round() as u64;
    let input = format!("{ticks_int}{TRUSTED_CLIENT_TOKEN}");
    token_hash(input.as_bytes())
}
