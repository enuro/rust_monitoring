use crate::app::{
    App, CurrentScreen, format_bytes, format_bytes_per_sec, format_duration, push_history,
    to_sparkline_data,
};
use crate::render3d::project;
use nvml_wrapper::enum_wrappers::device::TemperatureSensor;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::prelude::Modifier;
use ratatui::style::{Color, Style};
use ratatui::symbols::Marker;
use ratatui::text::{Line as TextLine, Span, Text};
use ratatui::widgets::canvas::{Canvas, Line};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Sparkline, Table, Wrap};
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate};

pub fn ui(frame: &mut Frame, app: &mut App) {
    collect_metrics(app);
    match app.current_screen {
        CurrentScreen::CPUScreen => ui_cpu(frame, app),
        CurrentScreen::MemoryScreen => ui_memory(frame, app),
        CurrentScreen::GPUScreen => ui_gpu(frame, app),
        CurrentScreen::NetworkScreen => ui_network(frame, app),
        CurrentScreen::DiskScreen => ui_disk(frame, app),
        CurrentScreen::TaskListScreen => ui_process(frame, app),
        CurrentScreen::Main => ui_main(frame, app),
    }
}

fn collect_metrics(app: &mut App) {
    // CPU (persistent System lives in App so the delta between refreshes is correct)
    app.system_cpu.refresh_cpu_usage();
    let global = app.system_cpu.global_cpu_usage();
    push_history(&mut app.cpu_history, global as f64);
    for (i, cpu) in app.system_cpu.cpus().iter().enumerate() {
        if i < app.cpu_per_core_history.len() {
            push_history(&mut app.cpu_per_core_history[i], cpu.cpu_usage() as f64);
        }
    }

    app.system_memory.refresh_memory();
    let used_memory = app.system_memory.used_memory() as f64;
    let total_memory = app.system_memory.total_memory() as f64;
    if total_memory > 0.0 {
        push_history(&mut app.ram_history, used_memory / total_memory * 100.0);
    }

    app.disks.refresh(false);
    let mut total_usage: f64 = 0.0;
    for disk in app.disks.list() {
        let total = disk.total_space() as f64;
        let avail = disk.available_space() as f64;
        if total > 0.0 {
            total_usage += (1.0 - avail / total) * 100.0;
        }
    }
    let dc = app.disks.list().len();
    let avg = if dc > 0 { total_usage / dc as f64 } else { 0.0 };
    push_history(&mut app.disk_history, avg);

    if let Some(nvml) = &app.nvml
        && let Ok(dev) = nvml.device_by_index(0)
        && let Ok(mem) = dev.memory_info()
    {
        push_history(
            &mut app.gpu_memory_history,
            mem.used as f64 / mem.total as f64 * 100.0,
        );
    }

    app.networks.refresh(false);
    let mut rx_delta: u64 = 0;
    let mut tx_delta: u64 = 0;
    for (_iface, data) in &app.networks {
        rx_delta = rx_delta.saturating_add(data.received());
        tx_delta = tx_delta.saturating_add(data.transmitted());
    }
    let tick_secs = sysinfo::MINIMUM_CPU_UPDATE_INTERVAL.as_secs_f64();
    push_history(&mut app.net_download_history, rx_delta as f64 / tick_secs);
    push_history(&mut app.net_upload_history, tx_delta as f64 / tick_secs);

    app.processes.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::everything(),
    );

    app.frame_count = app.frame_count.wrapping_add(1);
}

fn load_color(percent: f64) -> Color {
    if percent >= 80.0 {
        Color::Red
    } else if percent >= 50.0 {
        Color::Yellow
    } else {
        Color::Green
    }
}

