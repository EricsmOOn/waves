pub mod text_width;
pub mod theme;
pub mod ui_events;

mod renderer;
mod terminal;

pub use renderer::{AppView, render_app};
pub use terminal::{run_tui, run_tui_remote};
pub use theme::{StyleRole, Theme};
pub use ui_events::{UiEvent, UiEventKind, UiMotion, UiPriority, UiVisibility};
