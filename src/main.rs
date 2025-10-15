use ratatui::backend::Backend;
use ratatui::{DefaultTerminal, Terminal};

use crate::app::App;
use crate::ui::ui;

mod app;
mod ui;

fn main() {
    let mut terminal = ratatui::init();
    let mut app_result = App::new();

    let res = run_app(&mut terminal, &mut app_result);
}

fn run_app(terminal: &mut DefaultTerminal, app: &mut App) {
    loop {
        terminal.draw(|mut f| {ui(f, app)}).unwrap();
    }
}