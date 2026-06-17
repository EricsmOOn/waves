pub mod text_width;
pub mod theme;
pub mod ui_events;

mod renderer;
mod terminal;

pub use renderer::{AppView, render_app};
pub use terminal::{
    run_tui, run_tui_remote, run_tui_remote_with_hint, run_tui_remote_with_hint_and_scenarios_dir,
    run_tui_remote_with_scenarios_dir,
};
pub use theme::{StyleRole, Theme};
pub use ui_events::{UiEvent, UiEventKind, UiMotion, UiPriority, UiVisibility};