fn ui_main(frame: &mut Frame, app: &App) {
    let show_charts = frame.area().width >= 100;
    let row_constraints: [Constraint; 2] = if show_charts {
        [Constraint::Percentage(40), Constraint::Percentage(60)]
    } else {
        [Constraint::Percentage(100), Constraint::Percentage(0)]
    };

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(frame.area());

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(outer[0]);

    let chunks_data = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
        ])
        .split(chunks[1]);

    let cpu_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(row_constraints)
        .split(chunks_data[0]);
    let gpu_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(row_constraints)
        .split(chunks_data[1]);
    let memory_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(row_constraints)
        .split(chunks_data[2]);
    let net_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(row_constraints)
        .split(chunks_data[3]);
    let disk_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(row_constraints)
        .split(chunks_data[4]);

    let null = Block::default()
        .borders(Borders::BOTTOM)
        .title("")
        .style(Style::default());

    // MEMORY
    let memory_inf = app.ram_history.back().copied().unwrap_or(0.0);
    let memory = Paragraph::new(Text::styled(
        format!("{}\n{:.1}%", "▄▄▄▄▄", memory_inf),
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
    ))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("MEMORY")
            .style(Style::default().red()),
    )
    .alignment(Alignment::Center)
    .wrap(Wrap { trim: true });
    frame.render_widget(memory, memory_chunks[0]);

    // DISK
    let used_disk = app.disk_history.back().copied().unwrap_or(0.0);
    let disk = Paragraph::new(Text::styled(
        format!("{}\n{:.1}%", "▄▄▄▄▄", used_disk),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    ))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("DISK")
            .style(Style::default().cyan()),
    )
    .alignment(Alignment::Center)
    .wrap(Wrap { trim: true });
    frame.render_widget(disk, disk_chunks[0]);

    // CPU
    let cpu_inf = app.cpu_history.back().copied().unwrap_or(0.0);
    let cpu = Paragraph::new(Text::styled(
        format!("{}\n{:.1}%", "▄▄▄▄▄", cpu_inf),
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
    ))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("CPU")
            .style(Style::default().green()),
    )
    .alignment(Alignment::Center)
    .wrap(Wrap { trim: true });
    frame.render_widget(cpu, cpu_chunks[0]);

    // GPU
    let gpu_text = match app.gpu_memory_history.back().copied() {
        Some(p) => format!("{}\n{:.1}%", "▄▄▄▄▄", p),
        None => format!("{}\nN/A", "▄▄▄▄▄"),
    };
    let gpu_paragraph = Paragraph::new(Text::styled(
        gpu_text,
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    ))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("GPU")
            .style(Style::default().yellow()),
    )
    .alignment(Alignment::Center)
    .wrap(Wrap { trim: true });
    frame.render_widget(gpu_paragraph, gpu_chunks[0]);

    // NET
    let rx_bps = app.net_download_history.back().copied().unwrap_or(0.0);
    let tx_bps = app.net_upload_history.back().copied().unwrap_or(0.0);
    let net_paragraph = Paragraph::new(Text::styled(
        format!(
            "{}\n↓ {}\n↑ {}",
            "▄▄▄▄▄",
            format_bytes_per_sec(rx_bps),
            format_bytes_per_sec(tx_bps),
        ),
        Style::default()
            .fg(Color::Magenta)
            .add_modifier(Modifier::BOLD),
    ))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("NET")
            .style(Style::default().magenta()),
    )
    .alignment(Alignment::Center)
    .wrap(Wrap { trim: true });
    frame.render_widget(net_paragraph, net_chunks[0]);

    // 3D MODEL VIEWPORT
    let angle_y = (app.frame_count as f64) * 0.04;
    let tilt_x = 0.25;
    let projected: Vec<(f64, f64)> = app
        .mesh
        .vertices
        .iter()
        .map(|v| project(v.rotate_y(angle_y).rotate_x(tilt_x), 4.0))
        .collect();
    let edges_ref = &app.mesh.edges;
    let projected_ref = &projected;
    let canvas = Canvas::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("MODEL")
                .style(Style::default().fg(Color::DarkGray)),
        )
        .marker(Marker::Braille)
        .x_bounds([-1.7, 1.7])
        .y_bounds([-1.7, 1.7])
        .paint(move |ctx| {
            for (a, b) in edges_ref {
                let (x1, y1) = projected_ref[*a];
                let (x2, y2) = projected_ref[*b];
                ctx.draw(&Line {
                    x1,
                    y1,
                    x2,
                    y2,
                    color: Color::LightCyan,
                });
            }
        });
    frame.render_widget(canvas, chunks[0]);

    if show_charts {
        let cpu_sparkline = Sparkline::default()
            .block(Block::default().borders(Borders::ALL))
            .data(to_sparkline_data(&app.cpu_history, 100.0))
            .max(10000)
            .style(Style::default().fg(Color::Green));
        frame.render_widget(cpu_sparkline, cpu_chunks[1]);

        let gpu_sparkline = Sparkline::default()
            .block(Block::default().borders(Borders::ALL))
            .data(to_sparkline_data(&app.gpu_memory_history, 100.0))
            .max(10000)
            .style(Style::default().fg(Color::Yellow));
        frame.render_widget(gpu_sparkline, gpu_chunks[1]);

        let memory_sparkline = Sparkline::default()
            .block(Block::default().borders(Borders::ALL))
            .data(to_sparkline_data(&app.ram_history, 100.0))
            .max(10000)
            .style(Style::default().fg(Color::Red));
        frame.render_widget(memory_sparkline, memory_chunks[1]);

        let disk_sparkline = Sparkline::default()
            .block(Block::default().borders(Borders::ALL))
            .data(to_sparkline_data(&app.disk_history, 100.0))
            .max(10000)
            .style(Style::default().fg(Color::Cyan));
        frame.render_widget(disk_sparkline, disk_chunks[1]);

        let net_inner = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(net_chunks[1]);

        let net_dl_max = app
            .net_download_history
            .iter()
            .copied()
            .fold(1.0_f64, f64::max);
        let net_ul_max = app
            .net_upload_history
            .iter()
            .copied()
            .fold(1.0_f64, f64::max);

        let dl_sparkline = Sparkline::default()
            .block(Block::default().borders(Borders::BOTTOM).title("DL"))
            .data(to_sparkline_data(&app.net_download_history, 1.0))
            .max(net_dl_max as u64)
            .style(Style::default().fg(Color::Magenta));
        frame.render_widget(dl_sparkline, net_inner[0]);

        let ul_sparkline = Sparkline::default()
            .block(Block::default().title("UL"))
            .data(to_sparkline_data(&app.net_upload_history, 1.0))
            .max(net_ul_max as u64)
            .style(Style::default().fg(Color::LightMagenta));
        frame.render_widget(ul_sparkline, net_inner[1]);
    } else {
        frame.render_widget(null.clone(), cpu_chunks[1]);
        frame.render_widget(null.clone(), gpu_chunks[1]);
        frame.render_widget(null.clone(), memory_chunks[1]);
        frame.render_widget(null.clone(), net_chunks[1]);
        frame.render_widget(null.clone(), disk_chunks[1]);
    }

    // TOP PROCESS
    let top_proc = app.processes.processes().values().max_by(|a, b| {
        a.cpu_usage()
            .partial_cmp(&b.cpu_usage())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let process_text = match top_proc {
        Some(p) => format!(
            "{}\n{}\nCPU {:.1}%   MEM {} MB",
            "▄▄▄▄▄",
            p.name().to_string_lossy(),
            p.cpu_usage(),
            p.memory() / 1024 / 1024,
        ),
        None => format!("{}\nN/A", "▄▄▄▄▄"),
    };
    let process_paragraph = Paragraph::new(Text::styled(
        process_text,
        Style::default()
            .fg(Color::Blue)
            .add_modifier(Modifier::BOLD),
    ))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("PROCESS")
            .style(Style::default().blue()),
    )
    .alignment(Alignment::Center)
    .wrap(Wrap { trim: true });
    frame.render_widget(process_paragraph, chunks_data[5]);

    // Status bar
    let hint = Paragraph::new(" [C] CPU  [M] Memory  [G] GPU  [N] Net  [D] Disk  [T] Tasks  [Q] Quit ")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    frame.render_widget(hint, outer[1]);
}

