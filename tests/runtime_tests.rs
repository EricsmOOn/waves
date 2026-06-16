use waves::app::{build_external_session, run_scripted_ticks};
use waves::core::{DecisionSource, ExternalDecisionInput};

#[test]
fn runtime_produces_decisions_and_logs() {
    let mut session =
        build_external_session("sea_survival", "zh-CN", 42, None).expect("session should build");
    run_scripted_ticks(&mut session, 12).expect("run should complete");

    assert_eq!(session.state.tick, 12);
    assert!(session.decisions.len() >= 3);
    assert!(session.logs.len() >= 6);
    assert!(session.domain_events.len() >= 18);
}

#[test]
fn runtime_builds_desert_outpost_from_manifest_entry() {
    let mut session =
        build_external_session("desert_outpost", "zh-CN", 42, None).expect("session should build");
    run_scripted_ticks(&mut session, 8).expect("run should complete");

    assert_eq!(session.config.manifest.id, "desert_outpost");
    assert_eq!(session.config.manifest.entry, "desert_outpost");
    assert_eq!(session.state.tick, 8);
    assert_eq!(session.decisions.len(), 2);
    assert!(session.logs.len() >= 4);
    assert!(session.ui_events.len() >= 4);
    assert!(session.state.stats.hp > 0.0);
}

#[test]
fn runtime_pause_control_does_not_create_decision() {
    let mut session =
        build_external_session("sea_survival", "zh-CN", 42, None).expect("session should build");

    session.toggle_pause().expect("pause toggles");
    assert!(session.paused);
    assert_eq!(session.decisions.len(), 0);
}

#[test]
fn runtime_external_mode_stops_at_pending_decision() {
    let mut session =
        build_external_session("sea_survival", "zh-CN", 42, None).expect("session should build");

    session.run_ticks(4).expect("run should stop at decision");

    let pending = session
        .pending_decision()
        .expect("pending decision should exist");
    assert_eq!(pending.tick, 4);
    assert_eq!(pending.scenario_id, "sea_survival");
    assert!(!pending.actions.is_empty());
    assert_eq!(session.decisions.len(), 0);
}

#[test]
fn runtime_filters_actions_to_event_and_urgent_needs() {
    let mut session =
        build_external_session("sea_survival", "zh-CN", 42, None).expect("session should build");
    session.state.resources.water = 0.2;

    session.run_ticks(4).expect("run should stop at decision");

    let pending = session
        .pending_decision()
        .expect("pending decision should exist");
    let action_ids = pending
        .actions
        .iter()
        .map(|action| action.id.as_str())
        .collect::<Vec<_>>();

    assert!(action_ids.contains(&"collect_rain"));
    assert!(action_ids.contains(&"rest"));
    assert!(!action_ids.contains(&"fish"));
    assert!(!action_ids.contains(&"salvage"));
}

#[test]
fn runtime_eat_food_reduces_hunger_and_consumes_food() {
    let mut session =
        build_external_session("sea_survival", "zh-CN", 42, None).expect("session should build");
    session.state.stats.hunger = 64.0;
    session.state.resources.food = 1.2;
    session.run_ticks(4).expect("run should stop at decision");
    let pending = session
        .pending_decision()
        .expect("pending decision should exist")
        .clone();
    assert!(pending.actions.iter().any(|action| action.id == "eat_food"));
    let hunger_before = session.state.stats.hunger;
    let food_before = session.state.resources.food;

    session
        .submit_external_decision(ExternalDecisionInput {
            decision_id: pending.id,
            action_id: "eat_food".to_string(),
            reason: "Hunger is high, so consume stored food before HP damage starts.".to_string(),
            risk_attitude: Some("cautious".to_string()),
        })
        .expect("eat food should resolve");

    assert!(session.state.stats.hunger < hunger_before);
    assert!(session.state.resources.food < food_before);
    assert!(
        session
            .state
            .memory
            .recent
            .iter()
            .any(|entry| entry.contains("饥饿"))
    );
}

#[test]
fn scripted_decision_reason_uses_active_locale() {
    let mut session =
        build_external_session("sea_survival", "zh-CN", 42, None).expect("session should build");

    run_scripted_ticks(&mut session, 4).expect("run should complete");

    assert_eq!(session.decisions.len(), 1);
    assert_eq!(session.decisions[0].reason, "本地脚本选择第一个可用行动。");
    assert!(!session.decisions[0].reason.contains("Scripted"));
}

#[test]
fn runtime_decision_log_actor_uses_active_locale() {
    let mut session =
        build_external_session("sea_survival", "zh-CN", 42, None).expect("session should build");
    session.run_ticks(4).expect("run should stop at decision");
    let pending = session
        .pending_decision()
        .expect("pending decision should exist")
        .clone();
    let action_id = pending
        .actions
        .first()
        .expect("available action")
        .id
        .clone();

    session
        .submit_external_decision(ExternalDecisionInput {
            decision_id: pending.id,
            action_id,
            reason: "测试原因".to_string(),
            risk_attitude: Some("balanced".to_string()),
        })
        .expect("valid decision should resolve");

    let decision_log = session
        .logs
        .iter()
        .find(|log| log.body.contains("测试原因"))
        .expect("decision log should be recorded");
    assert!(decision_log.body.contains("智能体 选择"));
    assert!(!decision_log.body.contains("Agent"));
}

#[test]
fn runtime_rejects_invalid_external_decision_without_mutation() {
    let mut session =
        build_external_session("sea_survival", "zh-CN", 42, None).expect("session should build");
    session.run_ticks(4).expect("run should stop at decision");
    let pending = session
        .pending_decision()
        .expect("pending decision should exist")
        .clone();
    let state_before = session.state.clone();

    let result = session.submit_external_decision(ExternalDecisionInput {
        decision_id: pending.id.clone(),
        action_id: "missing_action".to_string(),
        reason: "try something impossible".to_string(),
        risk_attitude: Some("balanced".to_string()),
    });

    assert!(result.is_err());
    assert_eq!(session.state, state_before);
    assert_eq!(session.decisions.len(), 0);
    assert_eq!(
        session
            .pending_decision()
            .map(|decision| decision.id.as_str()),
        Some(pending.id.as_str())
    );
}

#[test]
fn runtime_accepts_valid_external_decision() {
    let mut session =
        build_external_session("sea_survival", "zh-CN", 42, None).expect("session should build");
    session.run_ticks(4).expect("run should stop at decision");
    let pending = session
        .pending_decision()
        .expect("pending decision should exist")
        .clone();
    let action_id = pending
        .actions
        .first()
        .expect("available action")
        .id
        .clone();

    let report = session
        .submit_external_decision(ExternalDecisionInput {
            decision_id: pending.id,
            action_id: action_id.clone(),
            reason: "The agent chooses the first available action for this test.".to_string(),
            risk_attitude: Some("balanced".to_string()),
        })
        .expect("valid decision should resolve");

    assert!(session.pending_decision().is_none());
    assert_eq!(session.decisions.len(), 1);
    assert_eq!(session.decisions[0].source, DecisionSource::Agent);
    assert_eq!(session.decisions[0].action_id, action_id);
    assert_eq!(report.decisions.len(), 1);
    assert!(!report.events.is_empty());
}
