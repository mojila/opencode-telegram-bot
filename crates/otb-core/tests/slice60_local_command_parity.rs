//! Parity port of the local-command-registry pure gates (slice 60, RED).
//!
//! Mirrors the pure remainder in:
//! - `src/app/services/local-command-registry.ts` (name pattern, command
//!   limit, execution timeout, Telegram text limit, truncation suffix,
//!   stderr tail limit, command-file validation, `.json` filter + name
//!   derivation + sort, built-in collision gate, limit gate, leading-slash
//!   lookup for `has` / `allowsWhenBusy`, exec-stripping `definitions`,
//!   `appendBounded` head cap, `appendTail` tail cap, `truncate` cut,
//!   unknown-command `failed` result, close-code success gate, empty-stdout
//!   classification, working-directory `dirname` derivation)
//!
//! Step 1: these tests assert the TS contract and MUST FAIL with
//! `unimplemented!()` panics until the Rust port lands (step 2).

use otb_core::local_command_gates::*;

// --- budgets / markers ---

#[test]
fn command_limit_is_100() {
    assert_eq!(COMMAND_LIMIT, 100);
    assert!(command_limit_reached(60, 40));
}

#[test]
fn command_name_max_len_is_32() {
    assert_eq!(COMMAND_NAME_MAX_LEN, 32);
    assert!(is_valid_command_name(&"a".repeat(32)));
}

#[test]
fn command_description_max_len_is_256() {
    assert_eq!(COMMAND_DESCRIPTION_MAX_LEN, 256);
    assert!(is_valid_command_description(&"d".repeat(256)));
}

#[test]
fn execution_timeout_is_30s() {
    assert_eq!(EXECUTION_TIMEOUT_MS, 30_000);
}

#[test]
fn telegram_text_limit_is_4096() {
    assert_eq!(TELEGRAM_TEXT_LIMIT, 4096);
    assert_eq!(truncate_output(&"x".repeat(4096)), "x".repeat(4096));
}

#[test]
fn truncation_suffix_value() {
    assert_eq!(TRUNCATION_SUFFIX, "\n… [truncated]");
}

#[test]
fn stderr_tail_limit_is_1024() {
    assert_eq!(STDERR_TAIL_LIMIT, 1024);
}

// --- COMMAND_NAME_PATTERN ---

#[test]
fn valid_command_names() {
    assert!(is_valid_command_name("deploy"));
    assert!(is_valid_command_name("deploy2"));
    assert!(is_valid_command_name("my_cmd"));
    assert!(is_valid_command_name("a"));
}

#[test]
fn command_name_rejects_empty() {
    assert!(!is_valid_command_name(""));
}

#[test]
fn command_name_rejects_too_long() {
    assert!(!is_valid_command_name(&"a".repeat(33)));
}

#[test]
fn command_name_rejects_uppercase() {
    assert!(!is_valid_command_name("Deploy"));
}

#[test]
fn command_name_rejects_dash_and_dot() {
    assert!(!is_valid_command_name("my-cmd"));
    assert!(!is_valid_command_name("my.cmd"));
}

#[test]
fn command_name_rejects_spaces() {
    assert!(!is_valid_command_name("my cmd"));
}

#[test]
fn command_name_rejects_slash_prefix() {
    assert!(!is_valid_command_name("/deploy"));
}

// --- isCommandFile: description ---

#[test]
fn valid_description() {
    assert!(is_valid_command_description("Run the deploy"));
}

#[test]
fn description_rejects_empty() {
    assert!(!is_valid_command_description(""));
}

#[test]
fn description_rejects_too_long() {
    assert!(!is_valid_command_description(&"d".repeat(257)));
}

#[test]
fn description_rejects_newlines() {
    assert!(!is_valid_command_description("line1\nline2"));
    assert!(!is_valid_command_description("line1\rline2"));
}

// --- isCommandFile: exec ---

#[test]
fn valid_exec() {
    assert!(is_valid_command_exec("npm run deploy"));
}

#[test]
fn exec_rejects_blank() {
    assert!(!is_valid_command_exec(""));
    assert!(!is_valid_command_exec("   "));
}

// --- isCommandFile: full ---

fn raw(desc: Option<&str>, exec: Option<&str>, busy: Option<bool>, is_bool: bool) -> RawCommandFile {
    RawCommandFile {
        description: desc.map(str::to_string),
        exec: exec.map(str::to_string),
        allow_when_busy: busy,
        allow_when_busy_is_bool: is_bool,
    }
}

#[test]
fn valid_command_file() {
    assert!(is_command_file(&raw(Some("Deploy"), Some("npm run deploy"), None, true)));
}

#[test]
fn command_file_with_busy_flag() {
    assert!(is_command_file(&raw(
        Some("Deploy"),
        Some("./run.sh"),
        Some(true),
        true
    )));
}

#[test]
fn command_file_rejects_missing_description() {
    assert!(!is_command_file(&raw(None, Some("run"), None, true)));
}

#[test]
fn command_file_rejects_missing_exec() {
    assert!(!is_command_file(&raw(Some("Deploy"), None, None, true)));
}

#[test]
fn command_file_rejects_bad_description() {
    assert!(!is_command_file(&raw(Some(""), Some("run"), None, true)));
}

#[test]
fn command_file_rejects_non_bool_busy() {
    assert!(!is_command_file(&raw(Some("Deploy"), Some("run"), None, false)));
}

