//! Slice 72 parity: goal-service pure gates (RED).
//!
//! Mirrors the pure TS contract in `src/app/services/goal-service.ts`:
//! - `parseGoalState` (record/shape validation, optional non-empty reason)
//! - `isUnsupported` (name match, rpc-only type match)
//! - `messageOf` (Error / string / record `data.message` / `message` /
//!   `name`, unavailable fallback)
//! - `readFailure` split, `available` version check, `refresh` guards,
//!   `applyChanged` routing, `command` error mapping
//!
//! Every test MUST FAIL until `goal_service_gates` is ported (step 2).

use otb_core::goal_service_gates::*;

fn obj(fields: Vec<(&str, JsonVal)>) -> JsonVal {
    JsonVal::Obj(
        fields
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect(),
    )
}

fn valid_goal() -> JsonVal {
    obj(vec![
        ("id", JsonVal::Str("g1".to_string())),
        ("objective", JsonVal::Str("Ship it".to_string())),
        ("status", JsonVal::Str("active".to_string())),
        ("rounds", JsonVal::Num(1.0)),
        ("maxRounds", JsonVal::Num(10.0)),
        ("updatedAt", JsonVal::Num(42.0)),
    ])
}

fn with_field(base: JsonVal, key: &str, value: JsonVal) -> JsonVal {
    match base {
        JsonVal::Obj(mut fields) => {
            fields.retain(|(k, _)| k != key);
            fields.push((key.to_string(), value));
            JsonVal::Obj(fields)
        }
        other => other,
    }
}

fn without_field(base: JsonVal, key: &str) -> JsonVal {
    match base {
        JsonVal::Obj(fields) => {
            JsonVal::Obj(fields.into_iter().filter(|(k, _)| k != key).collect())
        }
        other => other,
    }
}

// --- parse_goal_state: non-records ---

#[test]
fn slice72_parse_null_none_parity() {
    assert_eq!(parse_goal_state(&JsonVal::Null), None);
}

#[test]
fn slice72_parse_bool_none_parity() {
    assert_eq!(parse_goal_state(&JsonVal::Bool(true)), None);
}

#[test]
fn slice72_parse_num_none_parity() {
    assert_eq!(parse_goal_state(&JsonVal::Num(1.0)), None);
}

#[test]
fn slice72_parse_str_none_parity() {
    assert_eq!(parse_goal_state(&JsonVal::Str("goal".to_string())), None);
}

#[test]
fn slice72_parse_arr_none_parity() {
    assert_eq!(parse_goal_state(&JsonVal::Arr(vec![])), None);
}

#[test]
fn slice72_parse_empty_obj_none_parity() {
    assert_eq!(parse_goal_state(&obj(vec![])), None);
}

// --- parse_goal_state: shape validation ---

#[test]
fn slice72_parse_bad_status_none_parity() {
    let v = with_field(valid_goal(), "status", JsonVal::Str("done".to_string()));
    assert_eq!(parse_goal_state(&v), None);
}

#[test]
fn slice72_parse_missing_status_none_parity() {
    assert_eq!(
        parse_goal_state(&without_field(valid_goal(), "status")),
        None
    );
}

#[test]
fn slice72_parse_non_string_id_none_parity() {
    let v = with_field(valid_goal(), "id", JsonVal::Num(7.0));
    assert_eq!(parse_goal_state(&v), None);
}

#[test]
fn slice72_parse_non_string_objective_none_parity() {
    let v = with_field(valid_goal(), "objective", JsonVal::Bool(false));
    assert_eq!(parse_goal_state(&v), None);
}

#[test]
fn slice72_parse_non_number_rounds_none_parity() {
    let v = with_field(valid_goal(), "rounds", JsonVal::Str("1".to_string()));
    assert_eq!(parse_goal_state(&v), None);
}

#[test]
fn slice72_parse_non_number_max_rounds_none_parity() {
    let v = with_field(valid_goal(), "maxRounds", JsonVal::Null);
    assert_eq!(parse_goal_state(&v), None);
}

#[test]
fn slice72_parse_non_number_updated_at_none_parity() {
    let v = with_field(valid_goal(), "updatedAt", JsonVal::Str("now".to_string()));
    assert_eq!(parse_goal_state(&v), None);
}

// --- parse_goal_state: happy paths ---

