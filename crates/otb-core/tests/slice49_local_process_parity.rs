//! Parity port of the local-process pure gates (slice 49, RED).
//!
//! Mirrors the pure remainder in:
//! - `src/opencode/process.ts` (`resolveLocalOpencodeTarget` URL /
//!   local-host / port gates, `parseOpencodeVersionOutput` first-triple
//!   match, `getOpencodeApiVersion` parseInt-major gate,
//!   `findWindowsListeningPidInNetstat` / `findUnixListeningPidInSs`
//!   line scans, `parseUnixPidList` first-valid pid)
//!
//! Step 1: these tests assert the TS contract and MUST FAIL with
//! `unimplemented!()` panics until the Rust port lands (step 2).

use otb_core::local_process_gates::*;

// --- resolve_local_opencode_target (01-12) ---

#[test]
fn slice49_01_localhost_defaults_to_4096() {
    assert_eq!(
        resolve_local_opencode_target("http://localhost"),
        Some(LocalOpencodeTarget {
            host: "localhost".to_string(),
            port: 4096,
        })
    );
}

#[test]
fn slice49_02_explicit_port_is_kept() {
    assert_eq!(
        resolve_local_opencode_target("http://localhost:5000"),
        Some(LocalOpencodeTarget {
            host: "localhost".to_string(),
            port: 5000,
        })
    );
}

#[test]
fn slice49_03_loopback_ipv4_is_local() {
    assert_eq!(
        resolve_local_opencode_target("http://127.0.0.1:4096"),
        Some(LocalOpencodeTarget {
            host: "127.0.0.1".to_string(),
            port: 4096,
        })
    );
}

#[test]
fn slice49_04_loopback_ipv6_with_path_and_query() {
    assert_eq!(
        resolve_local_opencode_target("http://[::1]:4096/some/path?x=1"),
        Some(LocalOpencodeTarget {
            host: "::1".to_string(),
            port: 4096,
        })
    );
}

#[test]
fn slice49_05_wildcard_host_is_local() {
    assert_eq!(
        resolve_local_opencode_target("http://0.0.0.0:8080"),
        Some(LocalOpencodeTarget {
            host: "0.0.0.0".to_string(),
            port: 8080,
        })
    );
}

#[test]
fn slice49_06_hostname_case_is_normalized() {
    assert_eq!(
        resolve_local_opencode_target("HTTP://LOCALHOST:4096"),
        Some(LocalOpencodeTarget {
            host: "localhost".to_string(),
            port: 4096,
        })
    );
}

#[test]
fn slice49_07_remote_host_is_none() {
    assert_eq!(
        resolve_local_opencode_target("https://example.com:4096"),
        None
    );
}

#[test]
fn slice49_08_garbage_url_is_none() {
    assert_eq!(resolve_local_opencode_target("not a url"), None);
}

#[test]
fn slice49_09_empty_url_is_none() {
    assert_eq!(resolve_local_opencode_target(""), None);
}

#[test]
fn slice49_10_port_zero_is_none() {
    assert_eq!(resolve_local_opencode_target("http://localhost:0"), None);
}

#[test]
fn slice49_11_non_numeric_port_is_none() {
    assert_eq!(resolve_local_opencode_target("http://localhost:abc"), None);
}

#[test]
fn slice49_12_out_of_range_port_is_none() {
    assert_eq!(resolve_local_opencode_target("http://localhost:99999"), None);
}

// --- parse_opencode_version_output (13-19) ---

#[test]
fn slice49_13_plain_triple() {
    assert_eq!(
        parse_opencode_version_output("1.2.3"),
        Some("1.2.3".to_string())
    );
}

#[test]
fn slice49_14_version_inside_banner() {
    assert_eq!(
        parse_opencode_version_output("opencode v2.0.1 (abc123)"),
        Some("2.0.1".to_string())
    );
}