// --- .json filter + name derivation + sort ---

#[test]
fn json_file_filter() {
    assert!(is_command_json_file(true, "deploy.json"));
    assert!(!is_command_json_file(true, "deploy.js"));
    assert!(!is_command_json_file(false, "deploy.json"));
}

#[test]
fn command_name_from_json() {
    assert_eq!(
        command_name_from_file_name("deploy.json"),
        Some("deploy".to_string())
    );
}

#[test]
fn command_name_from_non_json_is_none() {
    assert_eq!(command_name_from_file_name("deploy.js"), None);
}

#[test]
fn sort_file_names_byte_order() {
    let names = vec!["b.json".to_string(), "a.json".to_string(), "c.json".to_string()];
    assert_eq!(
        sort_command_file_names(&names),
        vec!["a.json".to_string(), "b.json".to_string(), "c.json".to_string()]
    );
}

// --- collision + limit gates ---

#[test]
fn builtin_collision_gate() {
    let builtins = vec!["start".to_string(), "help".to_string()];
    assert!(is_builtin_collision(&builtins, "start"));
    assert!(!is_builtin_collision(&builtins, "deploy"));
}

#[test]
fn limit_gate_boundary() {
    assert!(!command_limit_reached(90, 9));
    assert!(command_limit_reached(90, 10));
    assert!(command_limit_reached(100, 0));
}

// --- leading-slash lookup ---

#[test]
fn strip_prefix_removes_slash() {
    assert_eq!(strip_command_prefix("/deploy"), "deploy");
}

#[test]
fn strip_prefix_without_slash_passthrough() {
    assert_eq!(strip_command_prefix("deploy"), "deploy");
}

fn loaded() -> Vec<LoadedCommand> {
    vec![
        LoadedCommand {
            command: "deploy".to_string(),
            description: "Deploy".to_string(),
            exec: "npm run deploy".to_string(),
            allow_when_busy: true,
        },
        LoadedCommand {
            command: "build".to_string(),
            description: "Build".to_string(),
            exec: "npm run build".to_string(),
            allow_when_busy: false,
        },
    ]
}

#[test]
fn has_command_lookup() {
    let cmds = loaded();
    assert!(has_command(&cmds, Some("/deploy")));
    assert!(!has_command(&cmds, Some("/missing")));
    assert!(!has_command(&cmds, None));
}

#[test]
fn allows_when_busy_lookup() {
    let cmds = loaded();
    assert!(allows_when_busy(&cmds, Some("/deploy")));
    assert!(!allows_when_busy(&cmds, Some("/build")));
    assert!(!allows_when_busy(&cmds, Some("/missing")));
    assert!(!allows_when_busy(&cmds, None));
}

#[test]
fn definitions_strip_exec() {
    let defs = definitions_from_loaded(&loaded());
    assert_eq!(
        defs,
        vec![
            LocalCommandDefinition {
                command: "deploy".to_string(),
                description: "Deploy".to_string(),
                allow_when_busy: true,
            },
            LocalCommandDefinition {
                command: "build".to_string(),
                description: "Build".to_string(),
                allow_when_busy: false,
            },
        ]
    );
}

#[test]
fn resolve_working_directory_dirname() {
    assert_eq!(resolve_working_directory("/app/commands"), "/app".to_string());
}

// --- appendBounded ---

#[test]
fn append_bounded_under_limit() {
    assert_eq!(append_bounded("ab", "cd", 10), "abcd".to_string());
}

#[test]
fn append_bounded_caps_at_limit() {
    assert_eq!(append_bounded("ab", "cdef", 4), "abcd".to_string());
}

#[test]
fn append_bounded_already_full_keeps_current() {
    assert_eq!(append_bounded("abcd", "ef", 4), "abcd".to_string());
}

// --- appendTail ---

#[test]
fn append_tail_under_limit() {
    assert_eq!(append_tail("ab", "cd", 10), "abcd".to_string());
}

#[test]
fn append_tail_keeps_last_chars() {
    assert_eq!(append_tail("abcdef", "gh", 4), "efgh".to_string());
}

// --- truncate ---

#[test]
fn truncate_short_passthrough() {
    assert_eq!(truncate_output("hi"), "hi".to_string());
}

#[test]
fn truncate_long_appends_suffix() {
    let long = "x".repeat(5000);
    let cut = truncate_output(&long);
    assert!(cut.ends_with(TRUNCATION_SUFFIX));
    assert_eq!(cut.chars().count(), TELEGRAM_TEXT_LIMIT);
}

// --- result classification ---

#[test]
fn unknown_command_is_failed() {
    assert_eq!(
        result_for_unknown_command(),
        LocalCommandResult::Failed {
            exit_code: None,
            stderr: String::new(),
        }
    );
}

#[test]
fn close_code_success_gate() {
    assert!(is_success_close_code(0));
    assert!(!is_success_close_code(1));
}

#[test]
fn empty_stdout_classifies_empty() {
    assert_eq!(classify_stdout_result(""), LocalCommandResult::Empty);
}

#[test]
fn non_empty_stdout_classifies_success() {
    let text = "done";
    assert_eq!(
        classify_stdout_result(text),
        LocalCommandResult::Success {
            text: text.to_string()
        }
    );
}

#[test]
fn timeout_variant_exists() {
    assert_eq!(
        LocalCommandResult::Timeout,
        LocalCommandResult::Timeout
    );
}