#[test]
fn slice72_parse_minimal_ok_no_reason_parity() {
    assert_eq!(
        parse_goal_state(&valid_goal()),
        Some(GoalStateView {
            id: "g1".to_string(),
            objective: "Ship it".to_string(),
            status: GoalStatusView::Active,
            reason: None,
            rounds: 1.0,
            max_rounds: 10.0,
            updated_at: 42.0,
        })
    );
}

#[test]
fn slice72_parse_reason_kept_parity() {
    let v = with_field(
        valid_goal(),
        "reason",
        JsonVal::Str("Paused by the user".to_string()),
    );
    let parsed = parse_goal_state(&v).expect("reasoned goal parses");
    assert_eq!(parsed.reason.as_deref(), Some("Paused by the user"));
}

// --- message_of ---

#[test]
fn slice72_message_error_instance_parity() {
    let e = FailVal::ErrorInstance {
        message: "boom".to_string(),
    };
    assert_eq!(message_of(&e), "boom");
}

#[test]
fn slice72_message_string_parity() {
    assert_eq!(message_of(&FailVal::Text("plain".to_string())), "plain");
}

#[test]
fn slice72_message_record_data_message_first_parity() {
    let e = FailVal::Record {
        name: Some("E".to_string()),
        error_type: None,
        message: Some("outer".to_string()),
        data_message: Some("inner".to_string()),
    };
    assert_eq!(message_of(&e), "inner");
}

#[test]
fn slice72_message_record_message_fallback_parity() {
    let e = FailVal::Record {
        name: Some("E".to_string()),
        error_type: None,
        message: Some("outer".to_string()),
        data_message: None,
    };
    assert_eq!(message_of(&e), "outer");
}

#[test]
fn slice72_message_record_name_fallback_parity() {
    let e = FailVal::Record {
        name: Some("Named".to_string()),
        error_type: None,
        message: None,
        data_message: None,
    };
    assert_eq!(message_of(&e), "Named");
}

#[test]
fn slice72_message_null_empty_parity() {
    assert_eq!(message_of(&FailVal::Null), "");
}

#[test]
fn slice72_message_unavailable_marker_fallback_parity() {
    let e = FailVal::ErrorInstance {
        message: "fetch failed".to_string(),
    };
    assert_eq!(message_of(&e), "fetch failed");
}

#[test]
fn slice72_message_blank_record_empty_parity() {
    let e = FailVal::Record {
        name: None,
        error_type: None,
        message: None,
        data_message: None,
    };
    assert_eq!(message_of(&e), "");
}

// --- is_unsupported ---

#[test]
fn slice72_unsupported_name_rpc_true_parity() {
    let e = FailVal::Record {
        name: Some("GoalUnsupportedError".to_string()),
        error_type: None,
        message: None,
        data_message: None,
    };
    assert!(is_unsupported(&e, UnsupportedSource::Rpc));
}

#[test]
fn slice72_unsupported_name_command_true_parity() {
    let e = FailVal::Record {
        name: Some("GoalUnsupportedError".to_string()),
        error_type: None,
        message: None,
        data_message: None,
    };
    assert!(is_unsupported(&e, UnsupportedSource::Command));
}

#[test]
fn slice72_unsupported_rpc_method_not_found_parity() {
    let e = FailVal::Record {
        name: None,
        error_type: Some("rpc.method_not_found".to_string()),
        message: None,
        data_message: None,
    };
    assert!(is_unsupported(&e, UnsupportedSource::Rpc));
}

#[test]
fn slice72_unsupported_rpc_unavailable_parity() {
    let e = FailVal::Record {
        name: None,
        error_type: Some("rpc.unavailable".to_string()),
        message: None,
        data_message: None,
    };
    assert!(is_unsupported(&e, UnsupportedSource::Rpc));
}

#[test]
fn slice72_unsupported_command_type_ignored_parity() {
    let e = FailVal::Record {
        name: None,
        error_type: Some("rpc.method_not_found".to_string()),
        message: None,
        data_message: None,
    };
    assert!(!is_unsupported(&e, UnsupportedSource::Command));
}

#[test]
fn slice72_unsupported_plain_error_false_parity() {
    let e = FailVal::ErrorInstance {
        message: "nope".to_string(),
    };
    assert!(!is_unsupported(&e, UnsupportedSource::Rpc));
}

