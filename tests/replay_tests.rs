use tempfile::tempdir;
use waves::app::{build_external_session, replay_run, run_headless, run_scripted_ticks};

#[test]
fn sqlite_persists_events_decisions_logs_and_snapshot() {
    let dir = tempdir().expect("tempdir");
    let db = dir.path().join("waves.sqlite");
    let mut session = build_external_session("sea_survival", "zh-CN", 123, Some(db))
        .expect("session should build");
    run_scripted_ticks(&mut session, 12).expect("run should complete");

    let counts = session
        .store_counts()
        .expect("counts")
        .expect("store exists");
    assert!(counts.0 >= 18, "domain event count: {}", counts.0);
    assert!(counts.1 >= 3, "decision count: {}", counts.1);
    assert!(counts.2 >= 6, "log count: {}", counts.2);
    let ui_count = session
        .store_ui_event_count()
        .expect("ui count")
        .expect("store exists");
    assert!(ui_count >= 12, "ui event count: {ui_count}");

    let snapshot = session
        .latest_persisted_snapshot()
        .expect("snapshot query")
        .expect("snapshot exists");
    assert_eq!(snapshot.tick, 12);
    assert_close(snapshot.stats.hp, session.state.stats.hp);
    assert_close(snapshot.stats.thirst, session.state.stats.thirst);
    assert_close(snapshot.resources.water, session.state.resources.water);
    assert_eq!(snapshot.environment.day, session.state.environment.day);
    assert_eq!(snapshot.outcome, session.state.outcome);
}

#[test]
fn replay_summary_reads_saved_run_without_agent_calls() {
    let dir = tempdir().expect("tempdir");
    let db = dir.path().join("waves.sqlite");
    let report =
        run_headless("sea_survival", "zh-CN", 8, 321, Some(db.clone())).expect("headless run");
    let run_id = report.run_id.clone();

    let summary = replay_run(db, &run_id).expect("replay summary");

    assert_eq!(summary.run.id, run_id);
    assert_eq!(summary.run.scenario_id, "sea_survival");
    assert_eq!(
        summary.latest_snapshot.as_ref().map(|state| state.tick),
        Some(8)
    );
    assert!(summary.counts.domain_events >= 12);
    assert!(summary.counts.decisions >= 2);
    assert!(
        summary
            .lines()
            .iter()
            .any(|line| line.contains("decisions="))
    );
}

fn assert_close(left: f64, right: f64) {
    assert!(
        (left - right).abs() < 0.000_001,
        "left={left}, right={right}"
    );
}
