use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders};
use crate::app::App;

pub fn ui(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Length(5),
            Constraint::Length(5),
            Constraint::Length(5),
            Constraint::Length(5),
        ]).split(frame.area());

    let cpu = Block::default().borders(Borders::ALL).title("CPU").style(Style::default());

    frame.render_widget(cpu.clone(), chunks[0]);
    frame.render_widget(cpu.clone(),chunks[1]);
    frame.render_widget(cpu.clone(), chunks[2]);
    frame.render_widget(cpu.clone(), chunks[3]);
    frame.render_widget(cpu.clone(), chunks[4]);

}