#[test]
fn slice72_unsupported_other_type_false_parity() {
    let e = FailVal::Record {
        name: None,
        error_type: Some("rpc.other".to_string()),
        message: None,
        data_message: None,
    };
    assert!(!is_unsupported(&e, UnsupportedSource::Rpc));
}

#[test]
fn slice72_unsupported_null_false_parity() {
    assert!(!is_unsupported(&FailVal::Null, UnsupportedSource::Rpc));
}

#[test]
fn slice72_unsupported_text_false_parity() {
    let e = FailVal::Text("GoalUnsupportedError".to_string());
    assert!(!is_unsupported(&e, UnsupportedSource::Rpc));
}

// --- classify_read_failure / is_available ---

#[test]
fn slice72_read_failure_unsupported_parity() {
    let e = FailVal::Record {
        name: Some("GoalUnsupportedError".to_string()),
        error_type: None,
        message: None,
        data_message: None,
    };
    assert_eq!(classify_read_failure(&e), ReadFailure::Unsupported);
}

#[test]
fn slice72_read_failure_error_message_parity() {
    let e = FailVal::ErrorInstance {
        message: "boom".to_string(),
    };
    assert_eq!(
        classify_read_failure(&e),
        ReadFailure::Error {
            message: "boom".to_string()
        }
    );
}

#[test]
fn slice72_read_failure_error_empty_message_parity() {
    assert_eq!(
        classify_read_failure(&FailVal::Null),
        ReadFailure::Error {
            message: String::new()
        }
    );
}

#[test]
fn slice72_available_v2_true_parity() {
    assert!(is_available("v2"));
}

#[test]
fn slice72_available_v1_false_parity() {
    assert!(!is_available("v1"));
}

#[test]
fn slice72_available_empty_false_parity() {
    assert!(!is_available(""));
}

// --- refresh_guard / decide_apply_changed / map_command_error ---

#[test]
fn slice72_refresh_proceed_parity() {
    assert_eq!(
        refresh_guard(Some("s1"), "s1", "tok", "tok"),
        RefreshGuard::Proceed
    );
}

#[test]
fn slice72_refresh_wrong_session_parity() {
    assert_eq!(
        refresh_guard(Some("s2"), "s1", "tok", "tok"),
        RefreshGuard::WrongSession
    );
}

#[test]
fn slice72_refresh_stale_token_parity() {
    assert_eq!(
        refresh_guard(Some("s1"), "s1", "before", "after"),
        RefreshGuard::StaleToken
    );
}

#[test]
fn slice72_apply_changed_wrong_session_ignore_parity() {
    assert_eq!(
        decide_apply_changed(Some("s2"), "s1", None),
        ApplyChangedDecision::Ignore
    );
}

#[test]
fn slice72_apply_changed_null_clears_parity() {
    assert_eq!(
        decide_apply_changed(Some("s1"), "s1", None),
        ApplyChangedDecision::Clear
    );
}

#[test]
fn slice72_apply_changed_valid_applies_parity() {
    let goal = valid_goal();
    assert_eq!(
        decide_apply_changed(Some("s1"), "s1", Some(&goal)),
        ApplyChangedDecision::Apply(GoalStateView {
            id: "g1".to_string(),
            objective: "Ship it".to_string(),
            status: GoalStatusView::Active,
            reason: None,
            rounds: 1.0,
            max_rounds: 10.0,
            updated_at: 42.0,
        })
    );
}

#[test]
fn slice72_apply_changed_bad_shape_ignore_parity() {
    let bad = obj(vec![("id", JsonVal::Str("g1".to_string()))]);
    assert_eq!(
        decide_apply_changed(Some("s1"), "s1", Some(&bad)),
        ApplyChangedDecision::Ignore
    );
}

#[test]
fn slice72_command_unsupported_parity() {
    assert_eq!(
        map_command_error(true, "whatever"),
        CommandErrorDecision::Unsupported
    );
}

#[test]
fn slice72_command_rejected_message_parity() {
    assert_eq!(
        map_command_error(false, "nope"),
        CommandErrorDecision::Rejected {
            message: "nope".to_string()
        }
    );
}

#[test]
fn slice72_command_empty_needs_fallback_parity() {
    assert_eq!(
        map_command_error(false, ""),
        CommandErrorDecision::ErrorNeedsFallback
    );
}