fn ui_cpu(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(7),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    frame.render_widget(detail_header("CPU details", Color::Green), chunks[0]);

    // Overall block: text + history sparkline
    let cpu_now = app.cpu_history.back().copied().unwrap_or(0.0);
    let cpu_color = load_color(cpu_now);
    let overall_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(chunks[1]);

    let cores = app.system_cpu.cpus().len();
    let brand = app
        .system_cpu
        .cpus()
        .first()
        .map(|c| c.brand().to_string())
        .unwrap_or_default();
    let overall_text = Paragraph::new(Text::styled(
        format!("{:.1}%\n{} cores\n{}", cpu_now, cores, brand),
        Style::default().fg(cpu_color).add_modifier(Modifier::BOLD),
    ))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Overall CPU")
            .style(Style::default().fg(cpu_color)),
    )
    .alignment(Alignment::Center)
    .wrap(Wrap { trim: true });
    frame.render_widget(overall_text, overall_layout[0]);

    let overall_sparkline = Sparkline::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("History")
                .style(Style::default().fg(cpu_color)),
        )
        .data(to_sparkline_data(&app.cpu_history, 100.0))
        .max(10000)
        .style(Style::default().fg(cpu_color));
    frame.render_widget(overall_sparkline, overall_layout[1]);

    // Per-core grid
    let core_count = app.cpu_per_core_history.len();
    if core_count > 0 {
        let cols = ((core_count as f64).sqrt().ceil() as usize).max(1);
        let rows = core_count.div_ceil(cols);

        let row_constraints: Vec<Constraint> =
            (0..rows).map(|_| Constraint::Ratio(1, rows as u32)).collect();
        let row_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(row_constraints)
            .split(chunks[2]);

        let col_constraints: Vec<Constraint> =
            (0..cols).map(|_| Constraint::Ratio(1, cols as u32)).collect();

        for r in 0..rows {
            let cell_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(col_constraints.clone())
                .split(row_chunks[r]);

            for c in 0..cols {
                let idx = r * cols + c;
                if idx >= core_count {
                    break;
                }
                let history = &app.cpu_per_core_history[idx];
                let current = history.back().copied().unwrap_or(0.0);
                let color = load_color(current);
                let sparkline = Sparkline::default()
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(format!(" Core {} · {:>5.1}% ", idx, current))
                            .style(Style::default().fg(color)),
                    )
                    .data(to_sparkline_data(history, 100.0))
                    .max(10000)
                    .style(Style::default().fg(color));
                frame.render_widget(sparkline, cell_chunks[c]);
            }
        }
    }

    // Footer
    let footer = Paragraph::new(format!(" {} cores detected ", core_count))
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    frame.render_widget(footer, chunks[3]);
}

