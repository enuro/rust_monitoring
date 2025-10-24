use color_eyre::owo_colors::OwoColorize;
use ratatui::{layout, Frame};
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::prelude::Modifier;
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::Text;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Wrap};
use sysinfo::{
    Components, Disks, Networks, System,
};
use sysinfo::{CpuRefreshKind, RefreshKind, MemoryRefreshKind};
use crate::app::App;

pub fn ui(frame: &mut Frame, app: &App) {
    let mut system_cpu = System::new_with_specifics(
        RefreshKind::nothing().with_cpu(CpuRefreshKind::everything()),
    );
    let mut system_memory = System::new_with_specifics(
        RefreshKind::nothing().with_memory(MemoryRefreshKind::everything()),
    );
    
    std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
    system_cpu.refresh_cpu_usage();
    system_memory.refresh_memory();

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50)
        ]).split(frame.area());

    let chunks_data = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
        ]).split(chunks[1]);

    let cpu_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(15),
            Constraint::Percentage(90),
        ])
        .split(chunks_data[0]);

    let gpu_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(15),
            Constraint::Percentage(90),
        ])
        .split(chunks_data[1]);
    let memory_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(15),
            Constraint::Percentage(90),
        ])
        .split(chunks_data[2]);
    let net_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(15),
            Constraint::Percentage(90),
        ])
        .split(chunks_data[3]);

    let null = Block::default().borders(Borders::BOTTOM).title("").style(Style::default());

    let gpu = Block::default().borders(Borders::ALL).title("GPU").style(Style::default().yellow());

    let memory_data = Block::default().borders(Borders::ALL).title("MEMORY").style(Style::default().red());
    let used_memory = system_memory.used_memory() as f64;
    let total_memory = system_memory.total_memory() as f64;
    let memory_inf = (used_memory / total_memory) * 100.0;
    let memory = Paragraph::new(Text::styled(
        format!("{}\n{:.1}%", "▄▄▄▄▄", memory_inf),
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
    )).block(memory_data).alignment(Alignment::Center).wrap(Wrap { trim: true });
    frame.render_widget(memory.clone(), memory_chunks[0]);
    frame.render_widget(null.clone(), memory_chunks[1]);

    let net = Block::default().borders(Borders::ALL).title("NET").style(Style::default().magenta());
    let tusk = Block::default().borders(Borders::ALL).title("TUSK").style(Style::default().blue());


    let cpu_data = Block::default().borders(Borders::ALL).title("CPU").style(Style::default().green());
    let cpu_inf = system_cpu.global_cpu_usage();
    let cpu = Paragraph::new(Text::styled(
        format!("{}\n{:.1}%", "▄▄▄▄▄", cpu_inf),
        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
    )).block(cpu_data).alignment(Alignment::Center).wrap(Wrap { trim: true });
    frame.render_widget(cpu.clone(), cpu_chunks[0]);
    frame.render_widget(null.clone(), cpu_chunks[1]);


    frame.render_widget(null.clone(), chunks[0]);
    frame.render_widget(gpu.clone(), gpu_chunks[0]);
    frame.render_widget(null.clone(), gpu_chunks[1]);

    frame.render_widget(net.clone(), net_chunks[0]);
    frame.render_widget(null.clone(), net_chunks[1]);
    frame.render_widget(tusk.clone(), chunks_data[4]);

}