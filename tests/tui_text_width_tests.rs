use waves::tui::text_width::{
    Align, display_width, pad_to_width, truncate_to_width, wrap_to_width,
};

#[test]
fn display_width_handles_chinese_and_mixed_text() {
    assert_eq!(display_width("淡水"), 4);
    assert_eq!(display_width("Raft 耐久 -8"), 12);
    assert_eq!(display_width("Water +0.4"), 10);
}

#[test]
fn truncate_and_pad_use_terminal_width() {
    assert_eq!(truncate_to_width("AI 判断海浪较大", 8), "AI 判断…");
    assert_eq!(display_width(&pad_to_width("淡水", 8, Align::Left)), 8);
    assert_eq!(display_width(&pad_to_width("HP", 8, Align::Right)), 8);
}

#[test]
fn wrap_supports_chinese_without_spaces() {
    let lines = wrap_to_width("AI判断海浪较大选择先观察", 8);
    assert!(lines.len() > 1);
    assert!(lines.iter().all(|line| display_width(line) <= 8));
}