fn detail_header(title: &str, accent: Color) -> Paragraph<'_> {
    Paragraph::new(format!(" {title} — [Esc] back   [Q] Quit "))
        .style(
            Style::default()
                .fg(Color::Black)
                .bg(accent)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
}

fn kv_line<'a>(label: &'a str, value: String, color: Color) -> TextLine<'a> {
    TextLine::from(vec![
        Span::styled(
            format!("{label:<14}"),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(
            value,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
    ])
}

fn ui_memory(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(8),
        ])
        .split(area);

    frame.render_widget(detail_header("Memory details", Color::Red), chunks[0]);

    let total = app.system_memory.total_memory();
    let used = app.system_memory.used_memory();
    let avail = app.system_memory.available_memory();
    let free = app.system_memory.free_memory();
    let swap_total = app.system_memory.total_swap();
    let swap_used = app.system_memory.used_swap();
    let swap_free = app.system_memory.free_swap();

    let percent = if total > 0 {
        used as f64 / total as f64 * 100.0
    } else {
        0.0
    };
    let swap_percent = if swap_total > 0 {
        swap_used as f64 / swap_total as f64 * 100.0
    } else {
        0.0
    };
    let color = load_color(percent);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(chunks[1]);

    let lines = vec![
        TextLine::from(Span::styled(
            format!("{percent:.1}%"),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )),
        TextLine::from(""),
        kv_line("Total", format_bytes(total), Color::White),
        kv_line("Used", format_bytes(used), color),
        kv_line("Available", format_bytes(avail), Color::Green),
        kv_line("Free", format_bytes(free), Color::Cyan),
        TextLine::from(""),
        TextLine::from(Span::styled(
            format!("Swap  {swap_percent:.1}%"),
            Style::default().fg(load_color(swap_percent)).add_modifier(Modifier::BOLD),
        )),
        kv_line("Swap total", format_bytes(swap_total), Color::White),
        kv_line("Swap used", format_bytes(swap_used), load_color(swap_percent)),
        kv_line("Swap free", format_bytes(swap_free), Color::Cyan),
    ];
    let text_panel = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("RAM")
                .style(Style::default().fg(Color::Red)),
        )
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false });
    frame.render_widget(text_panel, body[0]);

    let sparkline = Sparkline::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("RAM history")
                .style(Style::default().fg(color)),
        )
        .data(to_sparkline_data(&app.ram_history, 100.0))
        .max(10000)
        .style(Style::default().fg(color));
    frame.render_widget(sparkline, body[1]);

    let top_mem: Vec<_> = {
        let mut v: Vec<_> = app.processes.processes().values().collect();
        v.sort_by(|a, b| b.memory().cmp(&a.memory()));
        v.into_iter().take(6).collect()
    };
    let rows: Vec<Row> = top_mem
        .iter()
        .map(|p| {
            Row::new(vec![
                Cell::from(format!("{}", p.pid())),
                Cell::from(p.name().to_string_lossy().to_string()),
                Cell::from(format_bytes(p.memory())),
            ])
        })
        .collect();
    let table = Table::new(
        rows,
        [
            Constraint::Length(8),
            Constraint::Min(10),
            Constraint::Length(12),
        ],
    )
    .header(
        Row::new(vec!["PID", "Process", "Memory"])
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Top memory consumers")
            .style(Style::default().fg(Color::Red)),
    );
    frame.render_widget(table, chunks[2]);
}