#[test]
fn slice49_15_first_triple_wins() {
    assert_eq!(
        parse_opencode_version_output("version 10.20.30 extra 1.2.3"),
        Some("10.20.30".to_string())
    );
}

#[test]
fn slice49_16_multiline_picks_first() {
    assert_eq!(
        parse_opencode_version_output("release-1.2.3\nbuild 4.5.6"),
        Some("1.2.3".to_string())
    );
}

#[test]
fn slice49_17_empty_is_none() {
    assert_eq!(parse_opencode_version_output(""), None);
}

#[test]
fn slice49_18_short_version_is_none() {
    assert_eq!(parse_opencode_version_output("v2"), None);
}

#[test]
fn slice49_19_two_part_version_is_none() {
    assert_eq!(parse_opencode_version_output("1.2"), None);
}

// --- get_opencode_api_version (20-28) ---

#[test]
fn slice49_20_two_x_is_v2() {
    assert_eq!(get_opencode_api_version("2.0.0"), "v2");
}

#[test]
fn slice49_21_one_x_is_v1() {
    assert_eq!(get_opencode_api_version("1.9.9"), "v1");
}

#[test]
fn slice49_22_ten_x_is_v2() {
    assert_eq!(get_opencode_api_version("10.0.0"), "v2");
}

#[test]
fn slice49_23_zero_major_is_v1() {
    assert_eq!(get_opencode_api_version("0.9.1"), "v1");
}

#[test]
fn slice49_24_empty_is_v1() {
    assert_eq!(get_opencode_api_version(""), "v1");
}

#[test]
fn slice49_25_non_numeric_is_v1() {
    assert_eq!(get_opencode_api_version("abc"), "v1");
}

#[test]
fn slice49_26_leading_whitespace_is_skipped() {
    assert_eq!(get_opencode_api_version("  3.1.0"), "v2");
}

#[test]
fn slice49_27_leading_v_has_no_digits_so_v1() {
    assert_eq!(get_opencode_api_version("v2.1.0"), "v1");
}

#[test]
fn slice49_28_bare_major_two_is_v2() {
    assert_eq!(get_opencode_api_version("2"), "v2");
}

// --- find_windows_listening_pid_in_netstat (29-37) ---

const WIN_BASIC: &str = "  TCP    127.0.0.1:4096         0.0.0.0:0              LISTENING       1234";

#[test]
fn slice49_29_windows_basic_match() {
    assert_eq!(
        find_windows_listening_pid_in_netstat(WIN_BASIC, 4096),
        Some(1234)
    );
}

#[test]
fn slice49_30_windows_wrong_port_is_none() {
    assert_eq!(find_windows_listening_pid_in_netstat(WIN_BASIC, 5000), None);
}

#[test]
fn slice49_31_windows_empty_is_none() {
    assert_eq!(find_windows_listening_pid_in_netstat("", 4096), None);
}

#[test]
fn slice49_32_windows_header_lines_skipped() {
    let stdout = "Active Connections\n\n  Proto  Local Address          Foreign Address        State           PID\n  TCP    127.0.0.1:4096         0.0.0.0:0              LISTENING       1234";
    assert_eq!(
        find_windows_listening_pid_in_netstat(stdout, 4096),
        Some(1234)
    );
}

#[test]
fn slice49_33_windows_bad_pid_line_skipped_for_later_hit() {
    let stdout = "  TCP    127.0.0.1:4096         0.0.0.0:0              LISTENING       -\n  TCP    0.0.0.0:4096           0.0.0.0:0              LISTENING       5678";
    assert_eq!(
        find_windows_listening_pid_in_netstat(stdout, 4096),
        Some(5678)
    );
}

#[test]
fn slice49_34_windows_zero_pid_skipped() {
    let stdout = "  TCP    127.0.0.1:4096         0.0.0.0:0              LISTENING       0";
    assert_eq!(find_windows_listening_pid_in_netstat(stdout, 4096), None);
}

