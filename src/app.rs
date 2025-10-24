use crossterm::event;
use crossterm::event::{Event, KeyEventKind};
use crossterm::event::KeyCode;

pub enum CurrentScreen {
    Main,
    CPUScreen,
    GPUScreen,
    MemoryScreen,
    NetworkScreen,
    TaskListScreen,
}

pub struct App {
    pub exit: bool,
    pub current_screen: CurrentScreen,
}

impl App {
    pub fn new() -> App {
        App {
            current_screen: CurrentScreen::Main,
            exit: false,
        }
    }
}