fn ui_gpu(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);

    frame.render_widget(detail_header("GPU details", Color::Yellow), chunks[0]);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(chunks[1]);

    let lines: Vec<TextLine> = match app.nvml.as_ref().and_then(|n| n.device_by_index(0).ok()) {
        Some(dev) => {
            let name = dev.name().unwrap_or_else(|_| "Unknown GPU".to_string());
            let mem = dev.memory_info().ok();
            let util = dev.utilization_rates().ok();
            let temp = dev.temperature(TemperatureSensor::Gpu).ok();
            let power_mw = dev.power_usage().ok();
            let fan = dev.fan_speed(0).ok();

            let mut out = vec![
                TextLine::from(Span::styled(
                    name,
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )),
                TextLine::from(""),
            ];
            if let Some(m) = mem {
                let p = m.used as f64 / m.total as f64 * 100.0;
                out.push(TextLine::from(Span::styled(
                    format!("Memory   {p:.1}%"),
                    Style::default().fg(load_color(p)).add_modifier(Modifier::BOLD),
                )));
                out.push(kv_line("Mem used", format_bytes(m.used), load_color(p)));
                out.push(kv_line("Mem total", format_bytes(m.total), Color::White));
                out.push(kv_line("Mem free", format_bytes(m.free), Color::Cyan));
                out.push(TextLine::from(""));
            }
            if let Some(u) = util {
                out.push(kv_line(
                    "GPU util",
                    format!("{} %", u.gpu),
                    load_color(u.gpu as f64),
                ));
                out.push(kv_line(
                    "Mem util",
                    format!("{} %", u.memory),
                    load_color(u.memory as f64),
                ));
            }
            if let Some(t) = temp {
                let c = if t >= 80 {
                    Color::Red
                } else if t >= 65 {
                    Color::Yellow
                } else {
                    Color::Green
                };
                out.push(kv_line("Temperature", format!("{t} °C"), c));
            }
            if let Some(mw) = power_mw {
                out.push(kv_line("Power", format!("{:.1} W", mw as f64 / 1000.0), Color::Magenta));
            }
            if let Some(f) = fan {
                out.push(kv_line("Fan", format!("{f} %"), Color::Cyan));
            }
            out
        }
        None => vec![TextLine::from(Span::styled(
            "NVIDIA GPU not detected (NVML unavailable).",
            Style::default().fg(Color::DarkGray),
        ))],
    };

    let info = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("GPU info")
                .style(Style::default().fg(Color::Yellow)),
        )
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false });
    frame.render_widget(info, body[0]);

    let gpu_now = app.gpu_memory_history.back().copied().unwrap_or(0.0);
    let color = load_color(gpu_now);
    let sparkline = Sparkline::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("GPU memory history")
                .style(Style::default().fg(color)),
        )
        .data(to_sparkline_data(&app.gpu_memory_history, 100.0))
        .max(10000)
        .style(Style::default().fg(color));
    frame.render_widget(sparkline, body[1]);
}

