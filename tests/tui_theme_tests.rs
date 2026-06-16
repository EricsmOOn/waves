use waves::core::RiskLevel;
use waves::tui::{StyleRole, Theme, UiEvent};

#[test]
fn theme_maps_delta_to_semantic_roles() {
    assert_eq!(Theme::role_for_delta(1.0), StyleRole::Positive);
    assert_eq!(Theme::role_for_delta(-1.0), StyleRole::Negative);
    assert_eq!(Theme::role_for_delta(0.0), StyleRole::Muted);
}

#[test]
fn theme_maps_risk_to_semantic_roles() {
    assert_eq!(Theme::role_for_risk(RiskLevel::Low), StyleRole::Recovery);
    assert_eq!(Theme::role_for_risk(RiskLevel::Medium), StyleRole::Warning);
    assert_eq!(Theme::role_for_risk(RiskLevel::High), StyleRole::Critical);
}

#[test]
fn theme_prefers_delta_role_for_ui_events() {
    let event = UiEvent::control(1, "risk", "changed");
    assert_eq!(Theme::role_for_event(&event), StyleRole::Text);

    let delta_event = UiEvent::from_resolution(
        1,
        &waves::core::Resolution {
            success: true,
            summary: "ok".to_string(),
            changes: vec![waves::core::StateChange {
                target: "water".to_string(),
                before: 1.0,
                after: 1.5,
                delta: 0.5,
                reason: "test".to_string(),
            }],
        },
    )
    .remove(0);
    assert_eq!(Theme::role_for_event(&delta_event), StyleRole::Positive);
}
