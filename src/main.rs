use std::time::Instant;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crate::app::{App, CurrentScreen, ModelEntry};
use crate::ui::ui;

mod app;
mod render3d;
mod ui;

fn main() {
    render3d::ensure_default_models();

    let mesh = match std::env::args().nth(1) {
        Some(path) => match render3d::load_obj(&path) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("Failed to load OBJ '{path}': {e}. Falling back to procedural plushie.");
                render3d::plushie()
            }
        },
        None => render3d::plushie(),
    };

    let mut terminal = ratatui::init();
    let mut app = App::new(mesh);

    run_app(&mut terminal, &mut app);
    ratatui::restore();
}

fn run_app(terminal: &mut DefaultTerminal, app: &mut App) {
    let tick = sysinfo::MINIMUM_CPU_UPDATE_INTERVAL;
    loop {
        let frame_start = Instant::now();
        terminal.draw(|f| ui(f, app)).unwrap();

        loop {
            let elapsed = frame_start.elapsed();
            if elapsed >= tick {
                break;
            }
            let remaining = tick - elapsed;
            match event::poll(remaining) {
                Ok(true) => {
                    if let Ok(Event::Key(key)) = event::read()
                        && key.kind == KeyEventKind::Press
                    {
                        handle_key(app, key.code);
                    }
                }
                _ => break,
            }
        }

        if app.exit {
            break;
        }
    }
}

fn handle_key(app: &mut App, code: KeyCode) {
    if matches!(code, KeyCode::Char('q') | KeyCode::Char('Q')) {
        app.exit = true;
        return;
    }
    if app.picker_open {
        handle_picker_key(app, code);
        return;
    }
    match code {
        KeyCode::Esc => app.current_screen = CurrentScreen::Main,
        KeyCode::Char(ch) => match ch {
            'c' | 'C' | 'с' | 'С' => app.current_screen = CurrentScreen::CPUScreen,
            'm' | 'M' | 'ь' | 'Ь' | 'м' | 'М' => app.current_screen = CurrentScreen::MemoryScreen,
            'g' | 'G' | 'п' | 'П' | 'г' | 'Г' => app.current_screen = CurrentScreen::GPUScreen,
            'n' | 'N' | 'т' | 'Т' | 'и' | 'И' => app.current_screen = CurrentScreen::NetworkScreen,
            'd' | 'D' | 'в' | 'В' | 'д' | 'Д' => app.current_screen = CurrentScreen::DiskScreen,
            't' | 'T' | 'е' | 'Е' | 'з' | 'З' => app.current_screen = CurrentScreen::TaskListScreen,
            'o' | 'O' | 'щ' | 'Щ' | 'о' | 'О' => open_picker(app),
            _ => {}
        },
        _ => {}
    }
}

fn open_picker(app: &mut App) {
    let mut entries: Vec<ModelEntry> = vec![ModelEntry::Plushie];
    for p in render3d::list_model_files() {
        entries.push(ModelEntry::File(p));
    }
    app.picker_models = entries;
    app.picker_state.select(Some(0));
    app.picker_open = true;
}

fn handle_picker_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc => app.picker_open = false,
        KeyCode::Up | KeyCode::Char('k') => {
            let cur = app.picker_state.selected().unwrap_or(0);
            let new = cur.saturating_sub(1);
            app.picker_state.select(Some(new));
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let cur = app.picker_state.selected().unwrap_or(0);
            let max = app.picker_models.len().saturating_sub(1);
            let new = (cur + 1).min(max);
            app.picker_state.select(Some(new));
        }
        KeyCode::Enter => {
            if let Some(idx) = app.picker_state.selected()
                && let Some(entry) = app.picker_models.get(idx)
            {
                match entry {
                    ModelEntry::Plushie => app.mesh = render3d::plushie(),
                    ModelEntry::File(p) => {
                        if let Ok(m) = render3d::load_obj(&p.to_string_lossy()) {
                            app.mesh = m;
                        }
                    }
                }
            }
            app.picker_open = false;
        }
        _ => {}
    }
}