fn ui_network(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(7),
            Constraint::Min(0),
        ])
        .split(area);

    frame.render_widget(detail_header("Network details", Color::Magenta), chunks[0]);

    let dl = app.net_download_history.back().copied().unwrap_or(0.0);
    let ul = app.net_upload_history.back().copied().unwrap_or(0.0);
    let dl_max = app.net_download_history.iter().copied().fold(1.0_f64, f64::max);
    let ul_max = app.net_upload_history.iter().copied().fold(1.0_f64, f64::max);

    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);
    let dl_panel = Sparkline::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" ↓ Download · {} ", format_bytes_per_sec(dl)))
                .style(Style::default().fg(Color::Magenta)),
        )
        .data(to_sparkline_data(&app.net_download_history, 1.0))
        .max(dl_max as u64)
        .style(Style::default().fg(Color::Magenta));
    frame.render_widget(dl_panel, top[0]);
    let ul_panel = Sparkline::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" ↑ Upload · {} ", format_bytes_per_sec(ul)))
                .style(Style::default().fg(Color::LightMagenta)),
        )
        .data(to_sparkline_data(&app.net_upload_history, 1.0))
        .max(ul_max as u64)
        .style(Style::default().fg(Color::LightMagenta));
    frame.render_widget(ul_panel, top[1]);

    let tick_secs = sysinfo::MINIMUM_CPU_UPDATE_INTERVAL.as_secs_f64();
    let mut ifaces: Vec<_> = app.networks.iter().collect();
    ifaces.sort_by(|a, b| b.1.total_received().cmp(&a.1.total_received()));
    let rows: Vec<Row> = ifaces
        .iter()
        .map(|(name, data)| {
            let rx_sec = data.received() as f64 / tick_secs;
            let tx_sec = data.transmitted() as f64 / tick_secs;
            Row::new(vec![
                Cell::from(name.to_string()),
                Cell::from(format_bytes_per_sec(rx_sec)).style(Style::default().fg(Color::Magenta)),
                Cell::from(format_bytes_per_sec(tx_sec)).style(Style::default().fg(Color::LightMagenta)),
                Cell::from(format_bytes(data.total_received())),
                Cell::from(format_bytes(data.total_transmitted())),
                Cell::from(data.mac_address().to_string()),
            ])
        })
        .collect();
    let table = Table::new(
        rows,
        [
            Constraint::Min(8),
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Length(20),
        ],
    )
    .header(
        Row::new(vec!["Interface", "↓ /s", "↑ /s", "Total ↓", "Total ↑", "MAC"])
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Interfaces")
            .style(Style::default().fg(Color::Magenta)),
    );
    frame.render_widget(table, chunks[2]);
}

