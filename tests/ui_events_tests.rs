use waves::app::{build_external_session, run_scripted_ticks};
use waves::core::RiskLevel;
use waves::tui::{UiEventKind, UiPriority};

#[test]
fn scripted_run_emits_decision_and_value_delta_ui_events() {
    let mut session =
        build_external_session("sea_survival", "zh-CN", 42, None).expect("session should build");
    run_scripted_ticks(&mut session, 4).expect("run should complete");

    assert!(
        session
            .ui_events
            .iter()
            .any(|event| event.kind == UiEventKind::Decision)
    );
    assert!(
        session
            .ui_events
            .iter()
            .any(|event| event.kind == UiEventKind::ValueDelta)
    );
    assert!(
        session
            .ui_events
            .iter()
            .any(|event| event.kind == UiEventKind::WorldEvent)
    );
}

#[test]
fn ui_event_lifecycle_expires_deterministically() {
    let event = waves::tui::UiEvent::risk(8, RiskLevel::Critical).with_created_frame(100);
    assert!(event.ttl_frames > 30);
    assert!(event.is_visible(100));
    assert!(event.intensity(100) > event.intensity(100 + event.ttl_frames - 1));
    assert!(!event.is_visible(100 + event.ttl_frames + 1));

    let normal = waves::tui::UiEvent::control(8, "risk", "changed").with_created_frame(100);
    assert!(event.ttl_frames > normal.ttl_frames);
    assert_eq!(normal.priority, UiPriority::Normal);
}

#[test]
fn risk_ui_event_stores_locale_key_instead_of_english_sentence() {
    let event = waves::tui::UiEvent::risk(8, RiskLevel::High);

    assert_eq!(event.target, "risk.high");
    assert_eq!(event.message, "risk.high");
}