#[test]
fn slice49_35_windows_crlf_line_endings() {
    let stdout = "  TCP    127.0.0.1:4096         0.0.0.0:0              LISTENING       1234\r\n  TCP    127.0.0.1:5000         0.0.0.0:0              LISTENING       9999\r\n";
    assert_eq!(
        find_windows_listening_pid_in_netstat(stdout, 4096),
        Some(1234)
    );
}

#[test]
fn slice49_36_windows_protocol_agnostic_columns() {
    let stdout = "  UDP    0.0.0.0:4096           *:*                                    4321";
    assert_eq!(
        find_windows_listening_pid_in_netstat(stdout, 4096),
        Some(4321)
    );
}

#[test]
fn slice49_37_windows_port_must_match_exactly() {
    let stdout = "  TCP    127.0.0.1:14096        0.0.0.0:0              LISTENING       1234";
    assert_eq!(find_windows_listening_pid_in_netstat(stdout, 4096), None);
}

// --- find_unix_listening_pid_in_ss (38-44) ---

const SS_BASIC: &str =
    "tcp   ESTAB  0  0  127.0.0.1:4096  0.0.0.0:*  users:((\"node\",pid=4321,fd=3))";

#[test]
fn slice49_38_ss_basic_match() {
    assert_eq!(find_unix_listening_pid_in_ss(SS_BASIC, 4096), Some(4321));
}

#[test]
fn slice49_39_ss_header_skipped() {
    let stdout = "Netid State Recv-Q Send-Q Local Address:Port Peer Address:Port Process\ntcp   ESTAB  0  0  127.0.0.1:4096  0.0.0.0:*  users:((\"node\",pid=4321,fd=3))";
    assert_eq!(
        find_unix_listening_pid_in_ss(stdout, 4096),
        Some(4321)
    );
}

#[test]
fn slice49_40_ss_wrong_port_is_none() {
    assert_eq!(find_unix_listening_pid_in_ss(SS_BASIC, 5000), None);
}

#[test]
fn slice49_41_ss_port_match_without_pid_skipped() {
    let stdout = "tcp   ESTAB  0  0  127.0.0.1:4096  0.0.0.0:*\ntcp   ESTAB  0  0  127.0.0.1:4096  0.0.0.0:*  users:((\"node\",pid=777,fd=3))";
    assert_eq!(find_unix_listening_pid_in_ss(stdout, 4096), Some(777));
}

#[test]
fn slice49_42_ss_zero_pid_is_none() {
    let stdout = "tcp   ESTAB  0  0  127.0.0.1:4096  0.0.0.0:*  users:((\"node\",pid=0,fd=3))";
    assert_eq!(find_unix_listening_pid_in_ss(stdout, 4096), None);
}

#[test]
fn slice49_43_ss_empty_is_none() {
    assert_eq!(find_unix_listening_pid_in_ss("", 4096), None);
}

#[test]
fn slice49_44_ss_first_pid_in_line_wins() {
    let stdout = "tcp   ESTAB  0  0  127.0.0.1:4096  0.0.0.0:*  users:((\"a\",pid=111,fd=3),(\"b\",pid=222,fd=4))";
    assert_eq!(find_unix_listening_pid_in_ss(stdout, 4096), Some(111));
}

// --- parse_unix_pid_list (45-48) ---

#[test]
fn slice49_45_pid_list_single() {
    assert_eq!(parse_unix_pid_list("1234\n"), Some(1234));
}

#[test]
fn slice49_46_pid_list_first_valid_wins() {
    assert_eq!(parse_unix_pid_list("abc\n5678\n9012\n"), Some(5678));
}

#[test]
fn slice49_47_pid_list_garbage_is_none() {
    assert_eq!(parse_unix_pid_list("abc\n\n   \n"), None);
}

#[test]
fn slice49_48_pid_list_zero_and_negative_rejected() {
    assert_eq!(parse_unix_pid_list("0\n-5\n"), None);
}