fn ui_disk(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(5),
            Constraint::Min(0),
        ])
        .split(area);

    frame.render_widget(detail_header("Disk details", Color::Cyan), chunks[0]);

    let avg = app.disk_history.back().copied().unwrap_or(0.0);
    let color = load_color(avg);
    let summary_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(chunks[1]);
    let summary = Paragraph::new(Text::styled(
        format!("Avg {avg:.1}%\n{} disks", app.disks.list().len()),
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    ))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Overall")
            .style(Style::default().fg(color)),
    )
    .alignment(Alignment::Center);
    frame.render_widget(summary, summary_layout[0]);
    let sparkline = Sparkline::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Avg history")
                .style(Style::default().fg(color)),
        )
        .data(to_sparkline_data(&app.disk_history, 100.0))
        .max(10000)
        .style(Style::default().fg(color));
    frame.render_widget(sparkline, summary_layout[1]);

    let rows: Vec<Row> = app
        .disks
        .list()
        .iter()
        .map(|d| {
            let total = d.total_space();
            let avail = d.available_space();
            let used = total.saturating_sub(avail);
            let p = if total > 0 {
                used as f64 / total as f64 * 100.0
            } else {
                0.0
            };
            let pc = load_color(p);
            Row::new(vec![
                Cell::from(d.name().to_string_lossy().to_string()),
                Cell::from(d.mount_point().to_string_lossy().to_string()),
                Cell::from(d.file_system().to_string_lossy().to_string()),
                Cell::from(format!("{:?}", d.kind())),
                Cell::from(format_bytes(total)),
                Cell::from(format_bytes(used)).style(Style::default().fg(pc)),
                Cell::from(format_bytes(avail)).style(Style::default().fg(Color::Green)),
                Cell::from(format!("{p:.1} %")).style(Style::default().fg(pc)),
            ])
        })
        .collect();
    let table = Table::new(
        rows,
        [
            Constraint::Length(14),
            Constraint::Length(16),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Length(8),
        ],
    )
    .header(
        Row::new(vec![
            "Name", "Mount", "FS", "Kind", "Total", "Used", "Avail", "Use",
        ])
        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Disks")
            .style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(table, chunks[2]);
}

fn ui_process(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    frame.render_widget(detail_header("Processes", Color::Blue), chunks[0]);

    let core_count = app.system_cpu.cpus().len().max(1) as f32;
    let visible = (chunks[1].height.saturating_sub(2)) as usize;

    let mut procs: Vec<_> = app.processes.processes().values().collect();
    procs.sort_by(|a, b| {
        b.cpu_usage()
            .partial_cmp(&a.cpu_usage())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let rows: Vec<Row> = procs
        .iter()
        .take(visible)
        .map(|p| {
            let cpu = p.cpu_usage();
            let cpu_norm = cpu / core_count;
            let cpu_color = load_color(cpu_norm as f64);
            Row::new(vec![
                Cell::from(format!("{}", p.pid())),
                Cell::from(p.name().to_string_lossy().to_string()),
                Cell::from(format!("{cpu:>5.1} %")).style(Style::default().fg(cpu_color)),
                Cell::from(format_bytes(p.memory())),
                Cell::from(format_duration(p.run_time())),
            ])
        })
        .collect();
    let table = Table::new(
        rows,
        [
            Constraint::Length(8),
            Constraint::Min(20),
            Constraint::Length(10),
            Constraint::Length(12),
            Constraint::Length(14),
        ],
    )
    .header(
        Row::new(vec!["PID", "Process", "CPU", "Memory", "Run time"])
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" Top {} processes by CPU ", visible))
            .style(Style::default().fg(Color::Blue)),
    );
    frame.render_widget(table, chunks[1]);

    let footer = Paragraph::new(format!(
        " {} processes total · CPU normalized to {} cores ",
        app.processes.processes().len(),
        core_count as u32
    ))
    .style(Style::default().fg(Color::DarkGray))
    .alignment(Alignment::Center);
    frame.render_widget(footer, chunks[2]);
}
