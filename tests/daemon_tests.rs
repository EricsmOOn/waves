use serde_json::{Value, json};
use waves::app::build_external_session;
use waves::daemon::{SessionHost, SessionSnapshot};

#[test]
fn daemon_host_preserves_session_across_step_and_decision() {
    let session =
        build_external_session("sea_survival", "zh-CN", 42, None).expect("session should build");
    let mut host = SessionHost::with_session(session);

    let step_value = host
        .handle("step", json!({ "ticks": 4 }))
        .expect("step should succeed");
    let step_snapshot: SessionSnapshot =
        serde_json::from_value(step_value).expect("step should return snapshot");
    let pending = step_snapshot
        .pending_decision
        .expect("step should stop at pending decision");
    let action_id = pending
        .actions
        .first()
        .expect("pending decision should include actions")
        .id
        .clone();

    let submit_value = host
        .handle(
            "submit_decision",
            json!({
                "decision_id": pending.id,
                "action_id": action_id,
                "reason": "The agent selects the first available action for the daemon test.",
                "risk_attitude": "balanced"
            }),
        )
        .expect("submit decision should succeed");
    let submit_state: SessionSnapshot =
        state_from_submit(submit_value).expect("submit should include state");

    assert!(submit_state.pending_decision.is_none());
    assert_eq!(submit_state.counts.decisions, 1);
    assert_eq!(submit_state.decisions.len(), 1);
}

#[test]
fn daemon_get_state_advances_presentation_frame_only_when_requested() {
    let session =
        build_external_session("sea_survival", "zh-CN", 42, None).expect("session should build");
    let mut host = SessionHost::with_session(session);

    let first: SessionSnapshot = serde_json::from_value(
        host.handle("get_state", json!({}))
            .expect("get_state should succeed"),
    )
    .expect("get_state should return snapshot");
    let second: SessionSnapshot = serde_json::from_value(
        host.handle("get_state", json!({}))
            .expect("get_state should succeed"),
    )
    .expect("get_state should return snapshot");
    let advanced: SessionSnapshot = serde_json::from_value(
        host.handle("get_state", json!({ "advance_frame": true }))
            .expect("get_state should succeed"),
    )
    .expect("get_state should return snapshot");

    assert_eq!(first.presentation_frame, 0);
    assert_eq!(second.presentation_frame, 0);
    assert_eq!(advanced.presentation_frame, 1);
}

fn state_from_submit(value: Value) -> serde_json::Result<SessionSnapshot> {
    serde_json::from_value(value["state"].clone())
}
